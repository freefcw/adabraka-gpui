# Resource Profiles Guide

GPUI applications have vastly different resource needs. A full IDE benefits from large caches and GPU textures, while a tray icon or small popover can run with significantly less memory. Resource profiles allow you to tune internal cache sizes and GPU allocations at application startup.

## Quick Start

```rust
use gpui::{Application, AppProfile};

// For a minimal tray icon app
Application::new()
    .with_resource_profile(AppProfile::Minimal)
    .run(|cx| {
        // ... your app logic
    });

// For a settings panel or dialog
Application::new()
    .with_resource_profile(AppProfile::Utility)
    .run(|cx| {
        // ... your app logic
    });

// For a full desktop application (default)
Application::new()
    .with_resource_profile(AppProfile::Desktop)
    .run(|cx| {
        // ... your app logic
    });
```

## Preset Profiles

### Desktop Profile (Default)

Designed for full desktop applications like IDEs, editors, or browsers.

**Characteristics:**
- Large text layout caches (10,000 entries)
- Standard atlas textures (1024×1024)
- 2 MiB initial instance buffer
- 1 MiB element arena
- Unlimited raster bounds cache

**Memory Impact:**
- Line layout cache: up to ~12 MB per cache (unwrapped + wrapped)
- Atlas textures: 4 MB per texture (BGRA) or 1 MB (monochrome)
- Element arena: 1 MiB per thread (cleared each frame)
- Raster bounds cache: unbounded (grows with font/size/glyph diversity)

**Use When:**
- Your app displays large amounts of text
- Users interact with complex UIs (editors, browsers, dashboards)
- Memory usage is not a primary concern

### Utility Profile

Designed for lightweight utility windows like settings panels, dialog boxes, or tool palettes.

**Characteristics:**
- Moderate text layout caches (3,000 entries)
- Standard atlas textures (1024×1024)
- 1 MiB initial instance buffer
- 512 KiB element arena
- Raster bounds cache limited to 5,000 entries

**Memory Impact:**
- Line layout cache: up to ~3.6 MB per cache
- Atlas textures: 4 MB per texture (BGRA) or 1 MB (monochrome)
- Element arena: 512 KiB per thread
- Raster bounds cache: ~600 KB (5,000 × 120 bytes)

**Use When:**
- Your app has small to medium text display needs
- UI is relatively simple (forms, lists, small editors)
- You want to reduce memory footprint without sacrificing too much performance

### Minimal Profile

Designed for minimal resource usage like tray icons, status bars, or notification popups.

**Characteristics:**
- Small text layout caches (500 entries)
- Smaller atlas textures (512×512)
- 512 KiB initial instance buffer
- 256 KiB element arena
- Raster bounds cache limited to 2,000 entries

**Memory Impact:**
- Line layout cache: up to ~600 KB per cache
- Atlas textures: 1 MB per texture (BGRA) or 256 KB (monochrome)
- Element arena: 256 KiB per thread
- Raster bounds cache: ~240 KB (2,000 × 120 bytes)

**Use When:**
- Your app displays very little text
- UI is minimal (icons, tooltips, small labels)
- Memory efficiency is critical (long-running background processes)

## Configuration Options

### Text Layout Cache

Controls the global line-layout cache size. This cache stores shaped text lines to avoid reshaping the same text repeatedly.

**Fields:**
- `line_layout_cache_max_entries`: Maximum entries before eviction
- `line_layout_cache_low_watermark`: Target entries after eviction

**How It Works:**
- Cache grows on demand until reaching `max_entries`
- When full, oldest entries are evicted down to `low_watermark`
- Two separate caches: one for unwrapped lines, one for wrapped lines

**Memory Calculation:**
- Each entry: ~1.2 KB (varies with text length and font complexity)
- Peak memory: `max_entries × 1.2 KB × 2 caches`

**Tuning Guidelines:**
- Larger caches reduce CPU work for repeated text display
- Smaller caches reduce memory but increase reshaping overhead
- Watermark gap (max - low) trades eviction frequency vs peak memory

### Atlas Initial Size

Controls the initial dimensions of GPU textures allocated for the glyph/image atlas.

**Field:**
- `atlas_initial_size`: Width and height in device pixels (square texture)

**How It Works:**
- Atlas textures are allocated lazily when glyphs/images are first rasterized
- This is the **minimum** size; textures grow if needed for large glyphs
- Each renderer (window) has its own atlas

**Memory Calculation:**
- BGRA (color): `size × size × 4` bytes
- Monochrome (R8): `size × size × 1` bytes

**Tuning Guidelines:**
- 1024×1024: Standard, supports most fonts and emoji
- 512×512: Halves GPU memory, suitable for limited glyph sets
- Smaller sizes may cause texture growth if large glyphs are used

### Instance Buffer Initial Size

Controls the initial capacity of each renderer's per-frame instance buffer.

**Field:**
- `instance_buffer_initial_size`: Initial capacity in bytes

**How It Works:**
- The configured value is applied when a WGPU renderer is created.
- A renderer may grow beyond this value when a scene requires more space.
- Values below the renderer minimum are raised to 16 bytes; values above the device limit are clamped.
- The WGPU backend applies this setting on Linux. Other backends may use their own support path.

**Tuning Guidelines:**
- 2 MiB: default for desktop applications
- 1 MiB: suitable for utility windows
- 512 KiB: suitable for minimal applications
- Increase it when complex scenes frequently trigger early buffer growth.

### Element Arena Size

Controls the initial capacity of the per-thread element arena (bump allocator).

**Field:**
- `element_arena_size`: Initial capacity in bytes

**How It Works:**
- Arena is used during each frame's layout and paint phases
- Cleared after every frame
- Initial size only affects pre-allocation, not final memory usage

**Memory Calculation:**
- Per-thread allocation (cleared each frame)
- Peak usage depends on scene complexity

**Tuning Guidelines:**
- Larger sizes reduce mid-frame reallocations
- Smaller sizes reduce baseline memory
- For simple UIs, 256-512 KiB is often sufficient

### Raster Bounds Cache

Controls the glyph raster-bounds cache size.

**Field:**
- `raster_bounds_cache_max_entries`: Maximum entries (or `None` for unlimited)

**How It Works:**
- Maps `RenderGlyphParams → Bounds<DevicePixels>`
- Each entry: ~120 bytes
- When limit reached, entire cache is cleared (generational strategy)

**Tuning Guidelines:**
- `None`: Legacy behavior, unbounded growth
- Limited: Prevents unbounded memory with diverse font/size/glyph usage
- Raster bounds are cheap to recompute, so cache misses are acceptable

## Custom Profiles

For fine-grained control, use the `Custom` variant:

```rust
use gpui::{Application, AppProfile, AppResourceProfile, TextResourceBudget, GpuResourceBudget};

Application::new()
    .with_resource_profile(AppProfile::Custom(AppResourceProfile {
        text: TextResourceBudget {
            line_layout_cache_max_entries: 2_000,
            line_layout_cache_low_watermark: 1_500,
            raster_bounds_cache_max_entries: Some(3_000),
        },
        gpu: GpuResourceBudget {
            atlas_initial_size: 768,  // Non-standard size
            instance_buffer_initial_size: 768 * 1024,
        },
        element_arena_size: 384 * 1024,  // 384 KiB
    }))
    .run(|cx| {
        // ... your app logic
    });
```

## Choosing the Right Profile

### Decision Tree

```
Does your app display large amounts of text?
├─ Yes → Desktop profile
└─ No → Is it a long-running background process?
    ├─ Yes → Minimal profile
    └─ No → Utility profile
```

### Common Scenarios

| Application Type | Recommended Profile | Reason |
|---|---|---|
| IDE / Code Editor | Desktop | Heavy text display, complex UI |
| Browser | Desktop | Heavy text, diverse content |
| Dashboard / Analytics | Desktop | Moderate text, complex visualizations |
| Settings Panel | Utility | Moderate text, simple UI |
| Dialog Box | Utility | Limited text, simple UI |
| Tool Palette | Utility | Limited text, simple UI |
| Tray Icon | Minimal | Very limited text, background process |
| Status Bar | Minimal | Very limited text, background process |
| Notification Popup | Minimal | Very limited text, ephemeral |
| Menu Bar App | Minimal | Limited text, background process |

## Performance Considerations

### CPU vs Memory Tradeoff

Resource profiles primarily trade CPU work for memory:

- **Larger caches**: Less CPU reshaping, more memory
- **Smaller caches**: More CPU reshaping, less memory

For most applications, the CPU impact is negligible unless you're:
- Displaying thousands of lines of text per frame
- Using many different fonts/sizes
- Running on very low-end hardware

### GPU Memory

Atlas textures consume GPU memory, which is often more constrained than system memory:

- Each window has its own atlas
- Multiple windows multiply GPU memory usage
- Consider `Minimal` profile for multi-window apps

### Thread-local Arenas

The element arena is allocated per thread:

- Multi-threaded rendering multiplies arena usage
- Arena is cleared each frame, so peak memory matters more than average

## Migration Guide

### From Default (No Profile)

If you previously used `Application::new()` without a profile:

```rust
// Before (default Desktop behavior)
Application::new().run(|cx| { /* ... */ });

// After (explicit Desktop)
Application::new()
    .with_resource_profile(AppProfile::Desktop)
    .run(|cx| { /* ... */ });
```

Behavior is identical; the explicit form just makes your intent clear.

### Reducing Memory Footprint

To reduce memory for an existing app:

1. Start with `Utility` profile
2. Test for performance regressions
3. If acceptable, try `Minimal` profile
4. If not acceptable, use `Custom` to tune specific settings

## Troubleshooting

### High Memory Usage

If your app uses more memory than expected:

1. Check if you're using `Desktop` profile when `Utility` or `Minimal` would suffice
2. Monitor line layout cache growth with `line_layout_cache_max_entries`
3. Consider limiting `raster_bounds_cache_max_entries` if you use many fonts/sizes
4. Reduce `element_arena_size` if you have many render threads

### Performance Degradation

If your app is slow after switching profiles:

1. Increase `line_layout_cache_max_entries` to reduce reshaping
2. Increase `line_layout_cache_low_watermark` to reduce eviction frequency
3. Increase `atlas_initial_size` to reduce texture reallocation
4. Increase `element_arena_size` to reduce mid-frame reallocations

### Texture Allocation Failures

If you see GPU texture allocation errors:

1. Increase `atlas_initial_size` to match your largest glyphs/images
2. Reduce the number of windows (each has its own atlas)
3. Use monochrome rendering if color is not required

## API Reference

For complete API documentation, see:

- [`gpui::AppProfile`](../crates/gpui/src/resource_profile.rs) - Profile enum
- [`gpui::AppResourceProfile`](../crates/gpui/src/resource_profile.rs) - Configuration struct
- [`gpui::TextResourceBudget`](../crates/gpui/src/resource_profile.rs) - Text cache settings
- [`gpui::GpuResourceBudget`](../crates/gpui/src/resource_profile.rs) - GPU resource settings
- [`Application::with_resource_profile`](../crates/gpui/src/app.rs) - Builder method
