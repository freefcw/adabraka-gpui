use crate::{BiometricKind, BiometricStatus};
use objc::runtime::{BOOL, Object, YES};
use objc::{class, msg_send, sel, sel_impl};
use objc2::rc::Retained;
use objc2_foundation::NSString;
use std::ptr::null_mut;

const LA_POLICY_BIOMETRICS: i64 = 1;

#[link(name = "LocalAuthentication", kind = "framework")]
unsafe extern "C" {}

pub fn biometric_status() -> BiometricStatus {
    unsafe {
        let context: *mut Object = msg_send![class!(LAContext), new];
        let mut error: *mut Object = null_mut();
        let can_evaluate: BOOL = msg_send![
            context,
            canEvaluatePolicy: LA_POLICY_BIOMETRICS
            error: &mut error
        ];
        let _: () = msg_send![context, release];
        if can_evaluate == YES {
            BiometricStatus::Available(BiometricKind::TouchId)
        } else {
            BiometricStatus::Unavailable
        }
    }
}

pub fn authenticate_biometric(reason: &str, callback: Box<dyn FnOnce(bool) + Send>) {
    unsafe {
        let context: *mut Object = msg_send![class!(LAContext), new];
        let reason_ns = NSString::from_str(reason);
        let reason_ns = Retained::as_ptr(&reason_ns).cast_mut().cast::<Object>();

        let callback = std::sync::Mutex::new(Some(callback));

        let block = block::ConcreteBlock::new(move |success: BOOL, _error: *mut Object| {
            if let Some(cb) = callback.lock().ok().and_then(|mut guard| guard.take()) {
                cb(success == YES);
            }
        });
        let block = block.copy();

        let _: () = msg_send![
            context,
            evaluatePolicy: LA_POLICY_BIOMETRICS
            localizedReason: reason_ns
            reply: &*block
        ];

        let _: () = msg_send![context, release];
    }
}
