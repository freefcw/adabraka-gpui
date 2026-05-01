# Linux wgpu Migration

This repository now uses a Zed-style wgpu renderer for Linux X11 and Wayland.
The migration replaces the previous Linux Blade renderer while keeping the public `adabraka-gpui`
application API stable.

## What Changed

- Linux `x11` and `wayland` features enable the optional `wgpu` dependency.
- Linux and FreeBSD no longer enable `blade-graphics`, `blade-macros`, or `blade-util`.
- Linux and FreeBSD no longer use the `naga` build dependency for shader validation.
- The wgpu backend lives inside `crates/gpui/src/platform/wgpu/`.
- X11 and Wayland windows create `WgpuRenderer` instances and share a `GpuContext`.
- Device-lost recovery recreates the wgpu context and forces a fresh render on the next frame.
- The old Blade module remains compiled only for `macos-blade`.

## Public API

No downstream application entry point changed. Applications should continue to use:

```rust
use gpui::Application;

Application::new().run(|cx| {
    // ...
});
```

The migration deliberately does not expose a public `gpui_wgpu` crate yet. A standalone backend
crate would require the broader upstream-style split into platform crates and would make renderer
protocol types such as `Scene`, `PlatformAtlas`, and atlas keys part of the public surface. In this
repository those types remain crate-internal to avoid expanding the downstream API.

## Shader Layout

The Linux wgpu shader is stored at:

```text
crates/gpui/src/platform/wgpu/shaders.wgsl
```

`crates/gpui/build.rs` only validates Blade WGSL when `macos-blade` is enabled on macOS. The wgpu
shader is consumed by wgpu at runtime through the renderer module.

## Compatibility Notes

- Existing `x11` and `wayland` feature names are unchanged.
- Existing `Application::new()` and `Application::headless()` constructors are unchanged.
- Linux runtime GPU selection now follows wgpu adapter selection. The renderer supports Vulkan and
  OpenGL backends through Zed's wgpu fork.
- `macos-blade` remains available for macOS only. It is not used for Linux rendering.
- A future full upstream-style crate split should be treated as a separate public API change.

## Verification

The migration has been checked with:

```sh
cargo fmt
cargo check -p adabraka-gpui --features wayland,x11
cargo check -p adabraka-gpui --no-default-features --features macos-blade
cargo check -p adabraka-gpui --lib --tests --features test-support
cargo doc -p adabraka-gpui --no-deps --features wayland,x11
```

Linux cross-compilation from the current macOS machine is blocked before GPUI code is compiled
because the host does not have `x86_64-linux-gnu-gcc` and `x86_64-linux-gnu-g++`. The failure occurs
in C/C++ build scripts for dependencies such as `ring` and `freetype-sys`.
