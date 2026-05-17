# Adabraka GPUI 与 Zed GPUI 上游差异分析

分析日期：2026-05-17（本地环境 Asia/Shanghai）

## 结论摘要

当前仓库已经不再是 Zed `crates/gpui` 的简单抽取版。Adabraka 在保留 GPUI 核心 API 的同时，把 daemon、托盘、全局热键、原生通知、自动启动、单实例、系统信息、窗口定位、资源 profile、Linux wgpu 后端等能力内置到 `adabraka-gpui`。上游 Zed 则在 `gpui-v0.2.2` 之后继续快速演进，并在 2026-02-19 起把原 GPUI 平台层拆分到多个 `gpui_*` crate。

这次比对的主要结论：

- 当前仓库基线仍接近上游 `gpui-v0.2.2`，但已经人工同步过一批 2026-04-30 前后的关键修复，例如 GIF frame index 越界、Anchored 负坐标尺寸、Wayland 后台 frame complete、macOS glyph dilation、SVG emoji/BGRA/fallback、SharedString smol_str、部分 hover/text 修复。
- 上游 2026-05 月新增的若干 bugfix 仍明显缺失，下一步优先处理：cached view + Input 崩溃、ListState scrollbar 拖拽/streaming 滚动稳定性、Wayland 鼠标点击复制、Wayland redundant commit flicker、Windows monitor disappearing panic、DirectX atlas device recovery panic。
- 上游拆分后的平台修复不能只看 `/zed/crates/gpui`。原来属于 GPUI 平台层的代码现在分布在 `gpui_linux`、`gpui_macos`、`gpui_windows`、`gpui_wgpu`、`gpui_platform`、`gpui_shared_string`。下一步同步 bugfix 时必须把这些 crate 纳入来源范围，再映射回当前仓库的 `crates/gpui/src/platform/*`。
- 目录级 diff 噪音很大，不适合直接作为合并依据。本次 `git diff --no-index --shortstat /zed/crates/gpui/src crates/gpui/src` 显示 `160 files changed, 54004 insertions(+), 11912 deletions(-)`，主要原因是 Adabraka 内嵌平台层而上游拆分平台层，并不代表所有差异都是可合并修复。

## 对比范围和基线

本次只读取本地两个仓库，没有访问远程网络。

| 项目 | 路径 | HEAD / 基线 |
| --- | --- | --- |
| Adabraka GPUI | `/Users/hejun/codespace/my/agenttray/adabraka-gpui` | `3153bfa20bf8df3558c62d44c2ed44ef8eb51890`，2026-05-11，`docs: add resource profile documentation` |
| Zed 上游 | `/Users/hejun/codespace/my/agenttray/zed` | `3bd9d13b63fc5a5ffa39326597bc4fd91adc82d1`，2026-05-16，`settings: Fix inverted VS Code import for files.simpleDialog.enable (#55678)` |
| Zed GPUI 发布基线 | `/Users/hejun/codespace/my/agenttray/zed` tag `gpui-v0.2.2` | `69e2130295c2649963eb639fc70b4f2ee8ea1624`，2025-10-21，`gpui 0.2.2` |

Adabraka `crates/gpui/Cargo.toml` 当前包名为 `adabraka-gpui`，版本 `0.6.2`。Zed `crates/gpui/Cargo.toml` 当前包名仍为 `gpui`，版本 `0.2.2`，但代码已经依赖拆分后的 `gpui_platform`、`gpui_shared_string`、`gpui_util` 等 workspace crate。

## 结构差异

### 当前仓库独有或内嵌的模块

当前仓库相对 `/zed/crates/gpui/src` 独有的主要文件包括：

- `src/elements/toast.rs`
- `src/resource_profile.rs`
- `src/shared_string.rs`
- `src/platform/linux/**`
- `src/platform/mac/**`
- `src/platform/windows/**`
- `src/platform/wgpu/**`
- `src/platform/single_instance.rs`
- `src/platform/window_positioner.rs`

这些差异大多是 Adabraka 的产品方向：后台应用、托盘、全局热键、系统 API、窗口定位、资源预算，以及把 Zed 后来拆走的平台层继续保留在单 crate 内。

### Zed 上游新增但当前仓库没有的核心文件

Zed `crates/gpui/src` 相对当前仓库新增或保留的主要文件：

- `src/app/headless_app_context.rs`
- `src/app/test_app.rs`
- `src/app/visual_test_context.rs`
- `src/platform/layer_shell.rs`
- `src/platform/visual_test.rs`
- `src/platform_scheduler.rs`
- `src/profiler.rs`
- `src/queue.rs`

其中 `platform/layer_shell.rs` 的能力在 Adabraka 已经用 `platform/linux/wayland/{ext_layer_shell,wlr_layer_shell}.rs` 和 `WindowOptions::layer_shell` 做过本地实现，不应直接覆盖。`platform_scheduler.rs`、`queue.rs`、`Task/scheduler` 相关提交属于上游较大架构变化，单独 cherry-pick 风险较高。

### 上游平台拆分映射

Zed 现在把原 GPUI 平台层拆到了这些 crate：

| Zed 上游 crate | 当前仓库对应位置 | 同步方式 |
| --- | --- | --- |
| `crates/gpui` | `crates/gpui/src` 核心文件 | 可按文件/commit 手动合并 |
| `crates/gpui_linux` | `crates/gpui/src/platform/linux/**` | 需要路径映射后人工合并 |
| `crates/gpui_macos` | `crates/gpui/src/platform/mac/**` | 需要保护 Adabraka macOS 托盘/热键/权限扩展 |
| `crates/gpui_windows` | `crates/gpui/src/platform/windows/**` | 需要保护 Adabraka Windows 托盘/通知/热键扩展 |
| `crates/gpui_wgpu` | `crates/gpui/src/platform/wgpu/**` 和 Linux wgpu 接入点 | 当前仓库已有本地移植，逐项比对 |
| `crates/gpui_platform` | `crates/gpui/src/platform.rs` trait/API | 只同步必要 trait/类型变更 |
| `crates/gpui_shared_string` | `crates/gpui/src/shared_string.rs` | 当前已基本采用 smol_str 方案 |

## 核心文件差异矩阵

下面的行数统计来自 `git diff --no-index --numstat <zed-file> <adabraka-file>`，表示“把 Zed 当前文件变成 Adabraka 当前文件”时的新增/删除行。它不能直接代表功能优劣，但能帮助判断同步风险。

| 共享文件 | + / - | 差异性质 | 同步建议 |
| --- | ---: | --- | --- |
| `app.rs` | `821 / 448` | Adabraka 有 daemon、资源 profile、托盘/系统状态相关 API；上游有 entity/query/test context 演进 | 只按 commit 合并，不直接覆盖 |
| `window.rs` | `466 / 864` | 两边都频繁改；当前缺 cached Input、force-render refresh 等新修复 | P0 手动合并，合并前后跑核心 window tests |
| `platform.rs` | `986 / 739` | 当前保留完整平台 trait 和 Adabraka 扩展；上游把平台实现拆出 | 只同步 trait 上必要 bugfix，例如 `DisplayId` |
| `elements/list.rs` | `142 / 886` | 上游列表滚动修复和测试明显更多；当前缺 5 月 scrollbar 修复 | 作为单独高优先级同步批处理 |
| `elements/div.rs` | `230 / 688` | 上游有布局、hover、image cache、mask/overflow 相关演进 | 不建议整文件覆盖，按视觉 bugfix 筛 |
| `text_system.rs` | `162 / 334` | 当前有 resource profile/cache stats，本地与上游 text render 参数有差异 | 按 glyph/cache/trim 修复逐项合并 |
| `text_system/line.rs` | `83 / 421` | 上游 `ShapedLine::split_at`、decoration、font run 测试更多 | 中优先级文本专项 |
| `text_system/line_layout.rs` | `290 / 303` | 两边都有 combining/letter spacing 修改 | 保留当前已通过的 combining tests，逐项 diff |
| `text_system/line_wrapper.rs` | `96 / 291` | 上游 truncation API 已演进，当前已有部分 word-char 修复 | 避免直接覆盖，先移植 UTF-8 panic 和 word wrap tests |
| `svg_renderer.rs` | `58 / 96` | 当前已有 SVG 字体/emoji 修复，但缺懒加载 enriched font DB | 低风险性能补丁可单独合并 |
| `scene.rs` | `227 / 162` | 渲染批处理和 atlas texture kind 差异；当前有 wgpu ABI 本地修复 | wgpu/atlas 专项里处理 |
| `style.rs` / `styled.rs` | `118 / 48`，`221 / 130` | 上游添加多个 builder、grid/aspect/text 相关 API；当前也有本地扩展 | 功能性 API 合并需看 semver |
| `keymap.rs` / `key_dispatch.rs` | `23 / 168`，`51 / 343` | 上游 keybinding/prefix/IME 处理更活跃 | 输入专项处理，不直接覆盖 |
| `elements/img.rs` | `141 / 109` | 当前已包含 GIF frame clamp；上游还有 animated image timing 等 | 中低优先级 |
| `elements/anchored.rs` | `51 / 63` | 当前已包含负坐标 union 修复；上游还有 center positioning 改进 | 非 bugfix 可暂缓 |

### 平台映射文件差异矩阵

这些文件在 Zed 上游已经位于 `gpui_linux`、`gpui_macos`、`gpui_windows`、`gpui_wgpu`，下表映射回当前仓库路径。

| 当前文件 | 对应上游文件 | + / - | 重点风险 |
| --- | --- | ---: | --- |
| `platform/linux/wayland/client.rs` | `gpui_linux/src/linux/wayland/client.rs` | `392 / 516` | Wayland clipboard serial、IME、selection、layer-shell/display_id |
| `platform/linux/wayland/window.rs` | `gpui_linux/src/linux/wayland/window.rs` | `812 / 509` | redundant commit flicker、ack_configure、device recovery、layer-shell 本地扩展 |
| `platform/linux/wayland/serial.rs` | `gpui_linux/src/linux/wayland/serial.rs` | `0 / 18` | 当前明显落后，缺 `SerialTracker::get_latest()` |
| `platform/linux/x11/client.rs` | `gpui_linux/src/linux/x11/client.rs` | `378 / 678` | XInput 版本、键盘状态同步、raw-window-handle、hotkey/IME |
| `platform/linux/x11/window.rs` | `gpui_linux/src/linux/x11/window.rs` | `450 / 322` | GPU recovery、window icon、override_redirect、Adabraka hotkey/window hints |
| `platform/windows/display.rs` | `gpui_windows/src/display.rs` | `84 / 33` | monitor disappearing panic，`DisplayId` 语义差异 |
| `platform/windows/window.rs` | `gpui_windows/src/window.rs` | `340 / 403` | device recovery、inactive click dispatch、window creation fallback |
| `platform/windows/events.rs` | `gpui_windows/src/events.rs` | `594 / 630` | foreground budget、IME、Direct Manipulation、本地托盘/热键消息 |
| `platform/windows/directx_atlas.rs` | `gpui_windows/src/directx_atlas.rs` | `30 / 40` | device recovery 后 stale atlas tile panic |
| `platform/windows/directx_renderer.rs` | `gpui_windows/src/directx_renderer.rs` | `414 / 514` | GPU reset、D3D resource resize、subpixel atlas |
| `platform/mac/window.rs` | `gpui_macos/src/window.rs` | `860 / 735` | cursor、deferred AppKit、display change、Adabraka overlay/tray |
| `platform/mac/platform.rs` | `gpui_macos/src/platform.rs` | `1604 / 394` | Adabraka 系统 API 扩展非常多，不能覆盖 |
| `platform/mac/text_system.rs` | `gpui_macos/src/text_system.rs` | `166 / 173` | glyph dilation 基本已同步，仍需核对 font smoothing defaults |
| `platform/wgpu/wgpu_atlas.rs` | `gpui_wgpu/src/wgpu_atlas.rs` | `20 / 20` | 格式 fallback、removed texture upload、device lost |
| `platform/wgpu/wgpu_context.rs` | `gpui_wgpu/src/wgpu_context.rs` | `17 / 55` | adapter feature guard、texture format selection |
| `platform/wgpu/wgpu_renderer.rs` | `gpui_wgpu/src/wgpu_renderer.rs` | `93 / 195` | draw 是否 present、surface lifecycle、resize/recover |

## 当前已确认包含的上游修复

下表基于源码特征验证，而不是只看提交信息。

| 上游 commit | 主题 | 当前状态 | 证据 |
| --- | --- | --- | --- |
| `b38194198b` | Anchored 子元素负坐标导致弹出层尺寸错误 | 已包含 | `crates/gpui/src/elements/anchored.rs` 使用 `reduce(|acc, bounds| acc.union(&bounds))` |
| `749fcfdfd8` | GIF 替换为较少帧时 `frame_index` 越界 panic | 已包含 | `crates/gpui/src/elements/img.rs` 对 `frame_index` 使用 `min(max_frame_index)` 并有回归测试 |
| `10122be9cb` | Wayland 后台窗口 frame throttle 后冻结 | 已包含 | `Window::new` request-frame 节流分支会调用 `window.complete_frame()` |
| `72eb842540` | 非 active 窗口限制到约 30 FPS | 已包含 | `window.rs` 中 inactive window 使用 `Duration::from_micros(33333)` |
| `58d3a9eef4` | `SharedString` 改用 `smol_str` | 已包含 | `crates/gpui/src/shared_string.rs` 以 `SmolStr` 为后端 |
| `a38fc8c8de` | macOS 基于亮度的 glyph dilation | 已包含 | `PlatformTextSystem::glyph_dilation_for_color`、`RenderGlyphParams::dilation`、`mac/text_system.rs` 已存在 |
| `d010b06a77` | macOS 光标 flicker / `resetCursorRects` | 基本已包含 | `mac/window.rs` 注册 `resetCursorRects`，`HitboxId::is_hovered_ignoring_last_input` 已存在 |
| `dbb8afe676` / `5197cb4da9` / `eaf14d028a` | SVG BGRA、emoji、字体 fallback | 大部分已包含 | `svg_renderer.rs` 有 emoji font selection、system font DB、`swap_rgba_pa_to_bgra` |
| `a7e677efa5` / `debf4c9988` / `55a59ca17d` | 文本 decoration、组合字符、word char 修复 | 多数已包含 | `line.rs` 有 final glyph decoration 修正，`line_layout.rs` 有 combining mark 测试，`line_wrapper.rs` 扩展 word char |
| `1623ad3`（Adabraka 本地） | captured hitbox 视为 hovered | 已包含 | `HitboxId::is_hovered` 和 `is_hovered_ignoring_last_input` 都检查 `captured_hitbox` |

## 高优先级：建议下一步合并的上游 bugfix

### P0. cached view + Input 崩溃

- 上游 commit：`a221a86d49`，2026-05-14，`Fix GPUI crash when using cached view with Input (#50665)`
- 上游文件：`crates/gpui/src/window.rs`
- 当前状态：缺失
- 当前证据：`crates/gpui/src/window.rs` 仍在 `Window::draw` 中把旧 input handler `push` 回 `rendered_frame.input_handlers`，并用 `self.next_frame.input_handlers.pop()` 取 handler。
- 问题：`pop()` 改变 `input_handlers` 长度，使 cached view 的 `paint_range.input_handlers_index` 失效；下一帧 `reuse_paint()` 用旧 range 切片时可能越界。
- 影响：Input 组件放在 cached view 内时，输入首字符或下一帧可能 panic。对组件库、Dock、复杂 overlay UI 风险高。
- 建议迁移：把上游 patch 手动应用到当前 `Window::draw`。核心是恢复旧 handler 时优先填回 `None` slot；注册新 handler 时用 `iter_mut().rev().find_map(|h| h.take())`，保留 vec 长度。
- 建议测试：新增或移植上游回归测试；至少运行 `cargo test -p adabraka-gpui --lib stale_frame_index_is_clamped_when_image_changes` 之外再覆盖 cached input 场景。

### P0. ListState streaming 内容增长时 scrollbar 拖拽错位

- 上游 commits：
  - `dfd8328f7b`，2026-05-06，`gpui: Fix material list unstable scrollbar position (#55808)`
  - `1c61cc3fc2`，2026-05-12，`gpui: Fix scrollbar drag position calculation in list (#53378)`
  - `51b43c90f9`，2026-05-12，`Add is_scrollbar_dragging() accessor`
- 上游文件：`crates/gpui/src/elements/list.rs`
- 当前状态：缺失
- 当前证据：当前 `StateInner.pending_scroll` 仍是 `Option<PendingScrollFraction>`，没有 `PendingScroll::Absolute` / `ScrollAnchor`；`scroll_px_offset_for_scrollbar` 仍计算 `drag_offset`；`set_offset_from_scrollbar` 仍用 `(point.y - drag_offset).abs()`。
- 问题：列表项 streaming 变高、滚动条拖拽和 follow-tail 同时发生时，scrollbar 映射使用 live height 与 frozen height 混用，可能跳动、反向、无法重新 follow tail。
- 影响：聊天、日志、agent 面板、长列表虚拟滚动。Adabraka 面向 tray/overlay 但仍暴露 `list`，影响下游 UI 稳定性。
- 建议迁移：先合并 `dfd8328f7b`，再合并 `1c61cc3fc2`，最后可选 `51b43c90f9`。注意当前仓库可能已有本地 list 改动，建议按函数手动移植并保留现有测试。
- 建议测试：移植上游 `test_remeasure_item_preserves_scroll_offset`、`test_scrollbar_drag_with_growing_content`、`test_follow_tail_reengages_after_scrollbar_drag_to_bottom_while_growing`。

### P0. Windows monitor 消失导致 panic

- 上游 commit：`f5945344cc`，2026-05-06，`gpui(windows): Fix unwrap panic when monitor goes missing (#55630)`
- 上游文件：`crates/gpui/src/platform.rs`、`crates/gpui_windows/src/display.rs`、`crates/gpui_windows/src/window.rs`
- 当前状态：缺失
- 当前证据：当前 `WindowsDisplay::new_with_handle`、`new_with_handle_and_id` 仍使用 `expect("unable to get monitor info")`，且 `display_id` 仍按 monitor enumeration index 表示。
- 问题：外接屏拔掉、系统 monitor 枚举短暂不一致时，`GetMonitorInfoW` 或 `position(...).unwrap()` 会 panic。
- 影响：Windows 笔记本外接屏、远程桌面、显示器睡眠/恢复。Adabraka 的后台/托盘应用尤其容易长期运行并遇到显示拓扑变化。
- 建议迁移：把 `DisplayId` 从枚举 index 改为稳定的 `HMONITOR` raw value（上游改为 `u64`），`WindowsDisplay::new` 返回 `Option` 并在窗口创建时 fallback 到 primary monitor。该改动触及 public-ish `DisplayId` 类型，需检查序列化、窗口定位和 tray anchor 的使用。

### P0. DirectX atlas 在 GPU device recovery 后 panic

- 上游 commit：`7d19e89988`，2026-05-07，`Fix DirectX atlas panic after GPU device recovery (#55878)`
- 上游文件：`crates/gpui/src/window.rs`，并与 `gpui_windows` device recovery 路径协同
- 当前状态：部分缺失
- 当前证据：当前 `Window::new` request-frame 分支没有在 `request_frame_options.force_render` 时调用 `window.refresh()`；Windows `DirectXAtlas::texture` 仍直接 `textures[id.index].unwrap()`。
- 问题：GPU device lost 后 atlas 被清空，但 cached view 的 `Scene::replay` 会重放旧 `AtlasTile`，渲染时访问不存在的 texture index。
- 影响：Windows suspend/resume、驱动重置、显示重配置，尤其 Intel iGPU。
- 建议迁移：先合入跨平台 `if request_frame_options.force_render { window.refresh(); }`，再检查当前 Windows device lost 是否有 `force_render_after_recovery` 等价机制。该跨平台改动也能强化 Linux wgpu recovery。

### P1. Wayland 鼠标点击触发复制时 clipboard serial 选择错误

- 上游 commit：`61e23fdb51`，2026-05-15，`Fix text copy via mouse click on Wayland (#50406)`
- 上游文件：`crates/gpui_linux/src/linux/wayland/client.rs`、`serial.rs`
- 当前状态：缺失
- 当前证据：当前 `write_to_primary` / `write_to_clipboard` 使用 `state.serial_tracker.get(SerialKind::KeyPress)`，`SerialTracker` 没有 `get_latest()`。
- 问题：首次通过鼠标按钮触发 copy 时没有 KeyPress serial，Wayland compositor 会拒绝 `set_selection`。
- 影响：Wayland 下按钮复制、右键菜单复制、托盘弹窗里的复制动作。
- 建议迁移：给 `SerialTracker` 加 `get_latest()`，`write_to_primary` 和 `write_to_clipboard` 改用最近 serial，而不是只取 KeyPress。

### P1. Wayland CPU load 下 redundant surface commit 导致 flicker

- 上游 commit：`923f315f26`，2026-05-06，`gpui_linux: Fix Wayland flickering under CPU load by skipping redundant surface commit (#54214)`
- 上游文件：`crates/gpui_linux/src/linux/wayland/window.rs`
- 当前状态：缺失
- 当前证据：当前 `WaylandWindowState` 没有 `renderer_presented` 字段；`draw` 直接 `state.renderer.draw(scene)`；`completed_frame()` 仍自行 `surface.commit()`。
- 问题：wgpu/Vulkan present 已经提交 Wayland surface，GPUI 再次 commit 可能在 wlroots/Sway + CPU load 下触发无 buffer commit 的竞态，造成 flicker/graphical corruption。
- 影响：Linux Wayland 是当前仓库 README 强调的 wgpu 后端路径，应优先修。
- 建议迁移：让 `renderer.draw(scene)` 返回或暴露是否实际 present；若 present 过，`completed_frame()` 跳过额外 `surface.commit()`。当前 Adabraka 的 wgpu renderer API 可能和上游略有差异，需沿 `platform/wgpu/wgpu_renderer.rs` 到 Wayland window 手动接线。

### P1. Windows / macOS 光标和输入状态后续修复

- 上游 commits：
  - `a03729b6c0`，2026-05-04，`Handle hiding cursor on keyboard input at GPUI level (#55664)`
  - `c01671eac1`，2026-04-29，`Restore mouse cursor on window deactivation (#55155)`
  - `1eba1ca72e`，2026-04-30，`Fix cursor style changes across windows (#55323)`
- 当前状态：部分包含
- 当前证据：当前已有 `last_input_was_keyboard`、`resetCursorRects`、`is_hovered_ignoring_last_input`，但还需要与上游 5 月后续 cursor 逻辑逐项 diff。
- 影响：键盘输入时隐藏光标、窗口失活恢复光标、多窗口切换 cursor style 混乱。
- 建议迁移：先解决 P0/P1 崩溃和 Wayland，再抽取上游 cursor 系列 commit 对 `Window` 与 `mac/window.rs` 的差异，避免重复改同一区域。

## 中优先级：建议评估后合并

### SVG renderer 懒加载 enriched font DB

- 上游 commit：`bf102668be`，2026-04-08，`gpui: Lazy-init font DB in SvgRenderer to avoid per-test overhead (#53381)`
- 当前状态：缺失
- 当前证据：当前 `SvgRenderer::new` 每次都会 clone system font DB、`load_bundled_fonts`、`fix_generic_font_families`；上游改为 `OnceLock` 在首次 SVG render 时构建。
- 影响：创建 `SvgRenderer` 但不实际渲染 SVG 的测试和应用路径有额外开销。
- 建议：低风险性能优化，可单独合并。注意当前 `load_bundled_fonts` 签名和上游不同，上游需要 `asset_source` 参数，当前函数可能已经本地改造过。

### WGPU atlas / device / format 稳定性补丁

- 相关上游 commits：
  - `dc3b25a4f4`，2026-04-13，unsupported atlas texture formats fallback
  - `e4ebd3aae5`，2026-04-03，screen share 时 WgpuAtlas crash
  - `924ac5c99b`，2026-03-31，device feature mismatch guard
  - `dbd95ea742`，2026-03-25，Linux GPU recovery 后强制 scene rebuild
  - `69d6bfd789`，2026-02-27，resize 中避免 wgpu panic
  - `2757aa4140`，2026-02-27，clamp window size on wgpu
- 当前状态：部分包含、需要逐项 diff
- 当前证据：当前 `platform/wgpu` 是 Adabraka 自己移植并已有 `fix(wgpu)` 系列本地提交；Wayland/X11 window 也已有 `force_render_after_recovery` 和 atlas recovery 注释。
- 建议：不要批量覆盖。按上游 commit 对 `crates/gpui_wgpu/src` 与当前 `crates/gpui/src/platform/wgpu/**` 做函数级 diff，优先合并 crash guard、format fallback、recovery scene rebuild。

### 文本系统后续修复

- 相关上游 commits：
  - `43d6ab5386`，2026-04-23，减少 `ShapedLine::split_at` SharedString 分配
  - `2c237899f9`，2026-04-22，分号换行归属前一个词
  - `3ed6c68f3b`，2026-01-15，`truncate_line` UTF-8 slicing panic
  - `97b429953e`，2025-11-19，不跨 styled text run 渲染 ligatures
  - `d1d419b209`，2025-12-01，进一步修正 font run extraction
- 当前状态：部分包含
- 当前证据：当前已有 combining mark / underline 相关测试，但 `line_wrapper.rs` 与上游 2026-05 的 truncation API 已有较大差异。
- 建议：对 `text_system/line.rs`、`line_wrapper.rs`、`line_layout.rs` 单独开一批同步任务；每个 commit 带测试迁移。

### Linux X11 / Wayland 输入修复

- 相关上游 commits：
  - `6a3111de79`，X11 keyboard state synchronization
  - `23830d5946`，XInput < 2.4 startup crash
  - `d92f47746f`，Wayland/Sunshine duplicate IME input
  - `608185be4e`，X11Window raw-window-handle traits
  - `007aba89bf`，throttled Wayland resize 仍 ack_configure
- 当前状态：部分包含或需人工核对
- 建议：这些更偏平台兼容性，Adabraka 的 Linux wgpu/layer-shell 路径比较活跃，建议在 P0/P1 后作为 Linux 平台专项处理。

## 低优先级或高风险：不建议现在直接合并

| 上游主题 | 原因 |
| --- | --- |
| `bc31ad4a8c` / `be705e677b`：抽出 `gpui_platform`、合并 `gpui::Task` 与 `scheduler::Task` | 架构级拆分，影响 crate 边界、Task 类型和 public API，不适合作为 bugfix cherry-pick |
| `7d42f276f2`：Pixel snapping | 渲染质量收益大，但触及 layout、style、taffy、window，多平台视觉回归风险高 |
| `af8ea0d6c2`：上游移除 Blade 并用 wgpu 重写 Linux renderer | 当前仓库已有独立 wgpu 移植，不能重复按上游大 patch 覆盖 |
| `14f37ed502`：GPUI on web / wasm 支持 | 与当前桌面/daemon 目标关系较弱 |
| `73d935330e`：scheduler 集成 GPUI | 与上游 Task 架构绑定，需先做 API 兼容设计 |
| 大量 Zed 应用层 commits | 触及 `agent_ui`、`editor`、`project_panel` 等，不属于独立 GPUI 仓库可直接应用范围 |

## Adabraka 独有功能差异

这些功能上游 Zed GPUI 没有等价实现，后续同步必须保护：

- daemon / keep-alive without windows
- 系统托盘、托盘菜单、托盘 anchor、tray panel mode
- 全局热键
- 原生通知和 toast
- overlay / click-through window
- window show/hide
- auto launch
- single instance
- focused window info
- macOS accessibility / microphone permission query
- 电源事件、网络状态、媒体键、原生对话框、任务栏/Dock progress、用户注意力请求、idle time、sleep inhibitor、context menu、OS info、biometric
- `WindowPosition` / `TrayAnchor` / layer-shell popup 定位扩展
- `AppResourceProfile`、GPU cache trimming、resource budget 配置
- Linux wgpu 后端在当前仓库中的单 crate 版本

合并上游 `platform.rs`、`app.rs`、`window.rs`、平台 window/client 文件时，应先 grep 这些关键词，避免覆盖 Adabraka API 和平台实现。

## 下一步建议顺序

1. 建立一个“上游修复候选”分支，只处理 P0/P1，不做架构拆分。
2. 第一批合并：
   - `a221a86d49` cached Input crash
   - `dfd8328f7b` + `1c61cc3fc2` ListState scrollbar fixes
   - `61e23fdb51` Wayland clipboard serial
3. 第二批合并：
   - `f5945344cc` Windows monitor disappearing
   - `7d19e89988` DirectX atlas device recovery cross-platform `force_render => refresh`
   - `923f315f26` Wayland redundant commit flicker
4. 第三批专项：
   - wgpu atlas/device recovery patch set
   - Linux input/IME/XInput compatibility patch set
   - text/SVG performance and correctness patch set
5. 每批合并后至少运行：
   - `cargo test -p adabraka-gpui --lib --features test-support`
   - Linux 路径：`cargo test -p adabraka-gpui --lib --features test-support,wayland,x11`
   - Windows/macOS 平台改动需要对应平台 smoke test，因为本地非目标平台无法覆盖 native event loop 和 GPU device recovery。

## 附录：上游修复候选池

这个表按“对独立 GPUI 库可能有价值”筛选，排除了明显只属于 Zed 应用层的改动。状态不是最终结论，下一步仍需按源码特征复核。

| 优先级 | Commit | 日期 | 主题 | 当前判断 |
| --- | --- | --- | --- | --- |
| P0 | `a221a86d49` | 2026-05-14 | cached view + Input 崩溃 | 缺失，直接影响 panic |
| P0 | `dfd8328f7b` | 2026-05-06 | streaming list 保持绝对 scroll offset | 缺失，需先于后续 scrollbar 修复 |
| P0 | `1c61cc3fc2` | 2026-05-12 | scrollbar drag position calculation | 缺失，需和上条成组 |
| P0 | `f5945344cc` | 2026-05-06 | Windows monitor disappearing unwrap panic | 缺失，平台长期运行高风险 |
| P0 | `7d19e89988` | 2026-05-07 | DirectX atlas device recovery panic | 部分缺失，需合入 force-render refresh |
| P1 | `61e23fdb51` | 2026-05-15 | Wayland 鼠标点击复制失败 | 缺失，低改动高收益 |
| P1 | `923f315f26` | 2026-05-06 | Wayland redundant commit flicker | 缺失，需 wgpu renderer 返回 present 状态 |
| P1 | `a03729b6c0` | 2026-05-04 | GPUI 层处理键盘输入隐藏光标 | 部分包含，需逐项 diff |
| P1 | `1eba1ca72e` | 2026-04-30 | 多窗口 cursor style 切换 | 部分包含，macOS 专项 |
| P1 | `c01671eac1` | 2026-04-29 | 窗口失活恢复鼠标光标 | 部分包含，核对平台实现 |
| P1 | `aa16a3bf9d` | 2026-05-11 | macOS font smoothing defaults 非数字值 | 值得核对，和 glyph dilation 同区 |
| P1 | `5c0b33f72e` | 2026-05-07 | Windows 避免 process-wide priority elevation | 值得核对，影响后台/daemon 友好性 |
| P1 | `38270bc027` | 2026-04-18 | Windows Alt-Tab 后 Alt modifier stuck | 值得合并，输入状态修复 |
| P1 | `6a3111de79` | 2026-04-16 | X11 keyboard state synchronization | Linux 输入专项 |
| P1 | `23830d5946` | 2026-04-10 | XInput < 2.4 startup crash | Linux 兼容性修复 |
| P1 | `dc3b25a4f4` | 2026-04-13 | WGPU atlas texture format fallback | wgpu 稳定性专项 |
| P1 | `e4ebd3aae5` | 2026-04-03 | WgpuAtlas screen share crash | wgpu 稳定性专项 |
| P1 | `924ac5c99b` | 2026-03-31 | WGPU device feature mismatch guard | wgpu 稳定性专项 |
| P1 | `dbd95ea742` | 2026-03-25 | Linux GPU recovery 后 force scene rebuild | 部分包含，需确认完整性 |
| P2 | `bf102668be` | 2026-04-08 | SvgRenderer 懒加载 enriched font DB | 缺失，性能优化 |
| P2 | `43d6ab5386` | 2026-04-23 | `ShapedLine::split_at` 减少 SharedString 分配 | 性能优化 |
| P2 | `2c237899f9` | 2026-04-22 | 分号换行归属前一词 | 文本细节 |
| P2 | `3ed6c68f3b` | 2026-01-15 | `truncate_line` UTF-8 slicing panic | 需核对当前是否完整包含 |
| P2 | `e1a46f9256` | 2026-05-08 | `ElementId` 尽量用 `SharedString::new_static` | 小性能优化 |
| P2 | `51b43c90f9` | 2026-05-12 | `ListState::is_scrollbar_dragging()` | 可随 list 批次合并 |
| P2 | `57765207c8` | 2026-05-12 | Linux examples unreachable panic | 多半是 Cargo/feature 级别，核对即可 |
| P2 | `1c16e13a2b` | 2026-05-15 | wgpu respect `buffer_font_fallbacks` | 若当前有对应设置再合并 |

### 建议拆分的实际任务批次

| 批次 | 范围 | 预期文件 | 风险 |
| --- | --- | --- | --- |
| A | cached Input + List scrollbar + Wayland clipboard | `window.rs`、`elements/list.rs`、`platform/linux/wayland/{client,serial}.rs` | 中，集中在核心 UI 行为 |
| B | Windows crash/device recovery | `platform.rs`、`platform/windows/{display,window,events,directx_atlas,directx_renderer}.rs`、`window.rs` | 高，需要 Windows smoke test |
| C | Wayland wgpu flicker/recovery | `platform/linux/wayland/window.rs`、`platform/wgpu/*` | 高，需要 Wayland release build 验证 |
| D | macOS cursor/font | `platform/mac/{window,platform,text_system}.rs`、`window.rs` | 中高，需要 macOS 手动验证 |
| E | Text/SVG 性能与渲染细节 | `svg_renderer.rs`、`text_system/*`、`elements/text.rs` | 中，视觉回归需截图/样例 |

## 关键核对命令

```bash
# 上游 GPUI 发布基线
cd /Users/hejun/codespace/my/agenttray/zed
git rev-parse gpui-v0.2.2
git log --format='%h %ad %s' --date=short gpui-v0.2.2..HEAD -- crates/gpui crates/gpui_linux crates/gpui_macos crates/gpui_windows crates/gpui_wgpu crates/gpui_platform crates/gpui_shared_string

# 当前仓库源码特征核对
cd /Users/hejun/codespace/my/agenttray/adabraka-gpui
rg -n 'input_handlers\.(pop|iter_mut)|PendingScroll|scrollbar_drag_start_height|frame_index|min\(max_frame_index|get_latest|renderer_presented|force_render.*refresh' crates/gpui/src

# 单个上游补丁查看示例
cd /Users/hejun/codespace/my/agenttray/zed
git show a221a86d49 -- crates/gpui/src/window.rs
git show 1c61cc3fc2 -- crates/gpui/src/elements/list.rs
git show 61e23fdb51 -- crates/gpui_linux/src/linux/wayland
```
