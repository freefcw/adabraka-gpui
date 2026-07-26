# Adabraka GPUI WGPU

Internal WGPU renderer crate for [Adabraka GPUI](https://crates.io/crates/adabraka-gpui).

This package owns the WGPU context, atlas, surface renderer, shader, resource-budget normalization, and device-loss recovery used by the Linux and FreeBSD backends. Applications should normally depend on `adabraka-gpui` rather than this crate directly.

Licensed under Apache-2.0.
