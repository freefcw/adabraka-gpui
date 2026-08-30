# Hai TDD: Public macro crate-path compatibility

## Target Behavior
Public GPUI derives and test/action macros compile when consumers use the documented `gpui` crate name or rename the dependency in Cargo.

## RED
- **Test added**: macro integration tests switched to dependency key `gpui`; `tests/downstream-compat`; `tests/downstream-renamed-compat`
- **Behavior asserted**: generated code names the consumer-visible GPUI crate instead of hard-coding `adabraka_gpui` or `gpui`.
- **Command**: `cargo test -p adabraka-gpui-macros --test render_test --test derive_context --test derive_inspector_reflection`
- **Observed failure**: six compile errors reported unresolved `adabraka_gpui` from `Render`, `AppContext`, `VisualContext`, and inspector reflection.
- **Failure is correct because**: the tests exposed the library under its real name `gpui`; production macros generated a different, unavailable crate path.

## GREEN
- **Minimal implementation**: added one `proc-macro-crate` resolver; migrated public `Render`, `AppContext`, `VisualContext`, `Action`, `IntoElement`, `register_action!`, inspector reflection, and `#[gpui::test]` expansions to the resolved path; mapped the default package key to lib target `gpui`.
- **Command**: `cargo test --offline -p adabraka-gpui-macros -p adabraka-gpui-renamed-dependency-compat`
- **Observed pass**: five tests passed across six suites, including execution of a renamed `#[ui::test]` test.

## REFACTOR
- **Refactor done**: yes
- **Change**: centralized path resolution in `gpui_crate_path`; marked style-generation proc macros internal because they are only invoked by GPUI itself; added default and renamed downstream workspace fixtures.
- **Command after refactor**: `cargo check --offline --workspace`; `cargo test --offline -p adabraka-gpui --lib --features test-support -- --test-threads=1`
- **Observed result**: workspace check passed; 204 core tests passed and 2 remained ignored.

## Next Behavior

The published compatibility package keeps `Application::new/headless` while construction can be injected through `Application::with_platform`.

## Windows Extraction Evidence

### GREEN
- **Implementation**: moved the Win32/DirectX backend, HLSL, resources, manifest, and build script to `adabraka-gpui-windows`; switched `gpui-platform` to the external selector; removed concrete Windows code and renderer dependencies from core.
- **Commands**: Zig-backed `cargo check -p adabraka-gpui-windows --target x86_64-pc-windows-msvc --no-default-features`; public `adabraka-gpui` no-default Windows check.
- **Observed pass**: both no-default checks type-check; host workspace/core/umbrella/macro/fixture/all-target checks also pass.

### Boundary Refactor
- **Changes**: replaced cross-crate orphan conversions with backend-local functions, exposed only narrow hidden core constructors/conversions, and kept `Pixels` storage private.
- **Behavior preserved**: clipboard metadata/image IDs, DirectWrite font mapping, atlas allocation, DirectX readback, HLSL shader selection, manifest embedding, and product integrations remain in the moved backend.

## Windows Screen Capture Dependency Repair

### Target Behavior
The public `screen-capture` feature builds for the Windows target with the same capture API version expected by `zed-scap`.

### RED
- **Test added**: none. This is dependency configuration, so the existing public feature compilation seam is the RED check.
- **Behavior asserted**: a downstream Windows application can enable every documented GPUI feature, including screen capture.
- **Observed failure**: registry `zed-scap` called `windows-capture 1.5`'s eight-argument `Settings::new` with the older five-argument form.
- **Failure is correct because**: compilation stopped in the enabled feature's transitive dependency before GPUI, proving the advertised feature was not buildable.

### GREEN
- **Minimal implementation**: added a root `[patch.crates-io]` entry for Zed's compatible `windows-capture` revision and updated only that lockfile node to 1.4.3.
- **Command**: the public all-feature Windows check above and `cargo check -p adabraka-gpui-windows --target x86_64-pc-windows-msvc --all-features`, both with the Zig wrappers.
- **Observed pass**: both target checks pass; `cargo tree` resolves `zed-scap -> windows-capture 1.4.3` from the pinned Zed revision.

### REFACTOR
- **Refactor done**: no production refactor was needed. Added `raw_metadata_preserves_the_clipboard_text` as tests-after coverage for the already-created cross-crate clipboard seam.
- **Command after refactor**: `cargo test -p adabraka-gpui-core`.
- **Observed result**: 195 tests passed.

### Superseded
A workspace `[patch.crates-io]` entry never reaches a published manifest, so the repair held only
inside this checkout. It is replaced by a `windows-capture = ">=1.3.6, <1.5"` requirement on
`adabraka-gpui-core`'s `screen-capture` feature, which travels with the published crate. The cost is
Zed's unpublished `Monitor::name` display detection fix, for which `zed-scap` already falls back to
the GDI device name.

### Next Behavior
Native Windows screen-capture and desktop-integration smoke still require a Windows host.
