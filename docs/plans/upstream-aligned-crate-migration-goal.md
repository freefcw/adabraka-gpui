# Goal Document: Upstream-Aligned Crate Migration

## Go / No-Go

- **Judgment**: Go
- **Reason**: The repository has a green baseline, the public compatibility contract is known, and the migration can be split into independently verifiable boundaries.

## Target Outcome

Adabraka GPUI keeps `adabraka-gpui` as its stable downstream entry point while moving renderer and operating-system implementations into upstream-aligned crates. Existing applications continue to compile with their current dependency declaration, startup API, feature names, and desktop behavior.

## Goal Definition

- **Type**: Technical and quality
- **Boundary**: Fix public macro resolution; add an injectable platform construction seam; create a compatibility facade; split WGPU and desktop platform implementations; preserve current tray, hotkey, overlay, daemon, permission, notification, single-instance, and resource-profile behavior.
- **Non-goals**:
  - Migrating the upstream scheduler, View API, web backend, or mobile APIs.
  - Redesigning desktop capabilities into a new service layer without measured need.
  - Removing or renaming currently documented downstream features.
  - Combining new upstream features with mechanical crate moves.
- **Deferred work**:
  - Structured notification actions, anchored popup/dialog behavior, container queries, scheduler, and web/mobile support.
  - Feature-default reduction after compile-time and binary-size measurements exist.
- **Verification rule**: Every phase must keep the public compatibility fixture, workspace check, core test suite, examples, and locally installed target checks green before the next phase starts.
- **Evidence source**: Cargo compile fixtures, unit and integration tests, target checks, public API usage examples, and a final diff review.
- **Pass criteria**: Existing `adabraka-gpui` dependency syntax and `gpui::Application::new()` continue to compile; documented feature names remain accepted; the core test suite remains green; platform implementations no longer reside in the core implementation crate at completion.
- **Confidence note**: Compile and test evidence covers the local macOS target directly and Zig-backed Windows/Linux type-checks, including Windows public all-features after pinning the upstream-compatible capture dependency. Linux and Windows runtime desktop integration still require platform CI or host smoke tests.
- **Judgment owner**: Automated tests and target checks, followed by repository diff review.

## Current State

- `adabraka-gpui` is one published crate containing core, renderer, test platform, Linux, macOS, and Windows implementations.
- `Application::new()` selects a concrete backend inside core.
- `Platform` and several supporting backend traits are crate-private.
- The public derive macros generate a hard-coded `adabraka_gpui` path that fails for the documented `gpui` crate name.
- The workspace baseline passes `cargo check --workspace` and the current core test suite, with ignored/native visual coverage bounded separately.
- Adabraka-specific desktop behavior is implemented and must remain intact.

## Plan Rewrite Notes

| Existing item | Decision | Reason |
|---|---|---|
| Fix every issue before migration | Rewrite | Only proven blockers are fixed first; unrelated changes would obscure migration regressions. |
| Add `DesktopServices` immediately | Remove | No downstream or sync-conflict evidence justifies a new service-locator layer. |
| Keep `adabraka-gpui` as pure core | Rewrite | The published package remains a compatibility entry point so downstream startup code stays stable. |
| Mirror upstream platform crates | Keep | This is the change that directly reduces path mapping and sync amplification. |
| Migrate scheduler/View/web together | Remove | These are independent semantic changes and are not required for the crate boundary. |

## Drift Diagnosis

- **Goal drift**: New framework capabilities do not prove lower sync cost or downstream compatibility.
- **Phase drift**: Helper-crate extraction alone does not prove that the platform boundary works.
- **Validation drift**: Successful file movement is insufficient without downstream compile and behavior evidence.
- **Compatibility drift**: Removing `Application::new()` would transfer internal migration cost to every consumer.
- **Cleanup drift**: Feature cleanup and broad API redesign are excluded from mechanical movement phases.

## Priority Rationale

- Macro resolution is fixed first because it is a confirmed public failure and will affect any facade/core package rename.
- Platform injection comes before file movement to avoid a core-to-backend dependency cycle.
- Renderer and one platform are split as narrow proof points before all backends move.
- Behavior changes are deferred until the physical dependency direction is stable.

## Assumptions and Open Decisions

| Item | Status | Impact | Owner / Next step |
|---|---|---|---|
| Existing package remains the public compatibility entry point | Confirmed | Avoids mandatory downstream Cargo and startup changes | Enforced by compile fixture |
| Internal crates may use new package names | Confirmed | Cargo.lock changes, public imports do not | Keep internal dependencies exact and unpublished until stable |
| Platform traits can be made available to backend crates | Confirmed | Creates a new low-level contract | Keep backend APIs documented as internal/unstable |
| Linux and Windows runtime smoke tests are available locally | Unresolved | Cross-compilation cannot prove runtime behavior | Require platform CI before release |

## Phases

### Phase 1: Public Compatibility Baseline

- **Purpose**: Fix the proven derive failure and lock down the downstream contract.
- **Entry condition**: Clean worktree and green baseline.
- **Phase rules**:
  - Production changes require a failing compile test first.
  - Do not move platform files or change runtime behavior.
- **Todos**:
  - [ ] Resolve the GPUI crate path dynamically in all proc macros.
    - **Surface**: `crates/gpui-macros`
    - **Proof**: Macro tests compile with the normal `gpui` dependency name.
    - **Depends on**: None
  - [ ] Add a downstream compatibility fixture for dependency, derives, startup, daemon, overlay, and documented feature APIs.
    - **Surface**: Test fixture
    - **Proof**: Fixture compiles against the workspace package.
    - **Depends on**: Macro fix
- **Exit proof**: Macro tests, fixture check, workspace check, and core tests pass.
- **Stop condition**: A compatibility fixture requires an undocumented dependency alias.

### Phase 2: Injectable Construction Boundary

- **Purpose**: Let a facade construct the application without core selecting a concrete OS backend.
- **Entry condition**: Phase 1 green.
- **Phase rules**:
  - Additive API only; `Application::new()` and `headless()` remain functional.
  - No backend implementation moves in this phase.
- **Todos**:
  - [ ] Add `Application::with_platform` and expose the minimum backend contracts.
    - **Surface**: Core application and platform contracts
    - **Proof**: A fake platform constructs an application through the new seam.
    - **Depends on**: Phase 1
  - [ ] Add an internal platform facade that delegates to the existing backend selector.
    - **Surface**: New facade crate
    - **Proof**: Existing examples and compatibility fixture still compile unchanged.
    - **Depends on**: Injection seam
- **Exit proof**: Both old and injected construction paths pass tests.
- **Stop condition**: The seam requires duplicating backend state or behavior.

### Phase 3: Package Inversion and Compatibility Root

- **Purpose**: Make the published package the outward composition root before any concrete backend leaves core.
- **Entry condition**: Phase 2 green and independently reviewed.
- **Phase rules**:
  - Keep package `adabraka-gpui`, lib target `gpui`, all feature names/defaults, examples, README, and downstream startup syntax at the compatibility root.
  - Move implementation source to `adabraka-gpui-core`, lib target `gpui_core`.
  - Wrap only `Application`; directly re-export every other core nominal type.
  - Do not move a backend or WGPU in this phase.
- **Todos**:
  - [x] Move the current implementation source and unit tests to `adabraka-gpui-core`.
    - **Surface**: Package graph and source ownership
    - **Proof**: Core package checks and unit tests pass.
    - **Depends on**: Phase 2
  - [x] Convert `adabraka-gpui` into the compatibility umbrella with an exhaustive `Application` wrapper.
    - **Surface**: Public package, re-exports, examples, integration tests, features
    - **Proof**: Existing downstream fixtures remain source-identical and green; all current Application methods compile through the wrapper.
    - **Depends on**: Core package creation
  - [x] Rewire `gpui-platform` to depend on core, while still selecting the in-core backends temporarily.
    - **Surface**: Composition facade
    - **Proof**: Cargo metadata has no internal dependency on the umbrella and no cycle.
    - **Depends on**: Core package creation
- **Exit proof**: Umbrella, core, facade, examples, fixtures, and core tests are green with no backend source duplicated.
- **Stop condition**: Preserving the public entry point requires copying core implementation or wrapping any public type other than Application.

### Phase 4: Linux Extraction Bridge

- **Purpose**: Move Linux/FreeBSD out of core while temporarily consuming the single in-core WGPU implementation through a hidden bridge.
- **Entry condition**: Phase 3 green.
- **Phase rules**:
  - Preserve all fork-only Linux modules and public features.
  - Do not copy WGPU; expose only the minimum current WGPU entry types from core.
  - Switch only Linux selection in `gpui-platform` after checks pass.
- **Todos**:
  - [x] Extract `adabraka-gpui-linux` and its target dependencies/features.
    - **Surface**: Linux, Wayland, X11, headless, layer shell, desktop integrations
    - **Proof**: Linux no-feature, X11-only, Wayland-only, and combined checks.
    - **Depends on**: Phase 3
  - [x] Remove the in-core Linux implementation after facade selection is green.
    - **Surface**: Core module graph
    - **Proof**: No Linux implementation file remains in core; no duplicate implementation exists.
    - **Depends on**: External Linux checks
- **Compatibility exception**: `RealVisualTestContext` is wrapped only at the published compatibility root under `test-support`. Core owns injected-platform constructors and the umbrella preserves the existing no-argument `new`, `new_if_supported`, `with_asset_source`, consuming `run`, method deref, and public `app` field surface. This evidence-backed wrapper prevents core from selecting the extracted Linux backend; no registry or service locator is introduced.
- **Validation note**: An isolated Zig cross compiler completed `x86_64-unknown-linux-gnu` type-checks for headless, Wayland-only, X11-only, combined, and the public `test-support` composition. Those checks exposed and closed direct-dependency, orphan-rule, privacy, pixel-conversion, WGPU-bridge, and clipboard-construction defects. The `screen-capture` combination still stops before GPUI in the third-party `x11` build script because no Linux pkg-config sysroot is installed. Native X11/Wayland/FreeBSD runtime CI remains required.
- **Exit proof**: `gpui_linux -> gpui_core`; facade selects it on Linux; WGPU still exists once in core.
- **Stop condition**: Linux requires more than a narrow hidden WGPU bridge or any product behavior change.

### Phase 5: WGPU Extraction

- **Purpose**: Move the renderer after Linux no longer makes core depend outward.
- **Entry condition**: Phase 4 green.
- **Phase rules**:
  - Preserve atlas sizing, instance-buffer normalization, device-loss recovery, shader behavior, and current trim/stats semantics.
  - Do not import upstream subpixel/text features during movement.
- **Todos**:
  - [x] Extract `adabraka-gpui-wgpu` depending only on core.
    - **Surface**: WGPU context, atlas, renderer, shader, Cargo features
    - **Proof**: Renderer tests and Linux feature checks pass against the external crate.
    - **Depends on**: Phase 4
  - [x] Remove the temporary core WGPU bridge and implementation.
    - **Surface**: Core renderer modules
    - **Proof**: One WGPU implementation remains and Cargo metadata is acyclic.
    - **Depends on**: Linux switched to external WGPU
- **Exit proof**: `gpui_linux -> gpui_wgpu -> gpui_core`; core has no concrete WGPU implementation. The moved crate's 5 tests pass, the WGSL source is byte-identical, and Linux combined plus public `test-support` cross-target checks compile against the external renderer.
- **Stop condition**: A custom resource profile or recovery path changes behavior.

### Phase 6: macOS Extraction

- **Purpose**: Move AppKit, Metal, and Adabraka macOS integrations out of core.
- **Entry condition**: Phase 5 green.
- **Phase rules**:
  - Move shader/build responsibilities with the backend.
  - Preserve tray, hotkey, permission, notification, biometric, power, network, screen capture, and GPU cache behavior.
- **Todos**:
  - [x] Extract `adabraka-gpui-macos`, switch facade selection, and remove the in-core implementation.
    - **Surface**: macOS backend and build script
    - **Proof**: Native macOS tests and examples pass.
    - **Depends on**: Phase 5
- **Exit proof**: One macOS implementation remains outside core. The Metal and dispatch sources are byte-identical, both precompiled and runtime shader build paths pass, and the backend owns 14 passing tests with 2 ignored native visual tests.
- **Stop condition**: Metal shader output or a desktop integration changes behavior.

### Phase 7: Windows Extraction

- **Purpose**: Move Win32, DirectX, and Adabraka Windows integrations out of core.
- **Entry condition**: Phase 6 green.
- **Phase rules**:
  - Move HLSL and manifest/build responsibilities with the backend.
  - Preserve tray, hotkey, overlay, notification, power, network, auto-launch, and jump-list behavior.
- **Todos**:
  - [x] Extract `adabraka-gpui-windows`, switch facade selection, and remove the in-core implementation.
    - **Surface**: Windows backend and build script
    - **Proof**: Zig-backed no-default and all-feature checks pass for both the Windows backend and public umbrella after pinning the upstream-compatible `windows-capture` revision. Native host checks remain external.
    - **Depends on**: Phase 6
- **Exit proof**: One Windows implementation remains outside core. Win32/DirectX sources, HLSL, manifest/resources, and build logic are owned by `adabraka-gpui-windows`; core retains only contracts and product-independent Windows primitives.
- **Stop condition**: DirectX shader/manifest output or a desktop integration changes behavior.

### Phase 8: Final Cleanup and Release Proof

- **Purpose**: Remove temporary selectors/bridges and prove the public package hides the internal split.
- **Entry condition**: Phase 7 code extraction is green; native runtime and registry release proof remain explicitly bounded.
- **Phase rules**:
  - Existing dependency syntax, startup API, feature names/defaults, examples, and daemon behavior are hard contracts.
  - Core contains contracts and application state, not concrete desktop/render backends.
- **Todos**:
  - [ ] Remove temporary core backend selectors, target dependencies, and hidden bridges.
    - **Surface**: Core Cargo and module graph
    - **Proof**: Metadata dependency-direction check and source inventory.
    - **Depends on**: Phase 7
  - [ ] Update README and migration notes with internal topology and advanced opt-in APIs.
    - **Surface**: Documentation
    - **Proof**: Documented commands pass.
    - **Depends on**: Final package layout
- **Exit proof**: Full validation matrix passes and the diff contains no unexplained behavior changes.
- **Stop condition**: A temporary compatibility path still owns behavior or duplicates implementation.

## Dry-Run Findings

- Macro crate-path resolution must precede package inversion because generated code needs to resolve both umbrella and core contexts.
- `Application::new/headless` must move to the published umbrella before concrete backends can leave core; a re-export cannot add inherent methods.
- WGPU cannot move while Linux remains in core without a cycle. The acyclic route is package inversion, Linux extraction through a temporary core WGPU bridge, then WGPU extraction.
- Examples, integration tests, README, and downstream fixtures belong to the umbrella; core unit tests move with implementation source.
- Cross-target checks are available locally, but native Linux and Windows runtime behavior still requires platform hosts.
- The desktop-service redesign is not needed and remains outside this goal.

## Final Validation

- `cargo check --workspace`
- Core and umbrella test suites, macro tests, and both downstream fixtures.
- Compile every current example through the umbrella.
- Check installed macOS, Linux, and Windows Rust targets with supported feature sets.
- Verify Cargo metadata contains no internal dependency on the umbrella and no dependency cycle.
- Verify concrete renderer/platform implementation files are outside core and public compatibility APIs remain available.

## First Execution Step

Phases 1-7 are complete. Execute Phase 8 final cleanup and release proof after external native/runtime blockers are scheduled in platform CI.
