use crate::SharedString;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    cell::LazyCell,
    collections::{HashMap, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
    panic::Location,
    sync::{
        Arc, LazyLock, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::ThreadId,
    time::Instant,
};

pub type ProfilingInstant = Instant;

const MAX_TASK_TIMINGS: usize = (16 * 1024 * 1024) / core::mem::size_of::<TaskTiming>();

static PROFILER_ENABLED: AtomicBool = AtomicBool::new(false);
static PROFILER_GENERATION: AtomicU64 = AtomicU64::new(0);
static GLOBAL_THREAD_TIMINGS: LazyLock<Mutex<Vec<GlobalThreadTimings>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

thread_local! {
    static THREAD_TIMINGS: LazyCell<Arc<Mutex<ThreadTimingState>>> = LazyCell::new(|| {
        let current_thread = std::thread::current();
        let thread_id = current_thread.id();
        let timings = Arc::new(Mutex::new(ThreadTimingState::new(
            current_thread.name().map(str::to_string),
            thread_id,
        )));

        GLOBAL_THREAD_TIMINGS.lock().push(GlobalThreadTimings {
            timings: Arc::downgrade(&timings),
        });

        timings
    });
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct TaskTiming {
    pub location: &'static Location<'static>,
    pub start: ProfilingInstant,
    pub end: ProfilingInstant,
    pub status: TaskTimingStatus,
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TaskTimingStatus {
    Completed,
    Cancelled,
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ThreadTaskTimings {
    pub thread_name: Option<String>,
    pub thread_id: ThreadId,
    pub timings: Vec<TaskTiming>,
    pub total_pushed: u64,
}

#[doc(hidden)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SerializedLocation {
    pub file: SharedString,
    pub line: u32,
    pub column: u32,
}

impl From<&'static Location<'static>> for SerializedLocation {
    fn from(value: &'static Location<'static>) -> Self {
        Self {
            file: value.file().into(),
            line: value.line(),
            column: value.column(),
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SerializedTaskTiming {
    pub location: SerializedLocation,
    pub start: u128,
    pub duration: u128,
    pub status: TaskTimingStatus,
}

impl SerializedTaskTiming {
    fn convert(anchor: ProfilingInstant, timings: &[TaskTiming]) -> Vec<Self> {
        timings
            .iter()
            .map(|timing| Self {
                location: timing.location.into(),
                start: timing
                    .start
                    .checked_duration_since(anchor)
                    .unwrap_or_default()
                    .as_nanos(),
                duration: timing.end.duration_since(timing.start).as_nanos(),
                status: timing.status,
            })
            .collect()
    }
}

#[doc(hidden)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SerializedThreadTaskTimings {
    pub thread_name: Option<String>,
    pub thread_id: u64,
    pub timings: Vec<SerializedTaskTiming>,
}

impl SerializedThreadTaskTimings {
    pub fn convert(anchor: ProfilingInstant, timings: ThreadTaskTimings) -> Self {
        Self {
            thread_name: timings.thread_name,
            thread_id: hash_thread_id(timings.thread_id),
            timings: SerializedTaskTiming::convert(anchor, &timings.timings),
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ThreadTimingsDelta {
    pub thread_id: u64,
    pub thread_name: Option<String>,
    pub new_timings: Vec<SerializedTaskTiming>,
}

#[doc(hidden)]
pub struct ProfilingCollector {
    startup_time: ProfilingInstant,
    cursors: HashMap<ThreadId, u64>,
}

impl ProfilingCollector {
    pub fn new(startup_time: ProfilingInstant) -> Self {
        Self {
            startup_time,
            cursors: HashMap::default(),
        }
    }

    pub fn startup_time(&self) -> ProfilingInstant {
        self.startup_time
    }

    pub fn collect_unseen(
        &mut self,
        all_timings: Vec<ThreadTaskTimings>,
    ) -> Vec<ThreadTimingsDelta> {
        let mut deltas = Vec::with_capacity(all_timings.len());

        for thread in all_timings {
            let previous_cursor = self.cursors.get(&thread.thread_id).copied().unwrap_or(0);
            let buffer_len = thread.timings.len() as u64;
            let buffer_start = thread.total_pushed.saturating_sub(buffer_len);
            let skip = previous_cursor.saturating_sub(buffer_start) as usize;
            let unseen = &thread.timings[skip.min(thread.timings.len())..];

            self.cursors.insert(thread.thread_id, thread.total_pushed);

            if unseen.is_empty() {
                continue;
            }

            deltas.push(ThreadTimingsDelta {
                thread_id: hash_thread_id(thread.thread_id),
                thread_name: thread.thread_name,
                new_timings: SerializedTaskTiming::convert(self.startup_time, unseen),
            });
        }

        deltas
    }

    pub fn reset(&mut self) {
        self.cursors.clear();
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct TaskTimingHandle {
    location: &'static Location<'static>,
    start: ProfilingInstant,
    generation: u64,
}

struct GlobalThreadTimings {
    timings: Weak<Mutex<ThreadTimingState>>,
}

struct ThreadTimingState {
    thread_name: Option<String>,
    thread_id: ThreadId,
    timings: VecDeque<TaskTiming>,
    total_pushed: u64,
}

impl ThreadTimingState {
    fn new(thread_name: Option<String>, thread_id: ThreadId) -> Self {
        Self {
            thread_name,
            thread_id,
            timings: VecDeque::new(),
            total_pushed: 0,
        }
    }

    fn push(&mut self, timing: TaskTiming) {
        while self.timings.len() + 1 > MAX_TASK_TIMINGS {
            self.timings.pop_front();
        }
        self.timings.push_back(timing);
        self.total_pushed += 1;
    }

    fn snapshot(&self) -> ThreadTaskTimings {
        ThreadTaskTimings {
            thread_name: self.thread_name.clone(),
            thread_id: self.thread_id,
            timings: self.timings.iter().copied().collect(),
            total_pushed: self.total_pushed,
        }
    }

    fn clear(&mut self) {
        self.timings.clear();
        self.timings.shrink_to_fit();
        self.total_pushed = 0;
    }
}

pub fn is_enabled() -> bool {
    PROFILER_ENABLED.load(Ordering::Relaxed)
}

pub fn set_enabled(enabled: bool) -> bool {
    if PROFILER_ENABLED.swap(enabled, Ordering::AcqRel) == enabled {
        return false;
    }

    PROFILER_GENERATION.fetch_add(1, Ordering::AcqRel);
    if !enabled {
        clear_all_timings();
    }
    true
}

pub(crate) fn record_start(location: &'static Location<'static>) -> Option<TaskTimingHandle> {
    if !is_enabled() {
        return None;
    }

    Some(TaskTimingHandle {
        location,
        start: ProfilingInstant::now(),
        generation: PROFILER_GENERATION.load(Ordering::Acquire),
    })
}

pub(crate) fn record_end(handle: TaskTimingHandle, status: TaskTimingStatus) {
    if !is_enabled() || handle.generation != PROFILER_GENERATION.load(Ordering::Acquire) {
        return;
    }

    let timing = TaskTiming {
        location: handle.location,
        start: handle.start,
        end: ProfilingInstant::now(),
        status,
    };

    THREAD_TIMINGS.with(|timings| timings.lock().push(timing));
}

#[doc(hidden)]
pub fn profiler_collect_timings(collector: &mut ProfilingCollector) -> Vec<ThreadTimingsDelta> {
    collector.collect_unseen(global_timings())
}

fn global_timings() -> Vec<ThreadTaskTimings> {
    GLOBAL_THREAD_TIMINGS
        .lock()
        .iter()
        .filter_map(|global| global.timings.upgrade())
        .map(|timings| timings.lock().snapshot())
        .collect()
}

fn clear_all_timings() {
    let mut global_timings = GLOBAL_THREAD_TIMINGS.lock();
    global_timings.retain(|global| {
        let Some(timings) = global.timings.upgrade() else {
            return false;
        };
        timings.lock().clear();
        true
    });
}

fn hash_thread_id(thread_id: ThreadId) -> u64 {
    let mut hasher = DefaultHasher::new();
    thread_id.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TEST_LOCK.lock().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_profiler() {
        set_enabled(false);
        clear_all_timings();
    }

    #[track_caller]
    fn current_location() -> &'static Location<'static> {
        Location::caller()
    }

    #[test]
    fn task_timing_records_start_and_end() {
        let _guard = test_guard();
        reset_profiler();
        set_enabled(true);

        let handle = record_start(current_location()).unwrap();
        record_end(handle, TaskTimingStatus::Completed);

        let timings = global_timings();
        let timing = timings
            .iter()
            .flat_map(|thread| &thread.timings)
            .next()
            .unwrap();
        assert!(timing.end >= timing.start);
        assert_eq!(timing.status, TaskTimingStatus::Completed);

        reset_profiler();
    }

    #[test]
    fn profiling_disabled_records_nothing() {
        let _guard = test_guard();
        reset_profiler();

        assert!(record_start(current_location()).is_none());
        assert!(
            global_timings()
                .iter()
                .all(|thread| thread.timings.is_empty())
        );
    }

    #[test]
    fn thread_task_timings_append_and_retrieve() {
        let _guard = test_guard();
        reset_profiler();

        THREAD_TIMINGS.with(|timings| {
            let mut timings = timings.lock();
            timings.push(TaskTiming {
                location: current_location(),
                start: ProfilingInstant::now(),
                end: ProfilingInstant::now(),
                status: TaskTimingStatus::Completed,
            });
            let snapshot = timings.snapshot();
            assert_eq!(snapshot.timings.len(), 1);
            assert_eq!(snapshot.total_pushed, 1);
        });

        reset_profiler();
    }

    #[test]
    fn ring_buffer_wraps_without_panic() {
        let _guard = test_guard();
        reset_profiler();

        let mut state = ThreadTimingState::new(None, std::thread::current().id());
        let now = ProfilingInstant::now();
        for _ in 0..(MAX_TASK_TIMINGS + 2) {
            state.push(TaskTiming {
                location: current_location(),
                start: now,
                end: now,
                status: TaskTimingStatus::Completed,
            });
        }

        assert_eq!(state.timings.len(), MAX_TASK_TIMINGS);
        assert_eq!(state.total_pushed, (MAX_TASK_TIMINGS + 2) as u64);
    }

    #[test]
    fn collector_returns_only_unseen_delta() {
        let _guard = test_guard();
        reset_profiler();
        set_enabled(true);

        let mut collector = ProfilingCollector::new(ProfilingInstant::now());
        let handle = record_start(current_location()).unwrap();
        record_end(handle, TaskTimingStatus::Completed);
        assert_eq!(profiler_collect_timings(&mut collector).len(), 1);
        assert!(profiler_collect_timings(&mut collector).is_empty());

        let handle = record_start(current_location()).unwrap();
        record_end(handle, TaskTimingStatus::Completed);
        let deltas = profiler_collect_timings(&mut collector);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].new_timings.len(), 1);

        reset_profiler();
    }

    #[test]
    fn collector_after_wrap_returns_remaining_data() {
        let _guard = test_guard();
        reset_profiler();

        let thread_id = std::thread::current().id();
        let now = ProfilingInstant::now();
        let timings = (0..3)
            .map(|_| TaskTiming {
                location: current_location(),
                start: now,
                end: now,
                status: TaskTimingStatus::Completed,
            })
            .collect();
        let all_timings = vec![ThreadTaskTimings {
            thread_name: None,
            thread_id,
            timings,
            total_pushed: 10,
        }];

        let mut collector = ProfilingCollector::new(now);
        collector.cursors.insert(thread_id, 1);
        let deltas = collector.collect_unseen(all_timings);
        assert_eq!(deltas[0].new_timings.len(), 3);
    }

    #[test]
    fn serialized_location_captures_file_line_column() {
        let location = current_location();
        let serialized = SerializedLocation::from(location);
        assert_eq!(serialized.file.as_ref(), location.file());
        assert_eq!(serialized.line, location.line());
        assert_eq!(serialized.column, location.column());
    }

    #[test]
    fn serialized_thread_task_timings_hashes_thread_id() {
        let now = ProfilingInstant::now();
        let thread = ThreadTaskTimings {
            thread_name: Some("test-thread".to_string()),
            thread_id: std::thread::current().id(),
            timings: vec![TaskTiming {
                location: current_location(),
                start: now,
                end: now,
                status: TaskTimingStatus::Completed,
            }],
            total_pushed: 1,
        };

        let serialized = SerializedThreadTaskTimings::convert(now, thread);
        assert_eq!(serialized.thread_name.as_deref(), Some("test-thread"));
        assert_ne!(serialized.thread_id, 0);
        assert_eq!(serialized.timings.len(), 1);
    }
}
