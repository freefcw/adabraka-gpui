use crate::OsInfo;
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::CStr;

unsafe fn nsstring_to_string(nsstring: *mut Object) -> String {
    if nsstring.is_null() {
        return String::new();
    }
    let cstr: *const std::ffi::c_char = msg_send![nsstring, UTF8String];
    if cstr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(cstr) }
        .to_string_lossy()
        .to_string()
}

pub fn get_os_info() -> OsInfo {
    unsafe {
        let process_info: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
        let version_string: *mut Object = msg_send![process_info, operatingSystemVersionString];
        let version = nsstring_to_string(version_string);

        let current_locale: *mut Object = msg_send![class!(NSLocale), currentLocale];
        let locale_id: *mut Object = msg_send![current_locale, localeIdentifier];
        let locale = nsstring_to_string(locale_id);

        let mut hostname_buf = [0u8; 256];
        let hostname = if libc::gethostname(
            hostname_buf.as_mut_ptr() as *mut libc::c_char,
            hostname_buf.len(),
        ) == 0
        {
            hostname_buf[hostname_buf.len() - 1] = 0;
            CStr::from_ptr(hostname_buf.as_ptr() as *const libc::c_char)
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        };

        OsInfo {
            name: "macOS".into(),
            version: version.into(),
            arch: std::env::consts::ARCH.into(),
            locale: locale.into(),
            hostname: hostname.into(),
        }
    }
}
