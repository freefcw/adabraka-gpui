# Goal Document: Migration Maintenance Hardening

## Go / No-Go

- **Judgment**: Go
- **Reason**: The crate split is already complete and the current workspace is green, but the old verification commands can pass without running core tests. The repair is bounded to test entry points, CI, release checks, and current documentation.

## Target Outcome

Upstream GPUI changes can be synchronized and released without silently skipping core tests, using one current document for crate/path mapping, known divergences, and valid verification commands.

## Goal Definition

- **Type**: quality and delivery
- **Boundary**: Repair the four verification scripts, strengthen the platform CI matrix, add a canonical migration/release verification entry point, correct active README commands and links, and establish one current sync document.
- **Non-goals**:
  - Migrating the upstream scheduler, structured notifications, native popup, web, mobile, or View APIs.
  - Reorganizing Cargo features or moving platform source files again.
  - Cleaning unrelated Clippy warnings.
- **Deferred work**:
  - Production task-priority semantics until profiling shows a user-visible scheduling problem.
  - Structured notification actions and parent-anchored popups until an application needs them.
- **Verification rule**: Focused commands must execute tests from `adabraka-gpui-core`; compatibility tests must execute from `adabraka-gpui`; the workspace suite, three-platform CI definitions, formatting, package inventories, and documentation links must remain valid.
- **Evidence source**: Cargo command output, shell syntax checks, workflow review, package inventories, and repository searches for stale active commands.
- **Pass criteria**: Core focused tests no longer report zero tests, the full workspace remains green, active docs identify the split topology and current upstream baseline, and the public package keeps its existing API fixtures.
- **Confidence note**: Local execution directly covers macOS and all platform-independent tests. Linux and Windows runtime confidence remains owned by their CI runners.
- **Judgment owner**: Automated commands and final diff review.

## Current State

- `adabraka-gpui` is a compatibility facade; `adabraka-gpui-core` owns 195 current unit tests.
- The four `scripts/verify-*.sh` files still target the facade for core test filters, so commands can exit successfully with zero tests.
- A macOS `--features wgpu` facade check does not include `adabraka-gpui-wgpu` in its dependency tree.
- Sync index, mapping, quick reference, README commands, and example paths still contain pre-split guidance.
- The workspace currently passes formatting, checking, and 277 tests with 2 ignored.

## Plan Rewrite Notes

| Existing item | Decision | Reason |
|---|---|---|
| Fix verification scripts and CI | Keep and move first | It closes a release-safety hole with direct evidence. |
| Create a machine-readable migration ledger | Rewrite | A single maintained Markdown status document is sufficient for the current team size. |
| Simplify Cargo feature forwarding | Remove from this goal | No user-facing failure or release blocker is proven. |
| Complete scheduler migration | Defer | Current priority behavior is intentionally test-only and has no measured production need. |
| Migrate notifications and native popup | Defer | Require an application-level trigger before adding behavior. |
| Enforce Clippy zero warnings | Remove from this goal | Useful cleanup, but unrelated to migration correctness. |

## Drift Diagnosis

- **Goal drift**: Earlier proposals mixed release correctness with optional upstream capability work.
- **Phase drift**: Verification, documentation, feature cleanup, and scheduler design were presented at the same priority.
- **Validation drift**: Commands were preserved after package ownership changed, so a successful exit no longer proved core behavior.
- **Compatibility drift**: The facade contract is tested, but active docs still describe the old package layout.
- **Cleanup drift**: Warning cleanup and source flattening do not prove safer synchronization or release.

## Priority Rationale

- Repair test ownership first because every later upstream migration depends on trustworthy evidence.
- Update current documentation second so future work starts from the real crate graph and baseline.
- Add release/API checks last, after the canonical verification commands are stable.

## Assumptions and Open Decisions

| Item | Status | Impact | Owner / Next step |
|---|---|---|---|
| Existing public package and startup API remain unchanged | Confirmed | No downstream migration is required | Keep compatibility fixtures green |
| Historical reports should remain available | Confirmed | Provenance is preserved | Mark them historical rather than rewriting their past claims |
| `cargo-semver-checks` is installed locally | Not confirmed | It cannot be a mandatory local baseline command | Make it an explicit release-only prerequisite |
| Linux and Windows runners execute repository CI | Assumed | Native validation is external | Keep platform jobs in the workflow matrix |

## Phases

### Phase 1: Restore trustworthy verification

- **Purpose**: Ensure every named core test command runs against the crate that owns the tests.
- **Entry condition**: Clean worktree and observed zero-test failure on the old facade command.
- **Phase rules**:
  - Do not change GPUI runtime behavior.
  - Keep public compatibility tests separate from core unit tests.
  - A WGPU check must name the WGPU package directly.
- **Todos**:
  - [x] Centralize package-aware Cargo commands used by the four verification scripts.
    - **Surface**: `scripts/verify-common.sh`, `scripts/verify-001.sh` through `verify-004.sh`
    - **Proof**: Focused profiler and scheduler commands execute nonzero core tests.
    - **Depends on**: none
  - [x] Make default CI entries execute workspace tests on Linux, macOS, and Windows.
    - **Surface**: `.github/workflows/gpui-feature-matrix.yml`
    - **Proof**: Workflow commands include `cargo test --workspace --lib --tests` while no-default feature checks remain.
    - **Depends on**: corrected package ownership
- **Exit proof**: Shell syntax checks pass; focused core tests and workspace tests pass.
- **Stop condition**: A corrected command requires changing public or runtime behavior.

### Phase 2: Establish the current synchronization source of truth

- **Purpose**: Prevent future patches from using obsolete paths, package ownership, or validation commands.
- **Entry condition**: Phase 1 commands are stable.
- **Phase rules**:
  - Preserve historical reports as historical evidence.
  - Only `docs/sync/CURRENT.md` may describe the current executable workflow.
  - Record intentional divergences without turning them into required work.
- **Todos**:
  - [x] Add the current upstream baseline, crate/path mapping, divergence decisions, and validation commands.
    - **Surface**: `docs/sync/CURRENT.md`
    - **Proof**: Every extracted GPUI crate has an upstream-to-local mapping.
    - **Depends on**: Phase 1 commands
  - [x] Redirect the sync index and old operational guides to the current document.
    - **Surface**: `docs/sync/README.md`, legacy mapping/quick/technical guides
    - **Proof**: Readers encounter the current-state warning before obsolete instructions.
    - **Depends on**: current document
  - [x] Correct active README test commands and example paths.
    - **Surface**: root and packaged compatibility README
    - **Proof**: Referenced files exist and commands execute the owning package.
    - **Depends on**: Phase 1 commands
- **Exit proof**: Searches find no stale paths or facade-owned core test commands in active docs.
- **Stop condition**: A historical claim must be rewritten to make the current guide work.

### Phase 3: Add release proof

- **Purpose**: Verify publishable package contents and public API compatibility before release.
- **Entry condition**: Phases 1 and 2 are green.
- **Phase rules**:
  - Do not make unavailable optional tooling part of ordinary development checks.
  - Keep semantic-version checking release-only and explicit.
- **Todos**:
  - [x] Add a release verification script for workspace tests, downstream fixtures, package inventories, and optional semantic API checking.
    - **Surface**: `scripts/verify-migration.sh`
    - **Proof**: The script passes through all locally available mandatory gates and clearly reports the optional tool prerequisite.
    - **Depends on**: Phase 1 helpers
- **Exit proof**: The canonical migration verification script passes locally, except an explicitly optional semver check when the tool is absent.
- **Stop condition**: Release verification requires registry credentials or publishing artifacts.

## Dry-Run Findings

- A shared script is preferable to replacing package names independently in four files; it leaves one owner for future crate-layout changes.
- `cargo test --workspace` protects broad behavior, but focused scripts still need explicit core ownership so filters cannot silently target the facade.
- Semantic API checking cannot be mandatory locally because `cargo-semver-checks` is not installed; compile fixtures remain the mandatory compatibility proof.
- Historical audit documents should not be rewritten because their dated evidence was accurate before the split.

## Final Validation

- `bash -n scripts/verify-common.sh scripts/verify-001.sh scripts/verify-002.sh scripts/verify-003.sh scripts/verify-004.sh scripts/verify-migration.sh`
- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo test --workspace --lib --tests --locked -- --test-threads=1`
- `cargo test -p adabraka-gpui-core --lib --features test-support --locked -- profiler`
- `cargo test -p adabraka-gpui --tests --features test-support --locked`
- `git diff --check`

## Completion Evidence

- The previous facade-owned profiler command ran zero tests; `test_core` now rejects filters that match no core tests, while `test_core profiler` runs 12 tests.
- `scripts/verify-001.sh` through `scripts/verify-004.sh` all pass and every focused filter matches at least one core test.
- `scripts/verify-migration.sh` passes the core, public compatibility, and workspace tests, verifies the normalized macro archive, and checks package inventories for all eight migration packages.
- Workflow YAML parsing, formatting, workspace checking, shell syntax, and `git diff --check` pass locally.
- The optional semantic API check is intentionally not executed because `cargo-semver-checks` is not installed; `--semver` now fails before running other work and prints the installation command.

## First Execution Step

Add the shared package-aware verification helper and prove that the former zero-test profiler command now executes core tests.
