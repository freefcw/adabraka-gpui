use anyhow::Result;
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use std::ptr::null_mut;

pub fn set_auto_launch(_app_id: &str, enabled: bool) -> Result<()> {
    unsafe {
        let service: *mut Object = msg_send![class!(SMAppService), mainApp];
        if service.is_null() {
            return Err(anyhow::anyhow!(
                "SMAppService not available (requires macOS 13+)"
            ));
        }

        if enabled {
            let mut error: *mut Object = null_mut();
            let success: bool = msg_send![service, registerAndReturnError: &mut error];
            if !success {
                return Err(anyhow::anyhow!("Failed to register auto-launch"));
            }
        } else {
            let mut error: *mut Object = null_mut();
            let success: bool = msg_send![service, unregisterAndReturnError: &mut error];
            if !success {
                return Err(anyhow::anyhow!("Failed to unregister auto-launch"));
            }
        }

        Ok(())
    }
}

pub fn is_auto_launch_enabled(_app_id: &str) -> bool {
    unsafe {
        let service: *mut Object = msg_send![class!(SMAppService), mainApp];
        if service.is_null() {
            return false;
        }
        let status: isize = msg_send![service, status];
        status == 1
    }
}
