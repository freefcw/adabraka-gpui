# Adabraka GPUI 与 Zed 上游差异审计（2026-05-17）

这份文档是重审版。旧稿里有几条结论已经过时，我先按当前仓库源码重新核对，再只保留仍然成立的差异。

## 结论

当前仓库和上游 Zed 的关系，已经不是“简单抽取后长期落后”那么单纯了。

很多 2026-05 的高风险修复，当前仓库其实已经有了，例如 cached view + Input、Wayland 点击复制、Wayland redundant commit、列表滚动条拖拽、Windows monitor missing、macOS font smoothing 等，所以不能再按旧分析直接当缺口。

但仍有几类明确差异还没补上：

1. Linux wgpu 的 GPU 恢复策略还少一层“拒绝软件渲染器”的保护。
2. Linux wgpu 还没接上上游的 `buffer_font_fallbacks`。
3. macOS 原生对话框还没有补上默认按钮焦点修正。
4. `PlatformDisplay::visible_bounds()` 仍缺失，窗口默认摆放还会忽略任务栏 / Dock。
5. 上游新增的 scheduler / profiler / visual test / headless app 基础设施，当前仓库没有对应实现，属于架构级差异，不适合直接整块搬。

## 比对范围

- 当前仓库：`/Users/hejun/codespace/my/agenttray/adabraka-gpui`
- 上游仓库：`/Users/hejun/codespace/my/agenttray/zed`
- 当前仓库 HEAD：`3f0ae7a006093afe3309fbf54f4dfad69847b5a1`（2026-05-17，`perf: reuse static strings for element ids`）
- 上游参考 HEAD：`3bd9d13b63fc5a5ffa39326597bc4fd91adc82d1`（2026-05-15，`settings: Fix inverted VS Code import for files.simpleDialog.enable`）
- 上游参考范围：`gpui` 相关最新本地 HEAD 以及 `gpui-v0.2.2` 之后的提交

说明：

- 上游 `gpui` 已拆成多个 crate，很多原来在 `crates/gpui` 的平台实现现在分散到 `gpui_linux`、`gpui_macos`、`gpui_windows`、`gpui_wgpu`、`gpui_platform` 等 crate。
- 当前仓库仍把平台代码留在 `crates/gpui/src/platform/**` 下，所以要按“功能映射”看，而不是按路径一一对齐。

## 已核实存在，不再算缺口

| 上游主题 | 当前状态 | 证据 |
| --- | --- | --- |
| cached view + Input 崩溃修复 | 已有 | `crates/gpui/src/window.rs` 里已经按 `None` slot 回填并保留 `input_handlers` 长度 |
| Wayland 鼠标点击复制 serial 修复 | 已有 | `crates/gpui/src/platform/linux/wayland/serial.rs` 有 `get_latest()`，`client.rs` 的 `write_to_primary` / `write_to_clipboard` 也在用它 |
| 列表滚动条拖拽稳定性 / `is_scrollbar_dragging()` | 已有 | `crates/gpui/src/elements/list.rs` 已有 `PendingScroll`、`scrollbar_drag_start_height`、`is_scrollbar_dragging()` |
| Wayland redundant surface commit flicker | 已有 | `crates/gpui/src/platform/linux/wayland/window.rs` 已有 `renderer_presented`，`completed_frame()` 会跳过重复 `surface.commit()` |
| macOS 非数字 font smoothing 默认值 | 已有 | `crates/gpui/src/platform/mac/text_system.rs` 里 `downcast_into::<CFNumber>()` 失败时直接回退为允许 smoothing |
| Windows monitor missing panic | 已基本规避 | `crates/gpui/src/platform/windows/display.rs` 现在是 `Option<Self>`，不是旧版那种 `expect(...)` 路径 |

## 仍缺失或仅部分覆盖

| 优先级 | 上游提交 / 文件 | 当前状态 | 适用性判断 |
| --- | --- | --- | --- |
| P0 | `008d54299b`，`crates/gpui_wgpu/src/wgpu_context.rs`、`crates/gpui_wgpu/src/wgpu_renderer.rs`、Linux window 相关 | 缺失 | 上游在 GPU 恢复时新增 `new_rejecting_software()`，避免刚唤醒时又挑到 llvmpipe / 软件后端；当前 `recover()` 仍走普通 `WgpuContext::new(...)` |
| P1 | `1c16e13a2b`，`crates/gpui_wgpu/src/cosmic_text_system.rs` | 缺失 | 当前 Linux text shaping 没有 `buffer_font_fallbacks` 这条路径；这会影响 buffer 级别的 fallback 顺序，尤其是 emoji / ZWJ / combining marks 的回退一致性 |
| P1 | `7ab0ce6e68`，macOS dialog / prompt | 缺失 | 当前 `crates/gpui/src/platform/mac/dialog.rs` 只是直接 `runModal()`，没有补上初始焦点重定向；“Save / Don’t Save / Cancel” 这类对话框仍可能把键盘焦点落到不理想的按钮上 |
| P1 | `PlatformDisplay::visible_bounds()`，`window.rs` 默认窗口摆放 | 缺失 | 当前 `crates/gpui/src/platform.rs` 只有 `bounds()`，`crates/gpui/src/window.rs` 也没有基于可见区域摆放窗口；在 macOS / Windows 上会更容易压到 Dock / taskbar |
| P2 | `ElementId` 的额外 `From` 实现，`crates/gpui/src/window.rs` | 部分缺失 | 当前保留了 `&'static str` / tuple / `SharedString` 的路线，但还缺 `String`、`Arc<str>`、`[u8; 20]` 等上游补充的 ergonomic 转换，属于小型 API 补全 |

### 这几项为什么值得迁

- `008d54299b` 是最直接的稳定性补丁，目标很明确，和当前仓库的 Linux wgpu 路径兼容性强。
- `buffer_font_fallbacks` 不是纯性能优化，它会影响实际字形选择和 fallback 顺序，属于可见行为修正。
- macOS dialog 的焦点问题是用户直接能感知的键盘可达性问题，改动点小，回归面也相对可控。
- `visible_bounds()` 是窗口摆放层面的基础设施，能直接改善默认位置和遮挡问题，适合单独补。

## 架构级差异，不建议直接整块合并

这些是上游新增、当前仓库没有 1:1 对应物的模块：

- `crates/gpui/src/app/headless_app_context.rs`
- `crates/gpui/src/app/test_app.rs`
- `crates/gpui/src/app/visual_test_context.rs`
- `crates/gpui/src/platform_scheduler.rs`
- `crates/gpui/src/platform/visual_test.rs`
- `crates/gpui/src/profiler.rs`
- `crates/gpui/src/queue.rs`

这批改动的共同点是：

- 牵涉 `Task` / executor / dispatcher 的整体调度语义，不是单个函数补丁。
- 上游已经把任务系统和平台调度拆得更细，当前仓库还停留在更简单的 `async_task` 路线。
- 当前仓库已经有自己的 `app/test_context.rs` 和 `platform/test/**`，所以更像“有局部替代品，但不完全同构”。

结论是：这部分值得长期跟，但不适合按 commit 直接机械搬运。

## 和当前仓库的关系判断

| 上游方向 | 当前仓库现状 | 结论 |
| --- | --- | --- |
| Linux wgpu 恢复策略 | 已有 GPU recovery，但缺软件后端拒绝 | 建议补 |
| 文本 fallback 链 | 有基础 `FontFallbacks`，但缺 buffer 级别接线 | 建议补 |
| 原生对话框交互 | 已有 `show_dialog`，但没做默认焦点修正 | 建议补 |
| 显示器可见区域 | 当前只有 `bounds()` | 建议补 |
| scheduler / profiler / visual test | 当前没有对应架构 | 先评估，不直接合并 |

## 推荐同步顺序

1. `visible_bounds()` + macOS dialog 焦点修正
2. Linux wgpu 的 `new_rejecting_software()` / recovery 路径
3. `buffer_font_fallbacks`
4. `ElementId` 的额外转换 API
5. scheduler / profiler / visual test 这类架构项，放到单独重构批次

## 结语

这次审计的核心不是“上游有哪些改动”，而是“当前仓库真正还缺什么，而且补了以后大概率不会把现有功能打坏”。

按这个标准看，当前最值得先动的是：

- Linux wgpu 恢复时拒绝软件后端
- `buffer_font_fallbacks`
- macOS dialog 的默认焦点
- `visible_bounds()`

其余架构级差异，建议单独开一轮同步，不要混在 bugfix 批次里。
