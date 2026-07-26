#![doc = include_str!("../README.md")]

mod wgpu_atlas;
mod wgpu_context;
mod wgpu_renderer;

pub use wgpu;
pub(crate) use wgpu_atlas::*;
#[doc(hidden)]
pub use wgpu_context::CompositorGpuHint;
pub(crate) use wgpu_context::WgpuContext;
pub use wgpu_renderer::{GpuContext, WgpuRenderer, WgpuSurfaceConfig};
