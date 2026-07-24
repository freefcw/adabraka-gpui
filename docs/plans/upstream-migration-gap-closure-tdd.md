# Hai TDD: Upstream migration gap closure

## Target Behavior

Upgrade the shared layout contract, implement cross-platform renderer readback, make native visual validation executable, and complete the bounded GPUI accessibility increment with direct behavior tests.

## RED 1 — Taffy 0.12.2 API contract

- **Test added**: dependency-only upgrade in `crates/gpui/Cargo.toml`
- **Behavior asserted**: GPUI must compile against Taffy 0.12.2 rather than silently retaining the old API.
- **Command**: `cargo check -p adabraka-gpui --lib`
- **Observed failure**: 16 errors for removed `AlignItems::{Start,...}` / `AlignContent::{Start,...}` associated items.
- **Failure is correct because**: Taffy 0.12 changed alignment values to safe-alignment structs exposed through uppercase associated constants; the failure was the expected dependency contract break.

## GREEN 1

- **Minimal implementation**: migrated alignment conversions to uppercase constants, ported the upstream max-content list/uniform-list probes, recorded truncation constraints in `TextLayoutInner`, and prevented truncated layouts from answering unconstrained intrinsic probes.
- **Command**: `cargo check -p adabraka-gpui --lib && cargo test -p adabraka-gpui --lib --features test-support -- elements::list elements::text elements::uniform_list --test-threads=1`
- **Observed pass**: compilation succeeded; 19 focused layout tests passed.

## REFACTOR 1

- **Refactor done**: yes
- **Change**: added explicit `f32` suffixes required by Taffy's generic grid helpers, eliminating future-incompatible literal fallback warnings.
- **Command after refactor**: Linux full feature check and `cargo tree -p adabraka-gpui`
- **Observed result**: Linux check passed; Taffy resolved to exactly 0.12.2 locally and on `mp-dev`.

## RED 2 — GPU row-pitch and channel conversion

- **Test added**: `platform::render_image::{converts_bgra_pixels_to_rgba, removes_gpu_row_padding, rejects_short_readback_buffers}`
- **Behavior asserted**: GPU staging data must remove row padding, convert BGRA to RGBA, and reject truncated buffers.
- **Command**: `cargo test -p adabraka-gpui --lib --features test-support -- platform::render_image --test-threads=1`
- **Observed failure**: all three tests failed with `BGRA readback conversion is not implemented`.
- **Failure is correct because**: the shared conversion seam existed but intentionally returned an error before the implementation.

## GREEN 2

- **Minimal implementation**: added checked row-size/buffer-size validation, compacted padded rows, converted BGRA channels, and preserved RGBA surfaces without swapping.
- **Command**: `cargo test -p adabraka-gpui --lib --features test-support -- platform::render_image --test-threads=1`
- **Observed pass**: 4 conversion tests passed.

## REFACTOR 2

- **Refactor done**: yes
- **Change**: extracted WGPU scene encoding into `prepare_scene_render` and `render_scene_to_view`; extracted DirectX rendering into `render_scene`; both normal present and test readback now share their production rendering pipelines.
- **Command after refactor**: local GPUI suite, Linux feature suite, Windows GNU cross-check, and X11 real visual smoke.
- **Observed result**: local 197 passed/2 ignored; Linux 219 passed; Windows target check passed; X11 screenshot smoke exited 0.

## RED 3 — Linux real visual context

- **Test added**: enabled `real_visual_smoke` for Linux.
- **Behavior asserted**: Linux should compile and run the same native renderer/readback smoke contract as macOS.
- **Command**: `cargo check -p adabraka-gpui --test real_visual_smoke --features test-support,x11` on `mp-dev`
- **Observed failure**: unresolved import `gpui::RealVisualTestContext` because the context and proxy were macOS-gated.
- **Failure is correct because**: Linux had WGPU windows but no exported real visual context.

## GREEN 3

- **Minimal implementation**: enabled `VisualTestPlatform` and `RealVisualTestContext` on Linux/FreeBSD, added Linux primary-selection forwarding, used screen-outside positions on X11 and compositor-selected positions on Wayland, and connected Wayland/X11 windows to WGPU readback.
- **Command**: `WGPU_BACKEND=vulkan xvfb-run -a -s '-screen 0 1024x768x24' cargo test -p adabraka-gpui --test real_visual_smoke --features test-support,x11 -- --ignored`
- **Observed pass**: native X11 window render, request-attention call, WGPU staging-buffer readback, size/content assertions, and process exit all passed (`STATUS:0`).

## REFACTOR 3

- **Refactor done**: yes
- **Change**: made capability detection distinguish real renderer, screenshot support, and absolute-position support so Wayland does not claim X11 positioning semantics.
- **Command after refactor**: combined Linux suite with `test-support,wayland,x11`.
- **Observed result**: 219 tests passed.

## RED 4 — Windows cross-target baseline

- **Test added**: Windows GNU cross-target compilation on `mp-dev`.
- **Behavior asserted**: DirectX readback must at least compile for a Windows target when runtime hardware is unavailable.
- **Command**: `cargo check -p adabraka-gpui --lib --features test-support --target x86_64-pc-windows-gnu`
- **Observed failure**: existing Direct Manipulation code resolved `borrow_mut` against `Rc` instead of the inner `RefCell`, and `ScreenToClient` was not imported from GDI.
- **Failure is correct because**: these were target-specific compile blockers that prevented reaching the new DirectX readback code.

## GREEN 4

- **Minimal implementation**: explicitly borrowed through `Rc::as_ref()` and imported the GDI API; implemented DirectX staging-texture copy/map/row-pitch conversion and wired `PlatformWindow::render_to_image`.
- **Command**: same Windows GNU cross-target check.
- **Observed pass**: full GPUI Windows-target check passed; remaining output contained warnings only.

## REFACTOR 4

- **Refactor done**: yes
- **Change**: ensured staging resources are unmapped before returning conversion errors and reused the shared checked row conversion.
- **Command after refactor**: Windows GNU cross-target check and `cargo fmt --all -- --check`.
- **Observed result**: passed.

## RED 5 — Coupled Taffy behavior contracts

- **Tests added**: `optional_width_uses_max_content_when_unbounded`, `truncated_layout_is_not_reused_for_an_unconstrained_measurement`, and `exact_fit_shaped_text_does_not_require_truncation`.
- **Behavior asserted**: an unbounded list width uses `MaxContent`; a size measured under truncation cannot answer a later unconstrained probe; an exact shaped width is considered a fit.
- **Command**: `cargo test -p adabraka-gpui --lib --features test-support -- elements::text::tests taffy::tests --test-threads=1`
- **Observed failure**: 5 compile errors because the three semantic helpers did not exist.
- **Failure is correct because**: the migrated behavior was embedded in closures and duplicate call sites without a directly testable contract.

## GREEN 5

- **Minimal implementation**: extracted `available_space_for_optional_width`, `cached_size_for_constraints`, and `all_shaped_line_widths_fit`, then used them in the existing list/text paths.
- **Observed pass**: 3 direct regression tests passed.

## RED 6 — ARIA description and keyboard shortcut metadata

- **Test added**: `aria_description_and_keyshortcuts_are_written_to_accesskit`.
- **Behavior asserted**: the fluent element API must write AccessKit `description` and `keyboard_shortcut` properties.
- **Observed failure**: compilation failed because `aria_description` did not exist.
- **Failure is correct because**: the bounded upstream accessibility API was absent.

## GREEN 6

- **Minimal implementation**: added two fluent methods, two `Interactivity` fields, and AccessKit node serialization.
- **Observed pass**: the focused node-property test passed.

## RED 7 — Accessibility debug tree

- **Test added**: `debug_tree_json_contains_focus_hierarchy_and_aria_metadata`.
- **Behavior asserted**: a completed accessibility frame exposes deterministic JSON containing root, focus, child relationships, role, label, description, and keyboard shortcut.
- **Observed failure**: compilation failed because `A11y::debug_tree_json` did not exist.
- **Failure is correct because**: the tree was sent to platform adapters and then discarded.

## GREEN 7

- **Minimal implementation**: added `window/a11y/debug.rs`, captured serialized output when `A11y::end_frame` completes, and exposed `Window::debug_a11y_tree_json`.
- **Observed pass**: the focused JSON test passed.

## REFACTOR 7

- **Refactor done**: yes
- **Change**: kept debug serialization outside the tree builder and sorted nodes by AccessKit ID for deterministic output.
- **Observed result**: full GPUI and workspace tests passed.

## RED 8 — Windows visual harness reachability

- **Test added**: Windows-only compile/runtime contract `windows_visual_harness_exposes_directx_screenshot_support`.
- **Behavior asserted**: Windows reports screenshot support and exports `RealVisualTestContext`.
- **Pre-change evidence**: screenshot capability, the visual platform module, the context, and `real_visual_smoke` all excluded Windows even though DirectX implemented `render_to_image`.

## GREEN 8

- **Minimal implementation**: enabled the shared visual platform/context/smoke cfgs for Windows, reported offscreen positioning and screenshot support, and added a native Windows visual smoke entry to `.github/workflows/gpui-feature-matrix.yml`.
- **Local result**: macOS build/tests remain green.
- **Pending proof**: Windows cross-compilation and runtime must run on `mp-dev`/Windows CI; `mp-dev` became unreachable during this follow-up and the local host lacks the Windows C toolchain headers.

## Next Behavior

Run the Windows GNU cross-check when `mp-dev` is reachable, then let the new `windows-2022` CI matrix entry execute the DirectX visual smoke. Wayland WGPU-on-wlroots still requires a runner with a DRM render node.
