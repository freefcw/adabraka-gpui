# Linux wgpu Migration

This repository now uses a Zed-style wgpu renderer for Linux X11 and Wayland.
The migration replaces the previous Linux Blade renderer while keeping the public `adabraka-gpui`
application API stable.

## What Changed

- Linux `x11` and `wayland` features select the WGPU renderer through the platform packages.
- Linux and FreeBSD no longer enable `blade-graphics`, `blade-macros`, or `blade-util`.
- Linux and FreeBSD no longer use the `naga` build dependency for shader validation.
- The renderer backend lives in `crates/gpui-wgpu/src/`, with Linux and FreeBSD integration in
  `crates/gpui-linux/src/linux/`.
- X11 and Wayland windows create `WgpuRenderer` instances and share a `GpuContext`.
- Device-lost recovery recreates the wgpu context and forces a fresh render on the next frame.
- The old Blade module and `macos-blade` compatibility feature have been removed.

## Public API

No downstream application entry point changed. Applications should continue to use:

```rust
use gpui::Application;

Application::new().run(|cx| {
    // ...
});
```

The crate split publishes `adabraka-gpui-wgpu` because the published Linux backend depends on it.
Its Rust library name is `gpui_wgpu`, and it exposes the renderer integration types required by the
platform packages. It remains an implementation package rather than the supported application entry
point; downstream applications should normally depend only on `adabraka-gpui`.

## Shader Layout

The Linux wgpu shader is stored at:

```text
crates/gpui-wgpu/src/shaders.wgsl
```

The wgpu shader is consumed by wgpu at runtime through the renderer module.

## Compatibility Notes

- Existing `x11` and `wayland` feature names are unchanged.
- Existing `Application::new()` and `Application::headless()` constructors are unchanged.
- Linux runtime GPU selection now follows wgpu adapter selection. The renderer supports Vulkan and
  OpenGL backends through Zed's wgpu fork.
- The removed `macos-blade` feature is no longer accepted by Cargo.
- The platform crate split does not change the recommended downstream application API.

## Verification

Run the canonical migration and release verification with:

```sh
scripts/verify-migration.sh
```

The script checks the facade and renderer compile baselines, core and compatibility tests, workspace
tests, a normalized macro archive, package inventories for the eight migration packages, formatting,
and diff hygiene. Before a release, install `cargo-semver-checks` and check those eight migration
packages for semantic API compatibility:

```sh
scripts/verify-migration.sh --semver
```

Platform-specific runtime behavior remains covered by native CI jobs and targeted smoke tests.
