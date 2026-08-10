use std::{
    collections::{BinaryHeap, VecDeque},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use async_task::Runnable;
use parking::{Parker, Unparker};
use parking_lot::{Condvar, Mutex};

use crate::{PlatformDispatcher, TaskLabel, TaskPriority};

const MIN_THREADS: usize = 2;
const MAX_THREADS: usize = 8;

/// A production-like multithreaded dispatcher for tests and benchmarks.
///
/// Background tasks execute on worker threads and timers use real wall-clock
/// time. Main-thread work remains queued until the thread that created the
/// dispatcher calls [`Self::run_until_idle`], [`Self::run_until`], or
/// [`Self::run_ready_main_tasks`].
///
/// Unlike [`crate::TestDispatcher`], this dispatcher does not use a virtual
/// clock and does not serialize background tasks.
#[doc(hidden)]
pub struct ThreadedDispatcher {
    background: Arc<BackgroundQueue>,
    main: Mutex<VecDeque<Runnable>>,
    timers: Arc<TimerQueue>,
    idle: Arc<IdleTracker>,
    main_thread_id: thread::ThreadId,
    parker: Mutex<Parker>,
    unparker: Unparker,
}

#[derive(Default)]
struct BackgroundQueue {
    state: Mutex<BackgroundQueueState>,
    condvar: Condvar,
}

#[derive(Default)]
struct BackgroundQueueState {
    high: VecDeque<Runnable>,
    normal: VecDeque<Runnable>,
    low: VecDeque<Runnable>,
}

impl BackgroundQueue {
    fn push(&self, priority: TaskPriority, runnable: Runnable) {
        let mut state = self.state.lock();
        match priority {
            TaskPriority::High => state.high.push_back(runnable),
            TaskPriority::Medium => state.normal.push_back(runnable),
            TaskPriority::Low => state.low.push_back(runnable),
        }
        self.condvar.notify_one();
    }

    fn pop(&self) -> Runnable {
        let mut state = self.state.lock();
        loop {
            if let Some(runnable) = state.high.pop_front() {
                return runnable;
            }
            if let Some(runnable) = state.normal.pop_front() {
                return runnable;
            }
            if let Some(runnable) = state.low.pop_front() {
                return runnable;
            }
            self.condvar.wait(&mut state);
        }
    }
}

#[derive(Default)]
struct IdleTracker {
    inflight: Mutex<usize>,
    condvar: Condvar,
}

impl IdleTracker {
    fn increment(&self) {
        *self.inflight.lock() += 1;
    }

    fn decrement(&self) {
        let mut inflight = self.inflight.lock();
        *inflight = inflight
            .checked_sub(1)
            .expect("threaded dispatcher in-flight count underflow");
        self.condvar.notify_all();
    }

    fn notify(&self) {
        self.condvar.notify_all();
    }
}

struct InflightGuard(Arc<IdleTracker>);

impl Drop for InflightGuard {
    fn drop(&mut self) {
        self.0.decrement();
    }
}

struct TimerQueue {
    state: Mutex<TimerQueueState>,
    condvar: Condvar,
}

#[derive(Default)]
struct TimerQueueState {
    heap: BinaryHeap<TimerEntry>,
    next_sequence: u64,
}

struct TimerEntry {
    due: Instant,
    sequence: u64,
    runnable: Runnable,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.due == other.due && self.sequence == other.sequence
    }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .due
            .cmp(&self.due)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl Default for ThreadedDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadedDispatcher {
    /// Creates a dispatcher whose main thread is the calling thread.
    pub fn new() -> Self {
        let background = Arc::new(BackgroundQueue::default());
        let idle = Arc::new(IdleTracker::default());
        let worker_count = thread::available_parallelism().map_or(MIN_THREADS, |count| {
            count.get().clamp(MIN_THREADS, MAX_THREADS)
        });

        for index in 0..worker_count {
            let background = background.clone();
            let idle = idle.clone();
            thread::Builder::new()
                .name(format!("ThreadedDispatcherWorker-{index}"))
                .spawn(move || {
                    loop {
                        let runnable = background.pop();
                        let _inflight = InflightGuard(idle.clone());
                        runnable.run();
                    }
                })
                .expect("failed to spawn threaded dispatcher worker");
        }

        let timers = Arc::new(TimerQueue {
            state: Mutex::new(TimerQueueState::default()),
            condvar: Condvar::new(),
        });
        {
            let timers = timers.clone();
            let background = background.clone();
            let idle = idle.clone();
            thread::Builder::new()
                .name("ThreadedDispatcherTimer".to_owned())
                .spawn(move || {
                    let mut state = timers.state.lock();
                    loop {
                        let Some(entry) = state.heap.peek() else {
                            timers.condvar.wait(&mut state);
                            continue;
                        };
                        if entry.due > Instant::now() {
                            let due = entry.due;
                            timers.condvar.wait_until(&mut state, due);
                            continue;
                        }
                        let entry = state.heap.pop().expect("timer heap was non-empty");
                        drop(state);

                        idle.increment();
                        background.push(TaskPriority::Medium, entry.runnable);
                        idle.notify();

                        state = timers.state.lock();
                    }
                })
                .expect("failed to spawn threaded dispatcher timer");
        }

        let (parker, unparker) = parking::pair();
        Self {
            background,
            main: Mutex::new(VecDeque::new()),
            timers,
            idle,
            main_thread_id: thread::current().id(),
            parker: Mutex::new(parker),
            unparker,
        }
    }

    /// Runs main-thread work and waits until currently runnable background work
    /// reaches quiescence. Timers whose deadlines are still in the future are
    /// not awaited.
    pub fn run_until_idle(&self) {
        self.assert_main_thread();
        loop {
            if self.run_one_main_task() {
                continue;
            }
            if self.has_due_timer() {
                let mut inflight = self.idle.inflight.lock();
                self.idle
                    .condvar
                    .wait_for(&mut inflight, Duration::from_millis(1));
                continue;
            }

            let mut inflight = self.idle.inflight.lock();
            if !self.main.lock().is_empty() {
                continue;
            }
            if *inflight == 0 {
                return;
            }
            self.idle.condvar.wait(&mut inflight);
        }
    }

    /// Runs dispatcher work until `ready` returns true.
    ///
    /// Unlike [`Self::run_until_idle`], this method waits for future timers and
    /// external wakeups because they may be required to satisfy the condition.
    pub fn run_until(&self, mut ready: impl FnMut() -> bool) {
        self.assert_main_thread();
        while !ready() {
            if self.run_one_main_task() {
                continue;
            }

            let next_timer = self.next_timer_deadline();
            let mut inflight = self.idle.inflight.lock();
            if ready() || !self.main.lock().is_empty() {
                continue;
            }
            if let Some(deadline) = next_timer {
                self.idle.condvar.wait_until(&mut inflight, deadline);
            } else {
                self.idle.condvar.wait(&mut inflight);
            }
        }
    }

    /// Runs only the main-thread tasks that were queued when this method began.
    /// Tasks queued by those runnables remain for the next call.
    pub fn run_ready_main_tasks(&self) -> bool {
        self.assert_main_thread();
        let queued = self.main.lock().len();
        let mut ran_any = false;
        for _ in 0..queued {
            let runnable = self.main.lock().pop_front();
            let Some(runnable) = runnable else {
                break;
            };
            runnable.run();
            ran_any = true;
        }
        ran_any
    }

    /// Drops all pending real-time timers and returns how many were cancelled.
    pub fn cancel_pending_timers(&self) -> usize {
        let timers = {
            let mut state = self.timers.state.lock();
            let timers = state.heap.drain().collect::<Vec<_>>();
            self.timers.condvar.notify_all();
            timers
        };
        let count = timers.len();
        drop(timers);
        count
    }

    /// Returns a concise snapshot for diagnosing a dispatcher that does not
    /// reach quiescence.
    pub fn debug_state(&self) -> String {
        let inflight = *self.idle.inflight.lock();
        let main = self.main.lock().len();
        let timers = self.timers.state.lock().heap.len();
        format!("ThreadedDispatcher {{ inflight: {inflight}, main: {main}, timers: {timers} }}")
    }

    fn assert_main_thread(&self) {
        assert!(
            self.is_main_thread(),
            "threaded dispatcher main work must run on its creating thread"
        );
    }

    fn run_one_main_task(&self) -> bool {
        let runnable = self.main.lock().pop_front();
        let Some(runnable) = runnable else {
            return false;
        };
        runnable.run();
        true
    }

    fn has_due_timer(&self) -> bool {
        self.next_timer_deadline()
            .is_some_and(|deadline| deadline <= Instant::now())
    }

    fn next_timer_deadline(&self) -> Option<Instant> {
        self.timers.state.lock().heap.peek().map(|entry| entry.due)
    }
}

impl PlatformDispatcher for ThreadedDispatcher {
    fn is_main_thread(&self) -> bool {
        thread::current().id() == self.main_thread_id
    }

    fn dispatch(&self, runnable: Runnable, label: Option<TaskLabel>) {
        self.idle.increment();
        self.background
            .push(label.map(TaskLabel::priority).unwrap_or_default(), runnable);
        self.idle.notify();
        self.unparker.unpark();
    }

    fn dispatch_on_main_thread(&self, runnable: Runnable) {
        self.main.lock().push_back(runnable);
        self.idle.notify();
        self.unparker.unpark();
    }

    fn dispatch_after(&self, duration: Duration, runnable: Runnable) {
        let mut state = self.timers.state.lock();
        let entry = TimerEntry {
            due: Instant::now() + duration,
            sequence: state.next_sequence,
            runnable,
        };
        state.next_sequence = state.next_sequence.wrapping_add(1);
        state.heap.push(entry);
        self.timers.condvar.notify_all();
        drop(state);
        self.idle.notify();
        self.unparker.unpark();
    }

    fn park(&self, timeout: Option<Duration>) -> bool {
        if let Some(timeout) = timeout {
            self.parker.lock().park_timeout(timeout);
        } else {
            self.parker.lock().park();
        }
        true
    }

    fn unparker(&self) -> Unparker {
        self.unparker.clone()
    }

    fn as_threaded(&self) -> Option<&ThreadedDispatcher> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use futures::channel::oneshot;

    use super::*;
    use crate::{BackgroundExecutor, ForegroundExecutor};

    #[test]
    fn background_work_can_dispatch_to_main_thread() {
        let dispatcher = Arc::new(ThreadedDispatcher::new());
        let background = BackgroundExecutor::new(dispatcher.clone());
        let done = Arc::new(AtomicBool::new(false));
        let done_for_task = done.clone();
        let dispatcher_for_task = dispatcher.clone();

        background
            .spawn(async move {
                let (runnable, task) = async_task::spawn(
                    async move {
                        done_for_task.store(true, Ordering::SeqCst);
                    },
                    |_| {},
                );
                task.detach();
                dispatcher_for_task.dispatch_on_main_thread(runnable);
            })
            .detach();

        dispatcher.run_until(|| done.load(Ordering::SeqCst));
        assert!(dispatcher.is_main_thread());
    }

    #[test]
    fn timer_and_external_wake_use_real_threads() {
        let dispatcher = Arc::new(ThreadedDispatcher::new());
        let background = BackgroundExecutor::new(dispatcher.clone());
        let timer_done = Arc::new(AtomicBool::new(false));
        let timer_done_for_task = timer_done.clone();
        let background_for_timer = background.clone();
        background
            .spawn(async move {
                background_for_timer.timer(Duration::from_millis(5)).await;
                timer_done_for_task.store(true, Ordering::SeqCst);
            })
            .detach();
        dispatcher.run_until(|| timer_done.load(Ordering::SeqCst));

        let (sender, receiver) = oneshot::channel();
        let wake_done = Arc::new(AtomicBool::new(false));
        let wake_done_for_task = wake_done.clone();
        background
            .spawn(async move {
                receiver.await.unwrap();
                wake_done_for_task.store(true, Ordering::SeqCst);
            })
            .detach();
        thread::spawn(move || sender.send(()).unwrap());
        dispatcher.run_until(|| wake_done.load(Ordering::SeqCst));
    }

    #[test]
    fn ready_main_tasks_are_bounded_to_the_starting_batch() {
        let dispatcher = Arc::new(ThreadedDispatcher::new());
        let foreground = ForegroundExecutor::new(dispatcher.clone());
        let polls = Arc::new(AtomicUsize::new(0));
        let polls_for_task = polls.clone();
        foreground
            .spawn(std::future::poll_fn(move |cx| {
                let poll = polls_for_task.fetch_add(1, Ordering::SeqCst) + 1;
                if poll == 3 {
                    std::task::Poll::Ready(())
                } else {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
            }))
            .detach();

        assert!(dispatcher.run_ready_main_tasks());
        assert_eq!(polls.load(Ordering::SeqCst), 1);
        assert!(dispatcher.run_ready_main_tasks());
        assert_eq!(polls.load(Ordering::SeqCst), 2);
        assert!(dispatcher.run_ready_main_tasks());
        assert_eq!(polls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn idle_does_not_wait_for_future_timers() {
        let dispatcher = Arc::new(ThreadedDispatcher::new());
        let background = BackgroundExecutor::new(dispatcher.clone());
        let background_for_timer = background.clone();
        background
            .spawn(async move {
                background_for_timer.timer(Duration::from_secs(60)).await;
            })
            .detach();

        let start = Instant::now();
        dispatcher.run_until_idle();
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(dispatcher.cancel_pending_timers(), 1);
        dispatcher.run_until_idle();
    }
}
