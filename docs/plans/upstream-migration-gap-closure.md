# Goal Document: Upstream Migration Gap Closure

## Go / No-Go
- **Judgment**: Go
- **Reason**: The four requested gaps have identifiable upstream references, bounded implementation surfaces, and test seams. Native Linux verification is available through `mp-dev`; Windows behavior can be covered by compile-time and pure readback tests where the host cannot execute DirectX.

## Target Outcome
The repository closes migration findings 1-4 in code and evidence: platform-specific P1 behavior has recorded native Linux validation, GPUI uses Taffy 0.12.2 with direct regression coverage for its coupled layout fixes, Linux WGPU and Windows DirectX implement `render_to_image`, and the bounded GPUI-only AccessKit increment provides descriptions, keyboard-shortcut metadata, and a debug accessibility tree.

## Goal Definition
- **Type**: technical / quality / delivery
- **Boundary**: Linux native validation, Taffy 0.12.2 migration, Linux WGPU and Windows DirectX scene readback, GPUI-only AccessKit description/shortcut metadata, accessibility debug-tree output, and Windows visual-harness readiness.
- **Non-goals**:
  - `gpui_web`, mobile/touch APIs, or Zed application UI migration.
  - Pixel-perfect cross-platform golden snapshots.
  - Full scheduler crate migration.
  - Replacing Adabraka platform extensions or crate boundaries.
- **Deferred work**:
  - Windows runtime validation if no Windows runner is available.
  - Strict cross-platform screenshot equivalence thresholds.
- **Verification rule**: Every behavioral change starts with a focused failing test or compile failure, passes locally after the minimal implementation, then passes the GPUI suite and workspace checks. Linux-specific behavior additionally passes on `mp-dev`.
- **Evidence source**: Rust tests, cargo checks, upstream diff review, native Linux execution, formatting and diff checks.
- **Pass criteria**:
  - Taffy resolves to 0.12.2 and coupled list/text/layout tests pass.
  - Linux WGPU and Windows DirectX no longer inherit the unsupported `render_to_image` default.
  - Linux readback produces an image with the expected dimensions and non-uniform content in a native smoke test where the environment supports it.
  - AccessKit core is documented as complete; `aria_description`, `aria_keyshortcuts`, and `debug_a11y_tree_json` are implemented and covered by focused tests.
  - Requested native Linux validation commands and outcomes are recorded.
  - Full GPUI tests, workspace check, rustfmt, and `git diff --check` pass.
- **Confidence note**: Linux runtime evidence is native. Windows readback can be compiled and structurally tested but remains a runtime confidence gap without a Windows runner.
- **Judgment owner**: Automated tests and native Linux command evidence; Windows release acceptance remains owned by a Windows runner/manual gate.

## Current State
- The seven P1 backports and their focused tests are present.
- Linux and Windows platform release gates were previously documented but not closed.
- Taffy is 0.10.1 locally and 0.12.2 upstream.
- macOS implements `PlatformWindow::render_to_image`; Linux and Windows inherit the unsupported default.
- AccessKit core and macOS/Linux/Windows adapters are implemented, but the July 1 audit still says the tree is absent.
- `mp-dev` is reachable at `/home/ubuntu/work/my/adabraka-gpui` on x86_64 Ubuntu.

## Priority Rationale
- Upgrade Taffy first because it changes shared layout contracts and may affect later visual-test assertions.
- Implement renderer readback after the layout baseline is green.
- Apply the small accessibility increment and documentation correction after core code risks are contained.
- Finish with native Linux validation across all touched areas and the previously deferred P1 gates.

## Assumptions and Open Decisions

| Item | Status | Impact | Owner / Next step |
|---|---|---|---|
| Upstream Taffy changes can be adapted without crate restructuring | confirmed | Keeps the migration bounded | Port the three upstream commits in order |
| Linux runner has a usable display/GPU for real visual smoke | confirmed for X11 | Xvfb + Vulkan llvmpipe supports native render/readback; Wayland lacks a compatible DRM render node | X11 smoke passed; Wayland blocker recorded |
| DirectX readback can reuse the existing renderer/device resources | confirmed | Keeps Windows implementation additive and test-support-only | Shared normal render path plus staging texture implemented and cross-compiled |
| Accessibility scope should remain GPUI-only | confirmed | Avoids importing Zed application UI changes | Port only GPUI APIs/tests/docs |

## Phases

### Phase 1: Taffy 0.12.2 migration
- **Purpose**: Align the layout dependency and preserve known list/text behavior after the upstream semantic changes.
- **Entry condition**: Current layout and GPUI tests pass.
- **Phase rules**:
  - Port upstream commits `91fdd558`, `b05f40c5`, and `31ceaf79` in order.
  - Add or run focused tests that expose the post-upgrade behavior before applying coupled fixes.
  - Do not mix unrelated style or layout refactors.
- **Todos**:
  - [x] Upgrade the dependency and adapt style conversions.
    - **Surface**: `crates/gpui/Cargo.toml`, `Cargo.lock`, `style.rs`
    - **Proof**: Initial check exposes only expected API/behavior changes, then compiles.
    - **Depends on**: none
  - [x] Port coupled list/text/uniform-list fixes.
    - **Surface**: `elements/list.rs`, `elements/text.rs`, `elements/uniform_list.rs`
    - **Proof**: Existing focused layout tests and full GPUI suite pass.
    - **Depends on**: dependency upgrade
  - [x] Add direct regression tests for optional-width `MaxContent`, truncated-cache rejection, and exact-fit shaped text.
    - **Surface**: `taffy.rs`, `elements/text.rs`
    - **Proof**: Tests fail against the pre-fix semantics and pass with the migrated behavior.
    - **Depends on**: coupled fixes
- **Exit proof**: Cargo resolves Taffy 0.12.2 and all layout-related tests pass locally and on `mp-dev`.
- **Stop condition**: The upgrade requires public layout behavior decisions not represented by upstream fixes.

### Phase 2: Cross-platform render readback
- **Purpose**: Give Linux WGPU and Windows DirectX the same test-support readback contract already available on macOS.
- **Entry condition**: Phase 1 is green.
- **Phase rules**:
  - Keep `render_to_image` test-support-only.
  - Reuse existing renderer/device/scene resources; do not add a second rendering pipeline.
  - Validate dimensions, alpha/content, row pitch, and BGRA/RGBA conversion explicitly.
- **Todos**:
  - [x] Add failing platform-contract tests for Linux and Windows overrides/readback helpers.
    - **Surface**: renderer tests and platform window implementations
    - **Proof**: Tests fail because the implementations/helpers are absent.
    - **Depends on**: Phase 1
  - [x] Implement WGPU scene-to-image readback.
    - **Surface**: WGPU renderer and Linux platform windows
    - **Proof**: Unit tests plus native `mp-dev` real visual smoke.
    - **Depends on**: failing tests
  - [x] Implement DirectX scene-to-image readback.
    - **Surface**: DirectX renderer and Windows platform window
    - **Proof**: Pure conversion/row-pitch tests and Windows target compilation where available.
    - **Depends on**: failing tests
- **Exit proof**: Both backends override `render_to_image`; Linux native screenshot smoke passes or an explicit environment blocker is recorded, and Windows target compilation passes where toolchains permit.
- **Stop condition**: Backend resource ownership requires a public renderer contract change.

### Phase 3: Accessibility increment and status correction
- **Purpose**: Complete the bounded user-facing GPUI accessibility increment that was originally in scope.
- **Entry condition**: Core build and renderer tests are green.
- **Phase rules**:
  - Preserve the historical 2026-07-01 finding as time-scoped context.
  - Do not import Zed application/UI call-site changes or the broader landmark/menu behavior batch.
  - Add focused RED tests before each public API or debug output implementation.
- **Todos**:
  - [x] Add `aria_description` and `aria_keyshortcuts` with AccessKit node assertions.
    - **Surface**: `elements/div.rs`
    - **Proof**: Fluent API values appear as AccessKit description and keyboard shortcut properties.
    - **Depends on**: Phase 2
  - [x] Add `debug_a11y_tree_json` with deterministic JSON tests.
    - **Surface**: `window/a11y.rs`, `window/a11y/debug.rs`, `window.rs`
    - **Proof**: The latest built tree serializes root, focus, child relationships and ARIA properties.
    - **Depends on**: description/shortcut properties
  - [x] Correct migration status documentation.
    - **Surface**: `docs/sync/ZED_GPUI_INCREMENTAL_AUDIT_2026-07-01.md`
    - **Proof**: The document distinguishes completed core/adapters, the completed bounded increment, and deferred landmark/menu work.
    - **Depends on**: focused accessibility tests
- **Exit proof**: Focused accessibility tests pass and current docs no longer classify these three APIs as remaining work.
- **Stop condition**: The bounded APIs require Zed application state or the broader landmark/menu migration.

### Phase 4: Native validation and closure
- **Purpose**: Close the deferred Linux platform gates and record residual Windows/macOS limits honestly.
- **Entry condition**: Phases 1-3 are green locally.
- **Phase rules**:
  - Run commands from the exact commit being reported.
  - Record unsupported environmental gates instead of treating a skipped test as a pass.
- **Todos**:
  - [x] Synchronize the commit to `mp-dev` and run Linux feature checks/tests.
    - **Surface**: native Linux checkout
    - **Proof**: command outputs for WGPU, Wayland/X11, visual smoke, layout and accessibility tests.
    - **Depends on**: Phases 1-3
  - [x] Update migration documentation with validation outcomes.
    - **Surface**: latest migration/plan docs
    - **Proof**: commands, host, pass/skip/failure and residual risks are recorded.
    - **Depends on**: native execution
- **Exit proof**: Local and Linux validation matrices are green, with Windows/macOS residual gates explicitly documented.
- **Stop condition**: `mp-dev` cannot build the repository after three recoverable environment attempts or requires external credentials/toolchain installation.

### Phase 5: Windows visual-harness readiness
- **Purpose**: Remove the mismatch where DirectX supports readback but the standard visual test entry point excludes Windows.
- **Entry condition**: Accessibility and layout tests are green.
- **Phase rules**:
  - Make Windows use the same visual-test contract; do not claim runtime success without a native Windows runner.
  - Keep the runner requirement explicit when only cross-compilation is available.
- **Todos**:
  - [x] Enable `VisualTestPlatform`, `RealVisualTestContext`, screenshot capability, and `real_visual_smoke` for Windows.
    - **Surface**: `platform/test.rs`, `app/test_context.rs`, `tests/real_visual_smoke.rs`
    - **Proof**: Shared harness remains green on macOS and a Windows-only compile contract is present.
    - **Depends on**: Phase 3
  - [ ] Re-run the Windows GNU cross-target check.
    - **Surface**: `x86_64-pc-windows-gnu` toolchain on `mp-dev`
    - **Proof**: Updated visual harness compiles for Windows.
    - **Depends on**: `mp-dev` becoming reachable
  - [ ] Execute the smoke on a native Windows runner.
    - **Surface**: `windows-2022` CI matrix entry
    - **Proof**: DirectX render/readback dimensions, alpha, and non-uniform pixels pass.
    - **Depends on**: pushing the changes so native CI can run
- **Exit proof**: Harness compiles for Windows; native runtime evidence is attached when a runner exists.
- **Stop condition**: No native Windows runner is available; record this as an external gate rather than a code pass.

## Dry-Run Findings
- Taffy must precede screenshot assertions because layout changes can alter visual output.
- Linux WGPU readback should be implemented in the shared renderer rather than separately in Wayland and X11.
- Windows runtime validation cannot be claimed from Linux; implementation, pure conversion tests, and Windows GNU cross-compilation are closed here.
- The July 1 AccessKit statement is preserved as historical context and annotated with the July 10 core completion plus the bounded 2026-07-24 increment.
- The follow-up completes finding 4's bounded code scope; broader landmarks/menu behavior remains a separate migration batch.

## Final Validation
- `cargo test -p adabraka-gpui --lib --features test-support -- --test-threads=1`
- `cargo test --workspace --lib --tests -- --test-threads=1`
- `cargo check --workspace`
- `cargo fmt --all -- --check`
- `git diff --check`
- Native `mp-dev` Linux checks for `wgpu`, `wayland`, `x11`, accessibility, and real visual smoke.

## First Execution Step
Port the Taffy dependency/API change, run the smallest layout check to capture the expected RED state, then apply the coupled upstream layout fixes.
