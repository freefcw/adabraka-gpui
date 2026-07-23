//! Macos screen have a y axis that goings up from the bottom of the screen and
//! an origin at the bottom left of the main display.
mod dispatcher;
mod display;
mod display_link;
mod events;
mod keyboard;

#[cfg(feature = "screen-capture")]
mod screen_capture;

mod metal_atlas;
pub mod metal_renderer;

use core_video::image_buffer::CVImageBuffer;
use metal_renderer as renderer;

mod attributed_string;

#[cfg(feature = "font-kit")]
mod open_type;

#[cfg(feature = "font-kit")]
mod text_system;

mod active_window;
mod auto_launch;
mod biometric;
mod dialog;
mod dock;
mod global_hotkey;
mod network;
mod os_info;
mod permissions;
mod platform;
mod power;
mod tray;
mod window;
mod window_appearance;

use crate::{DevicePixels, Pixels, Size, px, size};
use objc::{
    msg_send,
    runtime::{BOOL, NO, Object, YES},
    sel, sel_impl,
};
use objc2::rc::Retained;
use objc2_foundation::{NSNotFound, NSRect, NSSize, NSString};
use std::{
    ffi::{CStr, c_char},
    ops::Range,
};

pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub(crate) use display_link::*;
pub(crate) use keyboard::*;
pub(crate) use platform::*;
pub(crate) use window::*;

#[cfg(feature = "font-kit")]
pub(crate) use text_system::*;

/// A frame of video captured from a screen.
pub(crate) type PlatformScreenCaptureFrame = CVImageBuffer;

trait BoolExt {
    fn to_objc(self) -> BOOL;
}

impl BoolExt for bool {
    fn to_objc(self) -> BOOL {
        if self { YES } else { NO }
    }
}

trait NSStringExt {
    unsafe fn to_str(&self) -> &str;
}

impl NSStringExt for *mut Object {
    unsafe fn to_str(&self) -> &str {
        unsafe {
            let cstr: *const c_char = msg_send![*self, UTF8String];
            if cstr.is_null() {
                ""
            } else {
                CStr::from_ptr(cstr).to_str().unwrap()
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct NSRange {
    pub location: usize,
    pub length: usize,
}

impl NSRange {
    fn invalid() -> Self {
        Self {
            location: NSNotFound as usize,
            length: 0,
        }
    }

    fn is_valid(&self) -> bool {
        self.location != NSNotFound as usize
    }

    fn to_range(self) -> Option<Range<usize>> {
        if self.is_valid() {
            let start = self.location;
            let end = start + self.length;
            Some(start..end)
        } else {
            None
        }
    }
}

impl From<Range<usize>> for NSRange {
    fn from(range: Range<usize>) -> Self {
        NSRange {
            location: range.start,
            length: range.len(),
        }
    }
}

unsafe impl objc::Encode for NSRange {
    fn encode() -> objc::Encoding {
        let encoding = format!(
            "{{NSRange={}{}}}",
            usize::encode().as_str(),
            usize::encode().as_str()
        );
        unsafe { objc::Encoding::from_str(&encoding) }
    }
}

unsafe fn ns_string(string: &str) -> *mut Object {
    let string = Retained::into_raw(NSString::from_str(string)).cast::<Object>();
    unsafe { msg_send![string, autorelease] }
}

impl From<NSSize> for Size<Pixels> {
    fn from(value: NSSize) -> Self {
        Size {
            width: px(value.width as f32),
            height: px(value.height as f32),
        }
    }
}

impl From<NSRect> for Size<Pixels> {
    fn from(rect: NSRect) -> Self {
        let NSSize { width, height } = rect.size;
        size(width.into(), height.into())
    }
}

impl From<NSRect> for Size<DevicePixels> {
    fn from(rect: NSRect) -> Self {
        let NSSize { width, height } = rect.size;
        size(DevicePixels(width as i32), DevicePixels(height as i32))
    }
}
