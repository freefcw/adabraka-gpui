use crate::AttentionType;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSRequestUserAttentionType};
use objc2_foundation::NSString;

pub fn request_user_attention(attention_type: AttentionType) -> isize {
    let app = unsafe { NSApplication::sharedApplication(MainThreadMarker::new_unchecked()) };
    let request_type = match attention_type {
        AttentionType::Informational => NSRequestUserAttentionType::InformationalRequest,
        AttentionType::Critical => NSRequestUserAttentionType::CriticalRequest,
    };
    app.requestUserAttention(request_type)
}

pub fn cancel_user_attention(request_id: isize) {
    let app = unsafe { NSApplication::sharedApplication(MainThreadMarker::new_unchecked()) };
    unsafe {
        app.cancelUserAttentionRequest(request_id);
    }
}

pub fn set_dock_badge(label: Option<&str>) {
    let app = unsafe { NSApplication::sharedApplication(MainThreadMarker::new_unchecked()) };
    let dock_tile = unsafe { app.dockTile() };
    if let Some(text) = label {
        let label = NSString::from_str(text);
        unsafe {
            dock_tile.setBadgeLabel(Some(&label));
        }
    } else {
        unsafe {
            dock_tile.setBadgeLabel(None);
        }
    }
}
