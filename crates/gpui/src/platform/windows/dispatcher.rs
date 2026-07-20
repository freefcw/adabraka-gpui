use std::{
    ffi::c_void,
    thread::{ThreadId, current},
    time::Duration,
};

use async_task::Runnable;
use flume::Sender;
use parking::Parker;
use parking_lot::Mutex;
use util::ResultExt;
use windows::Win32::{
    Foundation::{FILETIME, LPARAM, WPARAM},
    System::Threading::{
        CloseThreadpoolTimer, CreateThreadpoolTimer, PTP_CALLBACK_INSTANCE, PTP_TIMER,
        SetThreadpoolTimer, TP_CALLBACK_ENVIRON_V3, TP_CALLBACK_PRIORITY_HIGH,
        TrySubmitThreadpoolCallback,
    },
    UI::WindowsAndMessaging::PostMessageW,
};

use crate::{
    HWND, PlatformDispatcher, SafeHwnd, TaskLabel, WM_GPUI_TASK_DISPATCHED_ON_MAIN_THREAD,
};

pub(crate) struct WindowsDispatcher {
    main_sender: Sender<Runnable>,
    parker: Mutex<Parker>,
    main_thread_id: ThreadId,
    platform_window_handle: SafeHwnd,
    validation_number: usize,
}

impl WindowsDispatcher {
    pub(crate) fn new(
        main_sender: Sender<Runnable>,
        platform_window_handle: HWND,
        validation_number: usize,
    ) -> Self {
        let parker = Mutex::new(Parker::new());
        let main_thread_id = current().id();
        let platform_window_handle = platform_window_handle.into();

        WindowsDispatcher {
            main_sender,
            parker,
            main_thread_id,
            platform_window_handle,
            validation_number,
        }
    }

    fn dispatch_on_threadpool(&self, runnable: Runnable) {
        let environment = TP_CALLBACK_ENVIRON_V3 {
            Version: 3,
            CallbackPriority: TP_CALLBACK_PRIORITY_HIGH,
            Size: size_of::<TP_CALLBACK_ENVIRON_V3>() as u32,
            ..Default::default()
        };
        let context = Box::into_raw(Box::new(runnable)).cast::<c_void>();

        // On submission failure the callback cannot consume `context`. Leaking
        // it keeps the scheduled task pending instead of cancelling it.
        unsafe {
            TrySubmitThreadpoolCallback(Some(run_work_callback), Some(context), Some(&environment))
                .log_err();
        }
    }

    fn dispatch_on_threadpool_after(&self, runnable: Runnable, duration: Duration) {
        let context = Box::into_raw(Box::new(runnable)).cast::<c_void>();

        unsafe {
            let Ok(timer) = CreateThreadpoolTimer(Some(run_timer_callback), Some(context), None)
            else {
                // Keep the task pending when Windows cannot create the timer.
                return;
            };

            // Negative FILETIME values are relative delays in 100ns ticks.
            // Round up so the timer never fires before the requested duration.
            let ticks = duration.as_nanos().div_ceil(100).clamp(1, i64::MAX as u128) as i64;
            let due = (-ticks) as u64;
            let due_time = FILETIME {
                dwLowDateTime: due as u32,
                dwHighDateTime: (due >> 32) as u32,
            };
            SetThreadpoolTimer(timer, Some(&due_time), 0, None);
        }
    }
}

impl PlatformDispatcher for WindowsDispatcher {
    fn is_main_thread(&self) -> bool {
        current().id() == self.main_thread_id
    }

    fn dispatch(&self, runnable: Runnable, label: Option<TaskLabel>) {
        self.dispatch_on_threadpool(runnable);
        if let Some(label) = label {
            log::debug!("TaskLabel: {label:?}");
        }
    }

    fn dispatch_on_main_thread(&self, runnable: Runnable) {
        match self.main_sender.send(runnable) {
            Ok(_) => unsafe {
                PostMessageW(
                    Some(self.platform_window_handle.as_raw()),
                    WM_GPUI_TASK_DISPATCHED_ON_MAIN_THREAD,
                    WPARAM(self.validation_number),
                    LPARAM(0),
                )
                .log_err();
            },
            Err(runnable) => {
                // NOTE: Runnable may wrap a Future that is !Send.
                //
                // This is usually safe because we only poll it on the main thread.
                // However if the send fails, we know that:
                // 1. main_receiver has been dropped (which implies the app is shutting down)
                // 2. we are on a background thread.
                // It is not safe to drop something !Send on the wrong thread, and
                // the app will exit soon anyway, so we must forget the runnable.
                std::mem::forget(runnable);
            }
        }
    }

    fn dispatch_after(&self, duration: Duration, runnable: Runnable) {
        self.dispatch_on_threadpool_after(runnable, duration);
    }

    fn park(&self, timeout: Option<Duration>) -> bool {
        if let Some(timeout) = timeout {
            self.parker.lock().park_timeout(timeout)
        } else {
            self.parker.lock().park();
            true
        }
    }

    fn unparker(&self) -> parking::Unparker {
        self.parker.lock().unparker()
    }
}

/// Reclaims the runnable transferred to a Windows thread-pool callback.
///
/// # Safety
///
/// `context` must come from `Box::into_raw(Box::new(runnable))` and this
/// function must be called exactly once for that pointer.
unsafe fn take_runnable(context: *mut c_void) -> Runnable {
    *unsafe { Box::from_raw(context.cast::<Runnable>()) }
}

unsafe extern "system" fn run_work_callback(
    _instance: PTP_CALLBACK_INSTANCE,
    context: *mut c_void,
) {
    unsafe { take_runnable(context) }.run();
}

unsafe extern "system" fn run_timer_callback(
    _instance: PTP_CALLBACK_INSTANCE,
    context: *mut c_void,
    timer: PTP_TIMER,
) {
    unsafe { take_runnable(context) }.run();
    unsafe { CloseThreadpoolTimer(timer) };
}
