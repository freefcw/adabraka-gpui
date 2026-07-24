# Goal Document: Zed GPUI P1 Backport

## Go / No-Go

- **Judgment**: Go
- **Reason**: The upstream range and local gaps identify eight reviewable P1 themes with bounded ownership surfaces. Existing Adabraka extensions can be preserved without crate restructuring or broad dependency migration.

## Target Outcome

Adabraka GPUI contains the seven applicable P1 correctness, interaction, platform, and rendering improvements from Zed range `717e1c0590fde11b6d397e0647f7cc0b9b3245e6..8f6f8a54b9933059f1963af601369e0555e32649`, with one independently reviewable local commit per upstream theme and full `Zed-Origin` provenance.

## Goal Definition

- **Type**: quality and delivery
- **Boundary**: Backport prompt click isolation, static-image EXIF orientation, auto-sized root fill, border-only quad overdraw reduction, runtime Wayland layer-shell exclusive-zone controls, macOS custom-titlebar drag ownership, and per-window attention. Audit group-hover redraw suppression for local equivalence.
- **Non-goals**:
  - Mobile/touch APIs, container queries, AccessKit expansion, gpui_web, and Zed application UI changes.
  - Taffy 0.12 migration and its coupled layout fixes.
  - Replacing Adabraka tray, daemon, hotkey, notification, layer-shell, or global attention extensions.
  - Applying EXIF orientation to animated GIF/WebP frames.
- **Deferred work**:
  - Native Linux/Windows runtime validation when the corresponding machines/toolchains are available.
  - Taffy dependency migration as a separate architecture batch.
- **Verification rule**: Each behavior has a RED regression test or a minimal platform validation, its focused test/check passes after implementation, the GPUI/workspace suites pass, and every backport commit records its full origin hash.
- **Evidence source**: Rust tests, host compilation, cross-target checks where toolchains permit, diff review, and native-platform manual test recipes.
- **Pass criteria**: All seven applicable themes are committed independently, group-hover has an evidence-backed disposition, host tests/checks and changed-file formatting pass, no known regression remains, and unavailable native gates are explicitly recorded.
- **Confidence note**: Pure layout, interaction, image, and geometry behavior can be covered directly. macOS, Wayland, Windows, and X11 behavior also requires native validation beyond host-side compilation or fake-platform tests.
- **Judgment owner**: Automated tests plus final independent code review; platform CI/manual validation owns native release acceptance.

## Current State

- P0 backports are complete in six clean commits ending at `6cbfe44`.
- The previous 2026-07-01 P1 batch is already present and must not be repeated.
- Seven newer P1 themes are absent or only partially represented in the monolithic `crates/gpui` platform structure. The local group-hover path already notifies only on hover transitions and does not use upstream's later stateful hover architecture.
- Adabraka already has creation-time layer-shell settings and application-global attention APIs; new behavior must extend those paths rather than replace them.
- Linux and Windows cross compilation is constrained by missing native C toolchains on this host.

## Priority Rationale

- Start with deterministic interaction tests to establish fast feedback and small commits.
- Follow with pure layout/image correctness, then platform APIs whose native behavior needs manual gates.
- Isolate border-overdraw geometry and attention API work last because they touch shared rendering/platform contracts.

## Assumptions and Open Decisions

| Item | Status | Impact | Owner / Next step |
| --- | --- | --- | --- |
| `image` resolves to an orientation-capable 0.25 release | confirmed | EXIF can be implemented without a dependency bump | Verify through RED test and compilation |
| Runtime layer-shell edge must retain local protocol-version checks | confirmed | Prevents fatal protocol errors on pre-v5 compositors | Preserve guard and add pure validation tests |
| Per-window attention must coexist with existing global attention | confirmed | Avoids API regression for daemon/tray apps | Add a window-targeted path; retain existing App API |
| macOS 27 custom-titlebar behavior cannot be exercised on all host versions | assumed | Compile plus manual recipe is the available proxy | Record native validation steps |

## Phases

### Phase 1: Prompt isolation and group-hover disposition

- **Purpose**: Prevent modal prompts from dismissing underlying popovers and verify whether the group-hover optimization applies to the fork.
- **Entry condition**: Clean P0 baseline.
- **Phase rules**:
  - Add upstream-compatible regression tests before production changes.
  - Do not alter general mouse propagation or drag behavior.
- **Todos**:
  - [x] Audit unchanged group-hover notifications.
    - **Surface**: `elements/div.rs`
    - **Proof**: Local `paint_hover_group_handler` already compares transition state; the upstream test's failure exposed unsupported layout-affecting group-hover state rather than redundant redraws, so no patch was applied.
    - **Depends on**: none
  - [x] Suppress `on_mouse_down_out` while a GPUI prompt is active.
    - **Surface**: `elements/div.rs`, `window.rs`
    - **Proof**: Prompt regression test passes while non-prompt behavior remains covered.
    - **Depends on**: none
- **Exit proof**: Focused div tests pass; commit `c98e132` records origin `058f01fa93503491a735bfded53e77bfaa276148`; `ae3bbdeb2df000acd8e0c5c75275cbb357e2b223` is recorded as locally equivalent and architecture-inapplicable.
- **Stop condition**: The tests require changing event propagation outside the affected handlers.

### Phase 2: Root layout and image correctness

- **Purpose**: Match viewport-root semantics and honor static-image EXIF orientation.
- **Entry condition**: Phase 1 passes.
- **Phase rules**:
  - Stretch only window and prompt roots with auto dimensions.
  - Preserve explicit dimensions and shrink-wrap behavior of drag, tooltip, deferred, and anchored roots.
  - Keep animated image decoding unchanged.
- **Todos**:
  - [x] Add auto-root RED tests and Taffy stretch helper.
    - **Surface**: `taffy.rs`, `window.rs`
    - **Proof**: Auto roots fill the viewport; explicit roots retain dimensions.
    - **Depends on**: none
  - [x] Add oriented JPEG fixture/test and shared static decoder helpers.
    - **Surface**: `platform.rs`, `elements/img.rs`, test fixture
    - **Proof**: Dimensions and corner pixels prove orientation in both static decode paths.
    - **Depends on**: orientation-capable `image` resolution
- **Exit proof**: Focused layout/image tests and host check pass; commits `28e2f7a` and `20c07ec` record origins `b0da438545633412d6792796b3d92337ea44146d` and `9552acc2bc242d45342fa9b5a987d43868aee1ec`.
- **Stop condition**: EXIF support requires a broad image dependency upgrade.

### Phase 3: Platform window controls

- **Purpose**: Allow safe runtime layer-shell reservation changes and separate macOS titlebar drag ownership from movability.
- **Entry condition**: Phase 2 passes.
- **Phase rules**:
  - Preserve local layer-shell protocol-version guards and extensions.
  - Invalid exclusive edges must be ignored rather than sent to the compositor.
  - Preserve `is_movable`; add an independent macOS-only semantic option.
- **Todos**:
  - [x] Add runtime exclusive-zone and exclusive-edge APIs.
    - **Surface**: platform/window traits and Wayland window state
    - **Proof**: Pure edge validation/version tests plus Wayland compilation where available.
    - **Depends on**: existing creation-time layer-shell support
  - [x] Add `app_owns_titlebar_drag` and AppKit content-view override.
    - **Surface**: window options/params, macOS window class, example or test
    - **Proof**: macOS host compilation and documented native custom/native-titlebar checks.
    - **Depends on**: none
- **Exit proof**: Commits `d28f691` and `51a7c8f` record origins `166f044fd046e0de73cd64a027505db90f523b97` and `23bb2fc135a69492847c3aa68444a7d14cc282f6`.
- **Stop condition**: A change would remove or fork Adabraka's existing layer-shell/window APIs.

### Phase 4: Rendering cost and attention targeting

- **Purpose**: Avoid shading transparent quad interiors and add window-targeted attention without duplicating global attention.
- **Entry condition**: Platform contracts from Phase 3 compile.
- **Phase rules**:
  - Keep border geometry identical under asymmetric widths, radii, clipping, and tiny bounds.
  - Do not import unrelated DirectX debug-label code.
  - Preserve existing application-global attention and cancellation APIs.
- **Todos**:
  - [x] Split eligible border-only quads into four bounded strips.
    - **Surface**: `window.rs`, scene geometry tests
    - **Proof**: Geometry/primitive-count tests and renderer suite pass.
    - **Depends on**: none
  - [x] Add per-window attention through existing platform backends.
    - **Surface**: platform/window traits and macOS/Windows/X11/Wayland implementations
    - **Proof**: Fake-platform dispatch test, host compilation, native manual recipe.
    - **Depends on**: existing `AttentionType` contract
- **Exit proof**: Commits `2728659` and `8931558` record origins `0c51c7fd2481859e9da5c490ef8e41ddbcf1a341` and `905e955a702707cd81a2e5bae9b381a7a9c7f614`.
- **Stop condition**: Per-window attention cannot share the existing attention semantics without a public compatibility break.

## Dry-Run Findings

- Upstream layer-shell code lacks the fork's protocol-version guard; the local backport must combine both.
- Upstream attention is informational-only while Adabraka has `AttentionType` and cancellation; a direct copy would regress the richer API.
- The border-only commit bundles DirectX debug annotations that are not required for the optimization and must stay out.
- Existing App-level attention and effective `ExitProcess` removal are equivalent coverage, not duplicate backport targets.

## Final Validation

- `cargo test -p adabraka-gpui --lib --features test-support -- --test-threads=1`
- `cargo test --workspace --lib --tests -- --test-threads=1`
- `cargo check --workspace`
- Changed-file `rustfmt --check` and `git diff --check`
- Native Linux/Wayland, Windows/MSVC, and macOS manual gates as applicable
- Independent review of the complete P1 commit range and every `Zed-Origin`

## Validation Closure (2026-07-24)

The previously deferred platform gates were revisited with the final migration-gap closure changes.

| Gate | Environment | Result | Evidence |
| --- | --- | --- | --- |
| GPUI Linux suite | `mp-dev`, Ubuntu x86_64 | passed | `cargo test -p adabraka-gpui --lib --features test-support,wayland,x11 -- --test-threads=1`: 219 passed |
| Linux feature compilation | `mp-dev` | passed | `cargo check -p adabraka-gpui --no-default-features --features wgpu,wayland,x11` |
| X11 WGPU render/readback | `mp-dev`, Xvfb, Vulkan llvmpipe | passed | `real_visual_smoke` rendered, requested per-window attention, read back RGBA pixels, and exited 0 |
| Wayland layer-shell edge validation | `mp-dev` native Linux build | passed | focused `exclusive_edge` tests: 2 passed; combined Wayland/X11 build passed |
| Wayland real compositor smoke | `mp-dev`, Sway headless | environment blocked | Sway pixman starts and exposes layer-shell, but the VM has no DRM render node; WGPU's Vulkan adapter cannot configure the pixman Wayland surface. This remains a runner capability gate, not an unrecorded validation gap. |
| Windows compilation | `mp-dev`, `x86_64-pc-windows-gnu` | passed | full GPUI Windows-target `cargo check`; DirectX readback and per-window attention compile |
| Windows runtime | unavailable | deferred | Requires a Windows runner for taskbar attention and DirectX staging-texture execution |
| macOS suite | local macOS host | passed | GPUI lib tests: 197 passed, 2 ignored; workspace check and rustfmt passed |

This closes the missing validation-record problem. Linux X11 behavior is runtime-validated, Linux Wayland is compile/pure-test validated with the exact compositor blocker recorded, and Windows runtime acceptance remains an explicit platform-runner responsibility.

## Follow-up Closure (2026-07-24)

- Added direct regression contracts for unbounded list `MaxContent`, truncated text-cache rejection, and exact-fit shaped text.
- Completed the bounded GPUI-only accessibility increment from upstream `2268045a119030735f762e5afaf59da0bda869f4`: `aria_description`, `aria_keyshortcuts`, and `Window::debug_a11y_tree_json`.
- Kept landmarks/menu, focus provenance, and tab-group behavior out of this batch.
- Enabled `VisualTestPlatform`, `RealVisualTestContext`, screenshot capability detection, and `real_visual_smoke` for Windows.
- Added a native `windows-2022` DirectX visual smoke entry to `.github/workflows/gpui-feature-matrix.yml`.
- Local final evidence after the follow-up: GPUI 203 passed/2 ignored; workspace 265 passed/2 ignored; workspace check, rustfmt, and diff check passed.
- External proof still pending: `mp-dev` became unreachable before the updated Windows cfg could be re-cross-compiled; the native DirectX result will come from the new Windows CI job.

## First Execution Step

Add the upstream group-hover transition regression test and confirm it fails by observing redundant render/paint counts before changing the notification logic.
