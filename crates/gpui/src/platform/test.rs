mod dispatcher;
mod display;
mod platform;
#[cfg(target_os = "macos")]
mod visual;
mod window;

pub use dispatcher::*;
pub(crate) use display::*;
pub(crate) use platform::*;
pub(crate) use window::*;

pub use platform::{TestScreenCaptureSource, TestScreenCaptureStream};
#[cfg(target_os = "macos")]
pub(crate) use visual::VisualTestPlatform;
pub use window::VisualRenderArtifact;
