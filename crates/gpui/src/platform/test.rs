mod dispatcher;
mod display;
mod platform;
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows"
))]
mod visual;
mod window;

pub use dispatcher::*;
pub(crate) use display::*;
pub(crate) use platform::*;
pub(crate) use window::*;

pub use platform::{TestScreenCaptureSource, TestScreenCaptureStream};
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows"
))]
pub(crate) use visual::VisualTestPlatform;
pub use window::VisualRenderArtifact;
