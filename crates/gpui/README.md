# Adabraka GPUI

A GPU-accelerated UI framework for Rust, forked from [Zed's GPUI](https://github.com/zed-industries/zed). Adabraka GPUI extends the original framework with daemon-mode capabilities, system tray integration, global hotkeys, native notifications, and more — making it suitable for background apps, menu bar utilities, and overlay tools.

## Getting Started

Add the following to your `Cargo.toml`:

```toml
adabraka-gpui = "0.6.0"
```

To keep downstream package size under control, you can disable default features and opt into only
the image formats you need:

```toml
adabraka-gpui = { version = "0.6.0", default-features = false, features = [
    "font-kit",
    "wayland",
    "x11",
    "image-format-png",
    "image-format-jpeg",
    "image-format-webp",
] }
```

`image-format-*` features map to the underlying `image` crate decoder features. `image-rayon`
re-enables parallel decoding. SVG support is handled separately through `resvg`.

### Platform Support

| Feature | macOS | Linux (X11) | Linux (Wayland) | Windows |
|---|---|---|---|---|
| GPU-accelerated rendering | Metal | Vulkan/OpenGL | Vulkan/OpenGL | DirectX |
| System tray icon & menu | Yes | Yes (DBus/SNI) | Yes (DBus/SNI) | Yes (Shell_NotifyIcon) |
| Tray menu actions | Yes | Yes | Yes | Yes |
| Global hotkeys | Yes | Yes (XGrabKey) | No (protocol limitation) | Yes (RegisterHotKey) |
| Native notifications | Yes (UNUserNotification) | Yes (notify-rust) | Yes (notify-rust) | Yes (Shell balloon) |
| Overlay windows (always-on-top) | Yes | Yes | Yes (layer-shell when available) | Yes |
| Click-through windows | Yes | Yes (Shape ext) | Yes (wl_region) | Yes (WS_EX_TRANSPARENT) |
| Window show/hide | Yes | Yes | Yes | Yes |
| Auto-launch at login | Yes (SMAppService) | Yes (XDG autostart) | Yes (XDG autostart) | Yes (Registry) |
| Single instance lock | Yes (Unix socket) | Yes (Unix socket) | Yes (Unix socket) | Yes (Named mutex) |
| Focused window info | Yes (Accessibility) | Yes (EWMH) | No | Yes (Win32) |
| Permission queries | Yes (Accessibility, Mic) | No | No | No |
| Daemon mode (no dock icon) | Yes (Accessory policy) | Yes | Yes | Yes |

### Rendering Backends

Linux X11 and Wayland use the internal wgpu renderer ported from Zed's current GPUI backend. Blade is
no longer part of the crate. The default macOS renderer is Metal and Windows uses DirectX.

This does not introduce a public `gpui_wgpu` crate or change the `Application::new()` entry point.
The renderer protocol stays crate-internal so downstream application APIs remain stable. See
[`../../docs/wgpu-migration.md`](../../docs/wgpu-migration.md) for details.

### Accessibility

Adabraka GPUI exposes AccessKit-backed roles, labels, values, focus, and action handlers on
elements. Native adapters connect the generated accessibility tree to macOS Accessibility,
Linux AT-SPI, and Windows UI Automation.

Native adapters are enabled by the default `accessibility` feature. Disable default features to
exclude their platform dependencies; the core accessibility API remains available but inactive.

```sh
cargo run -p adabraka-gpui --example a11y
```

See [`examples/a11y.rs`](examples/a11y.rs) for focus navigation, accessible actions, numeric
values, switches, and list metadata.

### Wayland Layer-Shell Popups

Set `WindowOptions::layer_shell` to place Wayland popup and overlay windows through layer-shell.
GPUI prefers `ext-layer-shell` and falls back to `wlr-layer-shell`; compositors without either
protocol still receive a normal `xdg_toplevel` window with a warning.

`LayerShellOptions::from_window_bounds` converts display-relative bounds into the anchor and margin
values layer-shell expects. `LayerShellOptions::tray_panel` does the same for a `TrayAnchor`.

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

The layer-shell regression tests avoid requiring a real compositor. They cover anchor selection,
edge margins, configure sizing, protocol enum mapping, and X11 window hints:

```sh
cargo test -p adabraka-gpui --lib --features test-support layer_shell
cargo test -p adabraka-gpui --lib --features test-support x11::window::tests
cargo check -p adabraka-gpui --example window_positioning --features wayland,x11
```

## Features

### Core UI Framework (inherited from GPUI)
- Hybrid immediate/retained mode rendering
- GPU-accelerated with Metal, Vulkan, OpenGL, and DirectX backends
- Tailwind-style layout and styling API
- Entity-based state management
- Declarative views with the `Render` trait
- Low-level `Element` API for custom rendering
- Async executor integrated with the platform event loop
- Action system for keyboard shortcuts
- AccessKit accessibility tree and native platform adapters
- Test framework with `#[gpui::test]`

### Daemon & Background App Support (new in Adabraka)
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

## Dependencies

### macOS

- [Xcode](https://apps.apple.com/us/app/xcode/id497799835?mt=12) with macOS components
- Xcode command line tools: `xcode-select --install`

### Linux

- Standard development packages for your distro
- For X11: `libxcb`, `libxkbcommon`
- For Wayland: `libwayland-client`, `libxkbcommon`
- GPU backend: wgpu with Vulkan/OpenGL support
- D-Bus (for system tray via StatusNotifierItem)

### Windows

- Visual Studio Build Tools with C++ workload
- Windows SDK

## License

Apache-2.0
