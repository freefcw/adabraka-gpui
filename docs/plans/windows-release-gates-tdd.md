# Hai TDD: Windows Release Gates

## Target Behavior 1

Windows workspace tests compile and run the draw coordinator unit test without an ambiguous test attribute.

## RED

- **Test added**: Existing `draw_coordinator_defers_reentrant_draws` regression test in `crates/gpui-windows/src/windows/events.rs`.
- **Behavior asserted**: The Windows backend's unit-test target must compile and execute the draw coordinator test.
- **Command**: GitHub Actions run `30327564415`, job `Windows / workspace tests`, running `cargo test --workspace --lib --tests --locked -- --test-threads=1`.
- **Observed failure**: Rust error `E0659` at `events.rs:1756`: `test` is ambiguous after `use super::*` imports the crate-wide `test` name.
- **Failure is correct because**: The existing regression test could not enter execution on Windows; the failure identifies the test-module import rather than an environment or dependency problem.

## GREEN

- **Minimal implementation**: Replace `use super::*` with `use super::DrawCoordinator` in the test module.
- **Command**: GitHub Actions run `30328204352`, job `Windows / workspace tests`, using the same workspace test command.
- **Observed pass**: Job completed successfully in 10m37s, including public example checks.

## REFACTOR

- **Refactor done**: no
- **Change**: No refactor needed; the explicit import is the smallest and clearest fix.
- **Command after refactor**: `cargo fmt --all -- --check` and the green GitHub Actions workspace job.
- **Observed result**: Formatting and Windows workspace tests passed.

## Next Behavior

DirectX visual readback must contain the rendered scene.

## Target Behavior 2

The Windows real visual smoke test returns a 64x64 image containing opaque red scene content and transparent background pixels.

## RED

- **Test added**: Existing `real_visual_smoke` integration test in `crates/gpui-compat/tests/real_visual_smoke.rs`.
- **Behavior asserted**: Rendering a red quad through the DirectX offscreen path must produce nontransparent, nonuniform image pixels.
- **Command**: GitHub Actions run `30327564415`, job `Windows / real visual smoke`, running `cargo test -p fc-gpui --test real_visual_smoke --features test-support --locked -- --ignored`.
- **Observed failure**: The scene contained a quad, but readback was fully transparent: `channel_min=[0, 0, 0, 0]`, `channel_max=[0, 0, 0, 0]`, and `nonzero_rgb_pixels=0`.
- **Failure is correct because**: Texture creation, dimensions, mapping, and scene construction succeeded; only rasterized scene content was absent from readback, isolating the DirectX scene/shader contract.

## GREEN

- **Minimal implementation**: Add the Rust `Quad` struct's `_pad_before_transform` and `_pad_end` fields to the HLSL `Quad` declaration so DirectX reads transform and blend fields from the correct offsets.
- **Command**: GitHub Actions run `30328204352`, job `Windows / real visual smoke`, using the same ignored integration-test command.
- **Observed pass**: Job completed successfully in 6m9s; the screenshot contained the expected rendered content.

## REFACTOR

- **Refactor done**: no
- **Change**: No refactor needed; the shader declaration now directly mirrors the existing Rust ABI.
- **Command after refactor**: `cargo check -p fc-gpui-windows --all-features`, `cargo fmt --all -- --check`, and the green GitHub Actions visual job.
- **Observed result**: Local checks passed and the full nine-job Linux, macOS, and Windows feature matrix completed successfully.

## Next Behavior

Done. The fixes are ready for the `0.8.1` patch release.
