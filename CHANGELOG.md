# Changelog

## 0.9.0 (Unreleased)

The 0.9 development line intentionally contains the public API and Cargo feature changes below.
Applications upgrading from 0.8.x should follow the migration mappings in this section.

### Breaking changes

- **`Window::blur` and `Window::disable_focus` now take `&mut App`** — pending
  keystrokes and keybinding-indicator observers are cleared when a window is
  blurred. Call `window.blur(cx)` / `window.disable_focus(cx)` instead of the
  previous no-argument forms.
- **`set_keep_alive_without_windows` replaced by `QuitMode`** — the boolean keep-alive flag is
  gone; use `App::set_quit_mode` / `Application::with_quit_mode` with `QuitMode::Explicit`
  (previous `true`) or `QuitMode::LastWindowClosed` (previous `false`) instead. The quit
  decision is now owned by the core application rather than each platform backend; platforms
  only receive `Platform::set_quit_mode` for platform-visible side effects (macOS
  `ActivationPolicy`). Behavior change on macOS: `QuitMode::Default` now keeps the app alive
  after the last window closes, following the macOS convention — previously the default quit
  on last window close on every platform. Other platforms are unchanged (`Default` still
  quits when the last window closes).
- **Removed pass-through dependency features** — implementation-dependency features that enabled
  no additional code are no longer public: `backtrace`, `rand`, `bitflags`, and `scap` on
  `adabraka-gpui-core`, plus the Linux backend dependency names (`as-raw-xcb-connection`,
  `ashpd`, `bytemuck`,
  `calloop-wayland-source`, `cosmic-text`, `filedescriptor`, `open`, `wayland-backend`,
  `wayland-client`, `wayland-cursor`, `wayland-protocols`, `wayland-protocols-plasma`,
  `wayland-protocols-wlr`, `wayland-scanner`, `x11-clipboard`, `x11rb`, `xim`, and
  `xkbcommon`) across `adabraka-gpui`, `adabraka-gpui-platform`, and `adabraka-gpui-linux`.
  Enable the corresponding semantic features instead: `leak-detection`, `test-support`,
  `wayland`, `x11`, `screen-capture`, or `perf-enabled`. The `wgpu` feature remains a no-op
  marker on `adabraka-gpui` and `adabraka-gpui-platform` for headless compile checks; the old
  `adabraka-gpui-linux/wgpu` feature is removed, and the Linux WGPU backend is enabled by the
  `wayland`/`x11` aggregates.

  | Removed feature | Replacement |
  | --- | --- |
  | `backtrace` | `leak-detection` |
  | `scap` | `screen-capture` |
  | Linux Wayland dependency names | `wayland` |
  | Linux X11 dependency names | `x11` |
  | `rand` on `adabraka-gpui-core` | `test-support` |

### Improvements

- **Spring animations** — `SpringConfig`, `SpringAnimation`, and `AnimationExt::with_spring`
  simulate interruptible physical motion. Overlay, HUD, and daemon surfaces can retarget a
  spring without restarting it. Duration-based easing can overshoot via `sampled_easing`.
- **Animation max FPS** — `Animation::with_max_fps` schedules timer-driven redraws instead of
  requesting a frame on every vsync, useful for looping HUD/daemon decorations.
- **Configurable inactive-window FPS** — `WindowOptions::inactive_frame_interval` controls the
  minimum interval between animation frames while a window is unfocused (`None` disables the
  throttle). The default remains ~30 FPS.
- **`LineLayout::{split_at, paint, paint_background}`** — callers can hold `Arc<LineLayout>`
  and their own decoration runs without allocating a large `ShapedLine`.
- **Windows Restart Manager** — `WM_QUERYENDSESSION` / `WM_ENDSESSION` shut the app down
  cleanly so installers can replace binaries. `Platform::on_quit` now reports whether
  shutdown ran synchronously.

- **Renderer GPU budgets moved off `WindowParams`** — `atlas_initial_size` and
  `instance_buffer_initial_size` are renderer memory policy, not per-window parameters. Platforms
  now receive them through `Platform::configure_gpu_resources` (driven by
  `Application::with_resource_profile`) and supply them to renderers at window creation, so
  `WindowParams` no longer carries renderer resource policy.
- **Feature contract checks** — `scripts/verify-features.sh` compiles the public feature
  combinations used by CI and the docs runbooks, and pins every published crate's public
  feature list against a whitelist so accidental additions or removals fail in CI.
- **Async benchmark feature** — `adabraka-gpui-core` exposes the `bench` feature used to compile
  the production-like async task completion benchmark without enabling it for normal builds.

## Utility crates 0.6.0 (Unreleased)

### Breaking changes

- **`adabraka_util` feature cleanup** — the dependency-name features `git2`, `rand`, and
  `util_macros` are removed. Use the semantic `test-support` feature when the test helpers are
  required.
- **`adabraka_util_macros` feature cleanup** — the dependency-name feature `perf` is removed. Use
  `perf-enabled` instead.

## 0.8.1 (2026-07-28)

### Fixes

- **Windows test compilation** — avoids importing the crate-wide `test` name into the draw
  coordinator unit-test module, removing an ambiguous `#[test]` attribute on Windows.
- **Windows visual snapshots** — aligns the DirectX HLSL `Quad` layout with the Rust scene struct's
  explicit transform padding so offscreen readback renders scene content instead of a fully
  transparent image.

## 0.8.0 (2026-07-28)

### Breaking changes

- **Permission capability reporting** — `PermissionStatus` now includes `Unavailable`, and
  accessibility and microphone permission requests return `PermissionRequestStatus`. Unsupported
  platforms no longer report permissions as granted.
- **Default image decoder set** — default builds now enable GIF, JPEG, PNG, and WebP. AVIF, BMP,
  DDS, EXR, farbfeld, HDR, ICO, PNM, QOI, TGA, and TIFF remain available through their individual
  `image-format-*` features but must be enabled explicitly.

### Features

- **Screen-capture lifecycle** — callers can opt into exactly-once `Ended`, `Cancelled`, or `Failed`
  notifications for streams that start successfully. Existing `ScreenCaptureSource::stream`
  implementations remain source-compatible.
- **Accessibility contracts and testing** — elements can expose form-control properties, ARIA
  descriptions, and keyboard shortcuts; test platforms can activate accessibility, dispatch
  assistive actions, and inspect a stable debug tree.
- **Cross-platform visual artifacts** — Linux WGPU and Windows DirectX backends can render scenes to
  images for visual tests, with native window sizing and offscreen Windows rendering handled by the
  test platform.
- **Window and image behavior** — applications can request attention for a specific window, update
  layer-shell exclusive zones at runtime, honor EXIF orientation for static images, and size
  auto-dimensioned window roots from the platform-provided viewport.

### Fixes

- **GPUI macro dependency resolution** — aligns `adabraka-gpui-macros` with the `0.8` release line,
  resolves renamed facade/core dependencies, and lets mixed direct dependency graphs select the
  intended crate through `[package.metadata.gpui-macros]`.
- **macOS screen capture** — releases native setup, startup-failure, and frame-callback resources
  so repeated screen-sharing attempts do not accumulate them.
- **Wayland clipboard/selection serial** — selection ownership now uses a dedicated press-derived
  serial instead of the largest serial across all kinds, preventing IME serials from poisoning
  clipboard ownership on Mutter/kWin after prolonged input-method use.
- **Rendering and event safety** — fixes truncated-text measurement reuse, nested deferred draws,
  reentrant Windows draws, arena clearing during draws, prompt click-through, and Windows text and
  atlas edge cases.
- **Platform lifecycle hardening** — retains delayed Linux tasks during shutdown, enables the
  Windows security APIs required by single-instance support, and deduplicates Wayland IME cursor
  commits.

### Improvements

- **Internal crate split** — separates core, platform selection, Linux, macOS, Windows, and WGPU
  implementations while preserving `adabraka-gpui` as the downstream compatibility facade.
- **Linux/WGPU resource profiles** — new windows now apply
  `GpuResourceBudget::instance_buffer_initial_size`, including after GPU device recovery, while
  clamping the allocation to renderer and device limits.
- **Rendering and scheduling efficiency** — reduces border-only quad overdraw, shares macOS display
  links per display, uses the Win32 thread pool for Windows tasks, and upgrades Taffy to `0.12.2`.
- **Release verification** — the canonical migration script verifies all eight package archives and
  runs semantic API comparison for the published public facade with a registry library baseline.

## 0.7.0 (2026-07-11)

### Breaking changes

- **Layer-shell API (`0.7.0`)** — layer-shell windows now use
  `WindowOptions.kind = WindowKind::LayerShell(LayerShellOptions { ... })`. This removes
  `WindowOptions::layer_shell`, `LayerShellProtocolPreference`,
  `LayerShellOptions::tray_panel`, and `LayerShellOptions::from_window_bounds`.
  `WindowKind::Overlay` no longer implicitly creates a layer-shell surface. The Wayland backend
  now requires `wlr-layer-shell`, while X11 and unsupported compositors return
  `LayerShellNotSupportedError`. See the
  [migration and implementation provenance guide](docs/layer-shell-migration.md).

### Features

- **App resource profiles** — new `AppResourceProfile` / `AppProfile` system lets
  callers tune internal cache sizes and GPU atlas allocation at startup via
  `Application::new().with_resource_profile(AppProfile::Minimal)`. Three presets
  are provided (`Desktop`, `Utility`, `Minimal`) plus a `Custom` variant for
  fine-grained control. The global line-layout cache now uses a configurable
  watermark-based eviction strategy instead of the previous fixed 50%-drop
  approach, which is significantly smoother for small caches.

### Fixes

- **Linux/WGPU rendering** — aligned the WGPU shader's `Quad`/`Background` layout with the Rust `repr(C)` scene structs so that borders, rounded corners, gradients, separators, and toggle tracks no longer drop intermittently on Linux (`fix(wgpu): align Quad/Background ABI with Rust scene struct`).
- **Multi-stop gradients** — `interpolate_multi_stop` now clamps `stop_count` to `[2, 4]` and protects every divisor with `max(p_high - p_low, 1e-6)`, so degenerate or collapsed stops no longer produce NaN/Inf that blank out a quad. Mirrored across WGSL, Metal, and HLSL.

### Behavior changes

- **Linux/WGPU sRGB gradients** — gradients with `ColorSpace::Srgb` now mix in the same color space as macOS/Windows. The previous WGPU path applied an extra `linear_to_srgba` / `srgba_to_linear` round-trip in vertex/fragment, producing visibly different gradients on Linux. After this change Linux gradients look slightly brighter and more saturated at the midpoint, matching macOS and Windows. Oklab gradients are unchanged.

## 0.6.2 (2026-04-30)

### Improvements

- **Cursor behavior** — restore cursor to Arrow style when window loses focus

### Documentation

- Added batch 2 evaluation report documenting architectural constraints
- Added immediate actions completion report

## 0.6.1 (2026-04-30)

### Fixes

- **Anchored element positioning** — fixed size calculation with negative coordinates that caused context menus and popups to appear at wrong locations (synced from Zed `b38194198b`)
- **GIF rendering stability** — fixed out-of-bounds panic when replacing a GIF with one that has fewer frames (synced from Zed `749fcfdfd8`)

### Documentation

- Added comprehensive Zed sync documentation in `docs/sync/`:
  - Complete mapping between Zed and Adabraka GPUI repositories
  - Technical sync guide with code-level instructions
  - Quick reference for daily sync operations
  - Analysis of 100+ mergeable updates from Zed (2024-2026)

## 0.5.1 (2026-02-17)

### Performance

- **DirectX pipeline state caching** — skip redundant `set_pipeline_state` calls when consecutive batches use the same pipeline, saving ~6 D3D11 API calls per batch in text-heavy UIs
- **Cross-window text layout cache** — global LRU cache on `TextSystem` prevents re-shaping text that another window already shaped

## 0.5.0 (2026-02-15)

### Desktop Platform Features

Added 15 new cross-platform capabilities to the `Platform` trait, with implementations for macOS, Windows, and Linux:

**System Integration**
- **System power events** — subscribe to suspend/resume/lock/unlock/shutdown notifications (macOS: `NSWorkspace`, Windows: `WM_POWERBROADCAST`, Linux: stub for D-Bus logind)
- **Power save blocker** — prevent display sleep or app suspension (macOS: `IOPMAssertionCreateWithName`, Windows: `SetThreadExecutionState`, Linux: `dbus-send` screensaver inhibit + `systemd-inhibit`)
- **System idle time** — query time since last user input (macOS: `CGEventSourceSecondsSinceLastEventType`, Windows: `GetLastInputInfo`, Linux: X11 screensaver extension)
- **Network status** — query online/offline connectivity (macOS: `NWPathMonitor`, Windows: `INetworkListManager`, Linux: `/sys/class/net/*/operstate`)
- **OS info** — query OS name, version, arch, locale, hostname
- **Biometric authentication** — Touch ID (macOS), Windows Hello, with availability detection

**Window Management**
- **User attention** — request/cancel taskbar attention (macOS: dock bounce, Windows: `FlashWindowEx`, Linux: X11 EWMH `_NET_WM_STATE_DEMANDS_ATTENTION`)
- **Progress bar** — set taskbar/dock progress state (macOS: `NSDockTile`, Windows: `ITaskbarList3`)
- **Window state save/restore** — `WindowState` struct for persisting window bounds, display, and fullscreen state
- **Window positioner** — `WindowPosition` enum for semantic positioning (center, tray-relative, corners)

**UI & Input**
- **Native dialogs** — modal alert/confirm dialogs with customizable buttons (macOS: `NSAlert`, Windows: `TaskDialogIndirect`, Linux: `zenity`/`kdialog`)
- **Context menus** — show native context menus at a position (macOS: `NSMenu`, Windows: `TrackPopupMenu`)
- **Media keys** — intercept play/pause/stop/next/previous hardware keys (macOS: `MPRemoteCommandCenter`, Windows: `WM_APPCOMMAND`, Linux: XF86 keysym interception)
- **Dock badge** — set dock icon badge label (macOS only)

**App API**
- New `App` and `Window` convenience methods for all platform features
- `app.os_info()`, `app.network_status()`, `app.start_power_save_blocker()`, `app.show_dialog()`, etc.
- `window.set_progress_bar()`, `window.request_user_attention()`, etc.

### Improvements

- **Tray panel mode** — position windows relative to the tray icon
- **Menu icons** — support icon data on tray menu items
- **Global hotkey normalization** — consistent key string handling across platforms
- **Platform features demo** — new `platform_features_demo` example exercising all new APIs

### Fixes

- Pin `core-text` to `=21.0.0` to prevent `core-graphics` version conflict on macOS
- Resolve all clippy warnings across the codebase
- Fix safety and correctness issues in platform feature implementations

## 0.4.1 (2026-02-14)

- Documentation and README updates for crates.io

## 0.4.0 (2026-02-14)

### Initial Murmur Extensions

- System tray with icon, tooltip, and menu
- Global hotkeys with platform-native registration
- Overlay windows (always-on-top, click-through)
- Window show/hide toggling
- Active window and focused window info queries
- Accessibility and microphone permission checks (macOS)
- Auto-launch at login
- Single-instance enforcement
- Desktop notifications
- Toast component for in-app notifications
- Keep-alive-without-windows daemon mode
- Element transforms (rotate, scale, transform-origin)
- Multi-stop, radial, and conic gradients
- Per-element blend modes
