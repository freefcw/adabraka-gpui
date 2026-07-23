use crate::{
    DevicePixels, ForegroundExecutor, ScreenCaptureStreamTermination,
    ScreenCaptureTerminationCallback, SharedString, SourceMetadata,
    platform::{ScreenCaptureFrame, ScreenCaptureSource, ScreenCaptureStream},
    size,
};
use anyhow::{Result, anyhow};
use block::ConcreteBlock;
use collections::HashMap;
use core_foundation::base::TCFType;
use core_graphics::display::{
    CGDirectDisplayID, CGDisplayCopyDisplayMode, CGDisplayModeGetPixelHeight,
    CGDisplayModeGetPixelWidth, CGDisplayModeRelease,
};
use ctor::ctor;
use futures::channel::oneshot;
use media::core_media::{CMSampleBuffer, CMSampleBufferRef};
use metal::NSInteger;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel, YES},
    sel, sel_impl,
};
use std::{
    cell::RefCell,
    ffi::c_void,
    ptr,
    rc::Rc,
    sync::{LazyLock, Mutex},
};

use super::{NSStringExt, ns_string};

type ObjcId = *mut Object;
type FrameCallback = Box<dyn Fn(ScreenCaptureFrame) + Send>;

#[allow(non_camel_case_types)]
type id = ObjcId;

#[allow(non_upper_case_globals)]
const nil: ObjcId = ptr::null_mut();

unsafe fn array_count(array: id) -> usize {
    unsafe { msg_send![array, count] }
}

unsafe fn array_object_at_index(array: id, index: usize) -> id {
    unsafe { msg_send![array, objectAtIndex: index] }
}

#[derive(Clone)]
pub struct MacScreenCaptureSource {
    sc_display: id,
    meta: Option<ScreenMeta>,
}

pub struct MacScreenCaptureStream {
    sc_stream: id,
    sc_stream_output: id,
    meta: SourceMetadata,
}

static mut DELEGATE_CLASS: *const Class = ptr::null();
static mut OUTPUT_CLASS: *const Class = ptr::null();
const FRAME_CALLBACK_IVAR: &str = "frame_callback";

type StreamTerminationCallbacks = HashMap<usize, ScreenCaptureTerminationCallback>;

static STREAM_TERMINATION_CALLBACKS: LazyLock<Mutex<StreamTerminationCallbacks>> =
    LazyLock::new(|| Mutex::new(HashMap::default()));

fn register_stream_termination_callback(stream: id, callback: ScreenCaptureTerminationCallback) {
    let previous = STREAM_TERMINATION_CALLBACKS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(stream as usize, callback);
    if let Some(previous) = previous {
        previous(ScreenCaptureStreamTermination::Cancelled);
    }
}

fn take_stream_termination_callback(stream: id) -> Option<ScreenCaptureTerminationCallback> {
    STREAM_TERMINATION_CALLBACKS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&(stream as usize))
}

fn notify_stream_termination(stream: id, termination: ScreenCaptureStreamTermination) {
    if let Some(callback) = take_stream_termination_callback(stream) {
        callback(termination);
    }
}

#[allow(non_upper_case_globals)]
const SCStreamOutputTypeScreen: NSInteger = 0;
// `SCStreamErrorUserStopped` from ScreenCaptureKit/SCError.h.
const SC_STREAM_ERROR_USER_STOPPED: NSInteger = -3817;

fn stream_termination_from_error(
    error_code: NSInteger,
    description: SharedString,
) -> ScreenCaptureStreamTermination {
    if error_code == SC_STREAM_ERROR_USER_STOPPED {
        ScreenCaptureStreamTermination::Cancelled
    } else {
        ScreenCaptureStreamTermination::Failed(description)
    }
}

impl ScreenCaptureSource for MacScreenCaptureSource {
    fn metadata(&self) -> Result<SourceMetadata> {
        let (display_id, size) = unsafe {
            let display_id: CGDirectDisplayID = msg_send![self.sc_display, displayID];
            let display_mode_ref = CGDisplayCopyDisplayMode(display_id);
            let width = CGDisplayModeGetPixelWidth(display_mode_ref);
            let height = CGDisplayModeGetPixelHeight(display_mode_ref);
            CGDisplayModeRelease(display_mode_ref);

            (
                display_id,
                size(DevicePixels(width as i32), DevicePixels(height as i32)),
            )
        };
        let (label, is_main) = self
            .meta
            .clone()
            .map(|meta| (meta.label, meta.is_main))
            .unzip();

        Ok(SourceMetadata {
            id: display_id as u64,
            label,
            is_main,
            resolution: size,
        })
    }

    fn stream(
        &self,
        foreground_executor: &ForegroundExecutor,
        frame_callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
    ) -> oneshot::Receiver<Result<Box<dyn ScreenCaptureStream>>> {
        self.stream_with_termination(foreground_executor, frame_callback, Box::new(|_| {}))
    }

    fn stream_with_termination(
        &self,
        _foreground_executor: &ForegroundExecutor,
        frame_callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
        termination_callback: ScreenCaptureTerminationCallback,
    ) -> oneshot::Receiver<Result<Box<dyn ScreenCaptureStream>>> {
        self.start_stream(frame_callback, termination_callback)
    }
}

impl MacScreenCaptureSource {
    fn start_stream(
        &self,
        frame_callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
        termination_callback: ScreenCaptureTerminationCallback,
    ) -> oneshot::Receiver<Result<Box<dyn ScreenCaptureStream>>> {
        unsafe {
            let stream: id = msg_send![class!(SCStream), alloc];
            let filter: id = msg_send![class!(SCContentFilter), alloc];
            let configuration: id = msg_send![class!(SCStreamConfiguration), alloc];
            let delegate: id = msg_send![DELEGATE_CLASS, alloc];
            let output: id = msg_send![OUTPUT_CLASS, alloc];

            let excluded_windows: id = msg_send![class!(NSArray), array];
            let filter: id = msg_send![filter, initWithDisplay:self.sc_display excludingWindows:excluded_windows];
            let configuration: id = msg_send![configuration, init];
            let _: id = msg_send![configuration, setScalesToFit: true];
            let _: id = msg_send![configuration, setPixelFormat: 0x42475241];
            // let _: id = msg_send![configuration, setShowsCursor: false];
            // let _: id = msg_send![configuration, setCaptureResolution: 3];
            let delegate: id = msg_send![delegate, init];
            let output: id = msg_send![output, init];

            output.as_mut().unwrap().set_ivar(
                FRAME_CALLBACK_IVAR,
                Box::into_raw(Box::new(frame_callback)) as *mut c_void,
            );

            let meta = self.metadata().unwrap();
            let _: id = msg_send![configuration, setWidth: meta.resolution.width.0 as i64];
            let _: id = msg_send![configuration, setHeight: meta.resolution.height.0 as i64];
            let stream: id = msg_send![stream, initWithFilter:filter configuration:configuration delegate:delegate];

            // `SCStream` retains these objects for its own lifetime.
            let _: () = msg_send![filter, release];
            let _: () = msg_send![configuration, release];
            let _: () = msg_send![delegate, release];

            let (mut tx, rx) = oneshot::channel();

            let mut error: id = nil;
            let _: () = msg_send![stream, addStreamOutput:output type:SCStreamOutputTypeScreen sampleHandlerQueue:0 error:&mut error as *mut id];
            if error != nil {
                let message: id = msg_send![error, localizedDescription];
                let _: () = msg_send![stream, release];
                let _: () = msg_send![output, release];
                tx.send(Err(anyhow!("failed to add stream output {message:?}")))
                    .ok();
                return rx;
            }

            register_stream_termination_callback(stream, termination_callback);

            let tx = Rc::new(RefCell::new(Some(tx)));
            let handler = ConcreteBlock::new({
                move |error: id| {
                    let result = if error == nil {
                        let stream = MacScreenCaptureStream {
                            meta: meta.clone(),
                            sc_stream: stream,
                            sc_stream_output: output,
                        };
                        Ok(Box::new(stream) as Box<dyn ScreenCaptureStream>)
                    } else {
                        take_stream_termination_callback(stream);
                        let _: () = msg_send![stream, release];
                        let _: () = msg_send![output, release];
                        let message: id = msg_send![error, localizedDescription];
                        Err(anyhow!("failed to start screen capture stream {message:?}"))
                    };
                    if let Some(tx) = tx.borrow_mut().take() {
                        tx.send(result).ok();
                    }
                }
            });
            let handler = handler.copy();
            let _: () = msg_send![stream, startCaptureWithCompletionHandler:handler];
            rx
        }
    }
}

impl Drop for MacScreenCaptureSource {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.sc_display, release];
        }
    }
}

impl ScreenCaptureStream for MacScreenCaptureStream {
    fn metadata(&self) -> Result<SourceMetadata> {
        Ok(self.meta.clone())
    }
}

impl Drop for MacScreenCaptureStream {
    fn drop(&mut self) {
        notify_stream_termination(self.sc_stream, ScreenCaptureStreamTermination::Cancelled);

        unsafe {
            let mut error: id = nil;
            let _: () = msg_send![self.sc_stream, removeStreamOutput:self.sc_stream_output type:SCStreamOutputTypeScreen error:&mut error as *mut _];
            if error != nil {
                let message: id = msg_send![error, localizedDescription];
                log::error!("failed to add stream  output {message:?}");
            }

            let handler = ConcreteBlock::new(move |error: id| {
                if error != nil {
                    let message: id = msg_send![error, localizedDescription];
                    log::error!("failed to stop screen capture stream {message:?}");
                }
            });
            let block = handler.copy();
            let _: () = msg_send![self.sc_stream, stopCaptureWithCompletionHandler:block];
            let _: () = msg_send![self.sc_stream, release];
            let _: () = msg_send![self.sc_stream_output, release];
        }
    }
}

#[derive(Clone)]
struct ScreenMeta {
    label: SharedString,
    // Is this the screen with menu bar?
    is_main: bool,
}

unsafe fn screen_id_to_human_label() -> HashMap<CGDirectDisplayID, ScreenMeta> {
    let screens: id = msg_send![class!(NSScreen), screens];
    let count = unsafe { array_count(screens) };
    let mut map = HashMap::default();
    let screen_number_key = unsafe { ns_string("NSScreenNumber") };
    for i in 0..count {
        let screen = unsafe { array_object_at_index(screens, i) };
        let device_desc: id = msg_send![screen, deviceDescription];
        if device_desc == nil {
            continue;
        }

        let nsnumber: id = msg_send![device_desc, objectForKey: screen_number_key];
        if nsnumber == nil {
            continue;
        }

        let screen_id: u32 = msg_send![nsnumber, unsignedIntValue];

        let name: id = msg_send![screen, localizedName];
        if name != nil {
            let cstr: *const std::os::raw::c_char = msg_send![name, UTF8String];
            let rust_str = unsafe {
                std::ffi::CStr::from_ptr(cstr)
                    .to_string_lossy()
                    .into_owned()
            };
            map.insert(
                screen_id,
                ScreenMeta {
                    label: rust_str.into(),
                    is_main: i == 0,
                },
            );
        }
    }
    map
}

pub(crate) fn get_sources() -> oneshot::Receiver<Result<Vec<Rc<dyn ScreenCaptureSource>>>> {
    unsafe {
        let (mut tx, rx) = oneshot::channel();
        let tx = Rc::new(RefCell::new(Some(tx)));
        let screen_id_to_label = screen_id_to_human_label();
        let block = ConcreteBlock::new(move |shareable_content: id, error: id| {
            let Some(mut tx) = tx.borrow_mut().take() else {
                return;
            };

            let result = if error == nil {
                let displays: id = msg_send![shareable_content, displays];
                let mut result = Vec::new();
                for i in 0..array_count(displays) {
                    let display = array_object_at_index(displays, i);
                    let id: CGDirectDisplayID = msg_send![display, displayID];
                    let meta = screen_id_to_label.get(&id).cloned();
                    let source = MacScreenCaptureSource {
                        sc_display: msg_send![display, retain],
                        meta,
                    };
                    result.push(Rc::new(source) as Rc<dyn ScreenCaptureSource>);
                }
                Ok(result)
            } else {
                let msg: id = msg_send![error, localizedDescription];
                Err(anyhow!(
                    "Screen share failed: {:?}",
                    NSStringExt::to_str(&msg)
                ))
            };
            tx.send(result).ok();
        });
        let block = block.copy();

        let _: () = msg_send![
            class!(SCShareableContent),
            getShareableContentExcludingDesktopWindows:YES
                                   onScreenWindowsOnly:YES
                                     completionHandler:block];
        rx
    }
}

#[ctor]
unsafe fn build_classes() {
    let mut decl = ClassDecl::new("GPUIStreamDelegate", class!(NSObject)).unwrap();
    unsafe {
        decl.add_method(
            sel!(outputVideoEffectDidStartForStream:),
            output_video_effect_did_start_for_stream as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(outputVideoEffectDidStopForStream:),
            output_video_effect_did_stop_for_stream as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(stream:didStopWithError:),
            stream_did_stop_with_error as extern "C" fn(&Object, Sel, id, id),
        );
        DELEGATE_CLASS = decl.register();

        let mut decl = ClassDecl::new("GPUIStreamOutput", class!(NSObject)).unwrap();
        decl.add_method(
            sel!(dealloc),
            dealloc_stream_output as extern "C" fn(&Object, Sel),
        );
        decl.add_method(
            sel!(stream:didOutputSampleBuffer:ofType:),
            stream_did_output_sample_buffer_of_type
                as extern "C" fn(&Object, Sel, id, id, NSInteger),
        );
        decl.add_ivar::<*mut c_void>(FRAME_CALLBACK_IVAR);

        OUTPUT_CLASS = decl.register();
    }
}

extern "C" fn output_video_effect_did_start_for_stream(_this: &Object, _: Sel, _stream: id) {}

extern "C" fn output_video_effect_did_stop_for_stream(_this: &Object, _: Sel, _stream: id) {}

extern "C" fn stream_did_stop_with_error(_this: &Object, _: Sel, stream: id, error: id) {
    let termination = unsafe {
        if error == nil {
            ScreenCaptureStreamTermination::Ended
        } else {
            let error_code: NSInteger = msg_send![error, code];
            let message: id = msg_send![error, localizedDescription];
            stream_termination_from_error(error_code, NSStringExt::to_str(&message).into())
        }
    };
    notify_stream_termination(stream, termination);
}

unsafe fn drop_stream_output_callback(output: &Object) {
    unsafe {
        let callback = *output.get_ivar::<*mut c_void>(FRAME_CALLBACK_IVAR) as *mut FrameCallback;
        if callback.is_null() {
            return;
        }

        drop(Box::from_raw(callback));
    }
}

extern "C" fn dealloc_stream_output(this: &Object, _: Sel) {
    unsafe {
        drop_stream_output_callback(this);
        let _: () = msg_send![super(this, class!(NSObject)), dealloc];
    }
}

extern "C" fn stream_did_output_sample_buffer_of_type(
    this: &Object,
    _: Sel,
    _stream: id,
    sample_buffer: id,
    buffer_type: NSInteger,
) {
    if buffer_type != SCStreamOutputTypeScreen {
        return;
    }

    unsafe {
        let sample_buffer = sample_buffer as CMSampleBufferRef;
        let sample_buffer = CMSampleBuffer::wrap_under_get_rule(sample_buffer);
        if let Some(buffer) = sample_buffer.image_buffer() {
            let callback =
                &*(*this.get_ivar::<*mut c_void>(FRAME_CALLBACK_IVAR) as *const FrameCallback);
            callback(ScreenCaptureFrame(buffer));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ptr::NonNull, sync::mpsc, time::Duration};

    struct CallbackDropSignal(mpsc::Sender<()>);

    impl Drop for CallbackDropSignal {
        fn drop(&mut self) {
            self.0.send(()).unwrap();
        }
    }

    #[test]
    fn user_stopped_stream_error_is_cancelled() {
        assert_eq!(
            stream_termination_from_error(-3817, "The stream was stopped by the user".into()),
            ScreenCaptureStreamTermination::Cancelled
        );
    }

    #[test]
    fn runtime_stream_error_preserves_its_failure_description() {
        assert_eq!(
            stream_termination_from_error(-3811, "Screen capture failed".into()),
            ScreenCaptureStreamTermination::Failed("Screen capture failed".into())
        );
    }

    #[test]
    fn terminal_notification_is_delivered_once() {
        let stream = NonNull::<Object>::dangling().as_ptr();
        let (sender, receiver) = mpsc::channel();
        register_stream_termination_callback(
            stream,
            Box::new(move |termination| sender.send(termination).unwrap()),
        );

        notify_stream_termination(
            stream,
            ScreenCaptureStreamTermination::Failed("capture device disconnected".into()),
        );
        notify_stream_termination(stream, ScreenCaptureStreamTermination::Ended);

        assert_eq!(
            receiver.recv().unwrap(),
            ScreenCaptureStreamTermination::Failed("capture device disconnected".into())
        );
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn stream_output_dealloc_releases_frame_callback() {
        let (sender, receiver) = mpsc::channel();
        let drop_signal = CallbackDropSignal(sender);
        let callback: FrameCallback = Box::new(move |_| {
            let _ = &drop_signal;
        });

        unsafe {
            let output: id = msg_send![OUTPUT_CLASS, alloc];
            let output: id = msg_send![output, init];
            output.as_mut().unwrap().set_ivar(
                FRAME_CALLBACK_IVAR,
                Box::into_raw(Box::new(callback)) as *mut c_void,
            );
            let _: () = msg_send![output, release];
        }

        assert_eq!(receiver.recv_timeout(Duration::from_millis(100)), Ok(()));
    }
}
