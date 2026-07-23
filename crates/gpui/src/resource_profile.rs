//! Resource budget configuration for GPUI applications.
//!
//! Different application types have vastly different resource needs. A full IDE
//! benefits from large caches and atlas textures, while a tray icon or a small
//! popover panel can run with significantly less memory.
//!
//! [`AppResourceProfile`] allows callers to tune internal cache sizes and GPU
//! resource allocation at application startup via
//! [`Application::with_resource_profile`](crate::Application::with_resource_profile).
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use gpui::{Application, AppProfile};
//!
//! Application::new()
//!     .with_resource_profile(AppProfile::Minimal)
//!     .run(|cx| {
//!         // open your tiny window...
//!     });
//! ```
//!
//! # Memory Characteristics
//!
//! - **Text layout cache** (`line_layout_cache_max_entries`): Controls the
//!   worst-case heap memory of the global line-layout cache. The cache grows on
//!   demand; this value only caps how large it can get before eviction kicks in.
//!   The cache stores both unwrapped and wrapped line layouts, each with its own
//!   entry limit. At ~1.2 KB per entry, 10 000 entries in one cache can occupy
//!   up to ~12 MB at peak.
//!
//! - **Atlas initial size** (`atlas_initial_size`): Controls the dimensions of
//!   GPU textures allocated for the glyph/image atlas. Each 1024×1024 BGRA
//!   texture costs 4 MB of GPU memory; a 512×512 texture costs 1 MB. These are
//!   **GPU** resources, not regular heap allocations.
//!
//! - **Instance buffer initial size** (`instance_buffer_initial_size`):
//!   Controls the initial capacity of renderer instance buffers where supported.
//!   Renderers may grow beyond this value automatically when a scene requires it.

use crate::{DevicePixels, Size};

/// Predefined application profiles for common use cases.
///
/// Use these as a convenient starting point. For fine-grained control, use
/// [`AppProfile::Custom`] with a manually constructed [`AppResourceProfile`].
#[derive(Clone, Debug, Default)]
pub enum AppProfile {
    /// Full desktop application (IDE, editor, browser).
    ///
    /// Large caches, standard atlas size. This is the default if no profile is
    /// specified.
    #[default]
    Desktop,

    /// Lightweight utility window (settings panel, dialog box).
    ///
    /// Moderate cache sizes, standard atlas.
    Utility,

    /// Minimal resource usage (tray icon, status bar, notification popup).
    ///
    /// Small caches, smaller atlas textures. Optimized for long-running
    /// processes with minimal UI.
    Minimal,

    /// Fully custom resource budget.
    Custom(AppResourceProfile),
}

impl AppProfile {
    /// Convert this profile into a concrete [`AppResourceProfile`].
    pub fn to_resource_profile(&self) -> AppResourceProfile {
        match self {
            AppProfile::Desktop => AppResourceProfile::desktop(),
            AppProfile::Utility => AppResourceProfile::utility(),
            AppProfile::Minimal => AppResourceProfile::minimal(),
            AppProfile::Custom(profile) => profile.clone(),
        }
    }
}

impl From<AppProfile> for AppResourceProfile {
    fn from(profile: AppProfile) -> Self {
        profile.to_resource_profile()
    }
}

/// Fine-grained resource budget for a GPUI application.
///
/// All fields have sensible defaults corresponding to [`AppProfile::Desktop`].
#[derive(Clone, Debug)]
pub struct AppResourceProfile {
    /// Text layout cache configuration.
    pub text: TextResourceBudget,

    /// GPU / Atlas resource configuration.
    pub gpu: GpuResourceBudget,

    /// Initial capacity (in bytes) of the per-thread element arena.
    ///
    /// The element arena is a bump allocator used during each frame's layout
    /// and paint phases. It is cleared after every frame, so the initial size
    /// only determines how much memory is pre-allocated to avoid mid-frame
    /// reallocations.
    ///
    /// Default: 1 048 576 (1 MiB). For minimal applications, 256 KiB or
    /// 512 KiB is recommended.
    pub element_arena_size: usize,
}

impl AppResourceProfile {
    /// Preset for full desktop applications.
    pub fn desktop() -> Self {
        Self {
            text: TextResourceBudget {
                line_layout_cache_max_entries: 10_000,
                line_layout_cache_low_watermark: 5_000,
                raster_bounds_cache_max_entries: None,
            },
            gpu: GpuResourceBudget {
                atlas_initial_size: 1024,
                instance_buffer_initial_size: 2 * 1024 * 1024,
            },
            element_arena_size: 1024 * 1024,
        }
    }

    /// Preset for utility/tool windows.
    pub fn utility() -> Self {
        Self {
            text: TextResourceBudget {
                line_layout_cache_max_entries: 3_000,
                line_layout_cache_low_watermark: 2_000,
                raster_bounds_cache_max_entries: Some(5_000),
            },
            gpu: GpuResourceBudget {
                atlas_initial_size: 1024,
                instance_buffer_initial_size: 1024 * 1024,
            },
            element_arena_size: 512 * 1024,
        }
    }

    /// Preset for minimal/tray applications.
    pub fn minimal() -> Self {
        Self {
            text: TextResourceBudget {
                line_layout_cache_max_entries: 500,
                line_layout_cache_low_watermark: 400,
                raster_bounds_cache_max_entries: Some(2_000),
            },
            gpu: GpuResourceBudget {
                atlas_initial_size: 512,
                instance_buffer_initial_size: 512 * 1024,
            },
            element_arena_size: 256 * 1024,
        }
    }

    /// Returns the atlas initial size as a [`Size<DevicePixels>`] for internal use.
    pub(crate) fn atlas_size(&self) -> Size<DevicePixels> {
        let s = self.gpu.atlas_initial_size as i32;
        Size {
            width: DevicePixels(s),
            height: DevicePixels(s),
        }
    }
}

impl Default for AppResourceProfile {
    fn default() -> Self {
        Self::desktop()
    }
}

/// Text system resource budget.
#[derive(Clone, Debug)]
pub struct TextResourceBudget {
    /// Maximum number of entries in the global line-layout cache.
    ///
    /// When the cache reaches this limit, older entries are evicted down to the
    /// low watermark. Default: 10 000.
    ///
    /// Each entry occupies approximately 1.2 KB on average (varies with text
    /// length and font complexity).
    pub line_layout_cache_max_entries: usize,

    /// Target number of entries to retain after eviction.
    ///
    /// When the cache exceeds `line_layout_cache_max_entries`, it evicts the
    /// oldest entries until only `low_watermark` remain. A larger gap between
    /// max and low watermark reduces eviction frequency but allows more peak
    /// memory usage. Default: 5 000.
    pub line_layout_cache_low_watermark: usize,

    /// Maximum number of entries in the glyph raster-bounds cache.
    ///
    /// This cache maps `RenderGlyphParams → Bounds<DevicePixels>`. Each entry
    /// is small (~120 bytes), but without a limit the cache can grow unbounded
    /// when many distinct font/size/glyph combinations are used. When the
    /// limit is reached the entire cache is cleared (a simple generational
    /// strategy, since re-computing raster bounds is inexpensive compared to
    /// full text shaping).
    ///
    /// Default: `None` (legacy behavior). Preset profiles may set a finite
    /// limit.
    pub raster_bounds_cache_max_entries: Option<usize>,
}

#[cfg_attr(
    not(any(target_os = "linux", target_os = "freebsd", test)),
    allow(dead_code)
)]
const MIN_INSTANCE_BUFFER_CAPACITY: u64 = 16;

/// GPU resource budget.
#[derive(Clone, Debug)]
pub struct GpuResourceBudget {
    /// Initial width and height (in device pixels) for newly created atlas
    /// textures.
    ///
    /// Atlas textures are allocated lazily when glyphs or images are first
    /// rasterized. This controls the **minimum** size of each texture. If a
    /// single glyph or image is larger, the texture will be sized to fit.
    ///
    /// Default: 1024 (meaning 1024×1024). For minimal applications, 512 is
    /// recommended.
    ///
    /// **Memory impact** (per texture):
    /// - Monochrome (R8): `size × size × 1` bytes
    /// - Polychrome (BGRA): `size × size × 4` bytes
    pub atlas_initial_size: u32,

    /// Initial renderer instance buffer capacity in bytes, where supported.
    ///
    /// Instance buffers hold per-frame draw data uploaded to the GPU. This is an
    /// initial budget, not a hard cap: renderers may grow the buffer on demand
    /// for complex scenes. Lower values reduce idle GPU memory for small apps,
    /// while larger values avoid early reallocations for heavy UIs.
    ///
    /// Default: 2 MiB for desktop applications.
    pub instance_buffer_initial_size: usize,
}

impl GpuResourceBudget {
    /// Returns a renderer-safe instance-buffer capacity for a requested budget.
    ///
    /// Renderers require at least 16 bytes for a dynamic buffer binding, while
    /// a device may impose a smaller upper bound than the requested profile.
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "freebsd", test)),
        allow(dead_code)
    )]
    pub(crate) fn normalize_instance_buffer_capacity(
        instance_buffer_initial_size: usize,
        max_buffer_size: u64,
    ) -> u64 {
        (instance_buffer_initial_size as u64)
            .max(MIN_INSTANCE_BUFFER_CAPACITY)
            .min(max_buffer_size)
    }
}

#[cfg(test)]
mod tests {
    use super::GpuResourceBudget;

    #[test]
    fn instance_buffer_capacity_uses_the_requested_budget_within_device_limits() {
        assert_eq!(
            GpuResourceBudget::normalize_instance_buffer_capacity(768 * 1024, 8 * 1024 * 1024),
            768 * 1024
        );
        assert_eq!(
            GpuResourceBudget::normalize_instance_buffer_capacity(0, 8 * 1024 * 1024),
            16
        );
        assert_eq!(
            GpuResourceBudget::normalize_instance_buffer_capacity(
                16 * 1024 * 1024,
                8 * 1024 * 1024,
            ),
            8 * 1024 * 1024
        );
    }
}
