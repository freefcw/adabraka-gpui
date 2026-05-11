# Adabraka GPUI

[![Crates.io](https://img.shields.io/crates/v/adabraka-gpui.svg)](https://crates.io/crates/adabraka-gpui)
[![License](https://img.shields.io/crates/l/adabraka-gpui.svg)](LICENSE-APACHE)

A GPU-accelerated UI framework for Rust, forked from [Zed's GPUI](https://github.com/zed-industries/zed). Adabraka GPUI extends the original framework with daemon-mode capabilities, system tray integration, global hotkeys, native notifications, and more — making it suitable for background apps, menu bar utilities, and overlay tools.

## Getting Started

```toml
adabraka-gpui = "0.6.1"
```

To trim image decoder footprint for downstream binaries, disable default features and list only the
platform and image formats you need:

```toml
adabraka-gpui = { version = "0.6.2", default-features = false, features = [
    "font-kit",
    "wayland",
    "x11",
    "image-format-png",
    "image-format-jpeg",
    "image-format-webp",
] }
```

The crate now exposes `image-format-*` features that map directly to `image` crate decoders, plus
`image-rayon` for parallel decoding. SVG rendering remains available separately via `resvg`.

## Platform Support

| Feature | macOS | Linux (X11) | Linux (Wayland) | Windows |
|---|---|---|---|---|
| GPU-accelerated rendering | Metal | Vulkan/OpenGL | Vulkan/OpenGL | DirectX |
| System tray icon & menu | Yes | Yes (DBus/SNI) | Yes (DBus/SNI) | Yes (Shell_NotifyIcon) |
| Tray menu actions | Yes | Yes | Yes | Yes |
| Global hotkeys | Yes | Yes (XGrabKey) | No | Yes (RegisterHotKey) |
| Native notifications | Yes (UNUserNotification) | Yes (notify-rust) | Yes (notify-rust) | Yes (Shell balloon) |
| Overlay windows (always-on-top) | Yes | Yes | Yes (layer-shell when available) | Yes |
| Click-through windows | Yes | Yes (Shape ext) | Yes (wl_region) | Yes (WS_EX_TRANSPARENT) |
| Window show/hide | Yes | Yes | Yes | Yes |
| Auto-launch at login | Yes (SMAppService) | Yes (XDG autostart) | Yes (XDG autostart) | Yes (Registry) |
| Single instance lock | Yes (Unix socket) | Yes (Unix socket) | Yes (Unix socket) | Yes (Named mutex) |
| Focused window info | Yes (Accessibility) | Yes (EWMH) | No | Yes (Win32) |
| Permission queries | Yes (Accessibility, Mic) | No | No | No |
| Daemon mode (no dock icon) | Yes | Yes | Yes | Yes |

### Rendering Backends

Linux X11 and Wayland now use the internal wgpu renderer ported from Zed's current GPUI stack.
The old Blade renderer and its `macos-blade` compatibility feature have been removed. The default
macOS path continues to use Metal and Windows continues to use DirectX.

This migration is internal to the `adabraka-gpui` crate. Downstream applications should keep using
`gpui::Application::new()` and the existing `x11`/`wayland` features. See
[`docs/wgpu-migration.md`](docs/wgpu-migration.md) for implementation notes and verification
status.

### Wayland Layer-Shell Popups

Wayland popup and overlay windows can opt into layer-shell placement with
`WindowOptions::layer_shell`. The backend prefers the stable `ext-layer-shell` protocol and falls
back to `wlr-layer-shell` when the compositor only exposes the older protocol. If neither protocol
is available, GPUI opens a normal `xdg_toplevel` window and logs a warning.

Use `LayerShellOptions::from_window_bounds` when you already have display-relative window bounds, or
`LayerShellOptions::tray_panel` for tray popovers positioned near a `TrayAnchor`.

```rust
use gpui::{
    Bounds, LayerShellOptions, WindowBounds, WindowKind, WindowOptions, point, px, size,
};

let display_bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(1920.0), px(1080.0)));
let popup_bounds = Bounds::new(point(px(1520.0), px(820.0)), size(px(320.0), px(220.0)));

let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(popup_bounds)),
    kind: WindowKind::PopUp,
    layer_shell: Some(LayerShellOptions::from_window_bounds(
        display_bounds,
        popup_bounds,
    )),
    ..WindowOptions::default()
};
```

Automated coverage for this path focuses on protocol-independent behavior: anchor selection,
edge-margin calculation, layer configure sizing, protocol enum mapping, and X11 window hints. Run:

```sh
cargo test -p adabraka-gpui --lib --features test-support layer_shell
cargo test -p adabraka-gpui --lib --features test-support x11::window::tests
cargo check -p adabraka-gpui --example window_positioning --features wayland,x11
```

## Features

### Core UI Framework
- Hybrid immediate/retained mode rendering
- GPU-accelerated with Metal, Vulkan, OpenGL, and DirectX backends
- Tailwind-style layout and styling API
- Entity-based state management
- Declarative views with the `Render` trait
- Low-level `Element` API for custom rendering
- Async executor integrated with the platform event loop
- Action system for keyboard shortcuts
- Test framework with `#[gpui::test]`
- **Resource profiles** — tune cache sizes and GPU allocations for different app types (desktop, utility, minimal)

### Daemon & Background App Support
- **System tray** — icon, tooltip, and nested menus with action callbacks
- **Global hotkeys** — register system-wide keyboard shortcuts
- **Native notifications** — OS-level notifications on all platforms
- **Overlay windows** — always-on-top transparent windows
- **Click-through windows** — mouse events pass through to windows below
- **Window show/hide** — programmatic visibility control
- **Auto-launch** — register your app to start at login
- **Single instance** — prevent multiple copies with activation signaling
- **Keep alive without windows** — app runs with no visible windows
- **Focused window info** — query which window the user is focused on
- **Permission status** — check accessibility and microphone permissions
- **In-app toast notifications** — stackable, auto-dismissing toast component

## Quick Example

```rust
use gpui::{App, Application, TrayMenuItem};

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.set_keep_alive_without_windows(true);
        cx.set_tray_tooltip("My App");

        cx.set_tray_menu(vec![
            TrayMenuItem::Action {
                label: "Settings".into(),
                id: "settings".into(),
            },
            TrayMenuItem::Separator,
            TrayMenuItem::Action {
                label: "Quit".into(),
                id: "quit".into(),
            },
        ]);

        cx.on_tray_menu_action(|id, cx| match id.as_ref() {
            "quit" => cx.quit(),
            _ => {}
        });
    });
}
```

See [`crates/gpui/examples/daemon_app.rs`](crates/gpui/examples/daemon_app.rs) for a full example with overlay windows, settings window, global hotkeys, and notifications.

### Resource Profiles

For lightweight applications (tray icons, status bars, small popups), you can reduce memory usage by selecting an appropriate resource profile:

```rust
use gpui::{Application, AppProfile};

// Minimal profile for tray icons and status bars
Application::new()
    .with_resource_profile(AppProfile::Minimal)
    .run(|cx| {
        // ... your app logic
    });

// Utility profile for settings panels and dialogs
Application::new()
    .with_resource_profile(AppProfile::Utility)
    .run(|cx| {
        // ... your app logic
    });
```

See [`docs/resource-profiles.md`](docs/resource-profiles.md) for detailed guidance on choosing and tuning resource profiles.

## Dependencies

### macOS
- Xcode with macOS components
- Xcode command line tools: `xcode-select --install`

### Linux
- For X11: `libxcb`, `libxkbcommon`
- For Wayland: `libwayland-client`, `libxkbcommon`
- GPU backend: wgpu with Vulkan/OpenGL support
- D-Bus (for system tray via StatusNotifierItem)

### Windows
- Visual Studio Build Tools with C++ workload
- Windows SDK

## License

Apache-2.0
