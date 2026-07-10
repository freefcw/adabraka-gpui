# Layer-shell migration and implementation provenance

Adabraka GPUI `0.7.0` changes the public layer-shell API from `0.6.x`. This document is the
project-maintained source of truth for upgrading callers and for distinguishing the
upstream-inspired implementation from Adabraka-specific follow-up changes.

## Migrating from 0.6.x

### Select the window kind explicitly

Layer-shell configuration is no longer a separate optional field on `WindowOptions`.

Before:

```rust
let options = WindowOptions {
    layer_shell: Some(LayerShellOptions {
        // ...
        ..LayerShellOptions::default()
    }),
    ..WindowOptions::default()
};
```

After:

```rust
let options = WindowOptions {
    kind: WindowKind::LayerShell(LayerShellOptions {
        namespace: "my-panel".to_string(),
        anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        margin: Some((px(0.0), px(0.0), px(0.0), px(0.0))),
        ..LayerShellOptions::default()
    }),
    ..WindowOptions::default()
};
```

`WindowKind::Overlay` remains a regular platform overlay. It no longer implicitly creates a
Wayland layer-shell surface; callers that require layer-shell semantics must select
`WindowKind::LayerShell` explicitly.

### Replace removed APIs

| Removed `0.6.x` API | `0.7.0` replacement |
|---|---|
| `WindowOptions::layer_shell` | Set `WindowOptions::kind` to `WindowKind::LayerShell(options)`. |
| `LayerShellProtocolPreference` | No replacement. The backend supports `wlr-layer-shell` only. |
| `LayerShellOptions::tray_panel` | Build `LayerShellOptions` explicitly and calculate the required `anchor` and `margin` at the call site. |
| `LayerShellOptions::from_window_bounds` | Build `LayerShellOptions` explicitly and translate the desired bounds into `anchor` and `margin` at the call site. |
| Implicit layer-shell behavior from `WindowKind::Overlay` | Use `WindowKind::LayerShell` explicitly. |

The runtime-aware conversion used by the repository is available in
[`window_positioning.rs`](../crates/gpui/examples/window_positioning.rs). It selects layer-shell
only when the active compositor is Wayland and otherwise uses a supported non-layer-shell window
kind.

### Runtime behavior in 0.7.0

- Only `wlr-layer-shell` is supported. There is no automatic `ext-layer-shell` fallback.
- Opening a layer-shell window on X11, or on a Wayland compositor without `wlr-layer-shell`,
  returns `LayerShellNotSupportedError` instead of silently creating a normal window.
- A requested width or height of zero is sent unchanged so the compositor can choose that
  dimension. For example, zero width with left and right anchors can request a full-width panel.
- `exclusive_edge` requires wlr-layer-shell version 5. On older protocol versions it is ignored
  with a warning and the compositor infers the edge from the anchors.
- The `WindowKind::LayerShell` variant is compiled only on Linux or FreeBSD when the `wayland`
  feature is enabled.

The current public options are defined in
[`platform/layer_shell.rs`](../crates/gpui/src/platform/layer_shell.rs). The Wayland and X11
boundaries are implemented in
[`wayland/window.rs`](../crates/gpui/src/platform/linux/wayland/window.rs) and
[`x11/window.rs`](../crates/gpui/src/platform/linux/x11/window.rs).

## Implementation provenance

The original migration commit combined an upstream-inspired API port with local compatibility and
documentation work. Because that commit is already published on `main`, its history is not
rewritten. The table below is the authoritative scope record for auditing the resulting behavior.

| Commit | Classification | Scope |
|---|---|---|
| `7dea49e` | Mixed upstream port and local adaptation | Introduced the `WindowKind::LayerShell` model, migrated the removed local APIs, added an `ext-layer-shell` fallback, and updated examples and documentation. Its commit message says it was ported from Zed `b92664c52d`; the Zed object is not stored in this repository, so that statement is provenance supplied by the commit message rather than a locally verifiable byte-for-byte equivalence. This commit must not be treated as evidence that every contained behavior came from upstream. |
| `8667443` | Adabraka corrective follow-up | Removed the unsafe `ext-layer-shell` fallback, made the backend wlr-only, preserved zero-size requests, rejected layer-shell on X11, and made the positioning example choose by runtime compositor. |
| `c89cc51` | Adabraka regression coverage | Added tests for zero-size configure behavior, protocol mappings, the X11 support boundary, and runtime compositor selection. |
| `e705511` | Adabraka build-boundary fix | Avoided layer-shell validation warnings in X11-only builds. |
| `1c067ae` | Adabraka cleanup | Removed legacy protocol abstractions and aligned internal names with the wlr-only implementation. |

The upstream hash named by `7dea49e` is
[`zed@b92664c52d`](https://github.com/zed-industries/zed/commit/b92664c52d). Future upstream ports
should keep the upstream-derived change separate from Adabraka-specific adaptation whenever the
changes can be reviewed independently, and should record the full upstream hash in the commit
message or the relevant file under `docs/sync/`.
