#![doc = include_str!("../README.md")]
#![cfg(target_os = "windows")]
#![allow(unused_mut)] // False positives in platform-specific state handling

#[path = "windows/mod.rs"]
mod backend;

pub(crate) use backend::*;

pub(crate) use gpui::*;

use std::rc::Rc;

/// Returns the default Windows platform implementation for this process.
pub fn current_platform(_headless: bool) -> Rc<dyn gpui::Platform> {
    Rc::new(
        WindowsPlatform::new()
            .inspect_err(|error| show_error("Failed to launch", error.to_string()))
            .unwrap(),
    )
}
