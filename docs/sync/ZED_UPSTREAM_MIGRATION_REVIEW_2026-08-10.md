# Zed 上游 GPUI 迁移价值审计

> 审计日期：2026-08-10  
> 当前仓库：`/Users/hejun/work/my/adabraka-gpui`  
> 当前 HEAD：`27afdb82f5c6de68af9ff959532767fc93a8beb5`  
> 上游仓库：`/Users/hejun/work/my/zed`  
> 上游 HEAD：`4bd1993783703e92affb781503916d1f152f599f`  
> 当前文档记录的上游增量基线：`ec3d887507f272119d9fe146c685f0a941d0e798`

## 1. 结论摘要

当前仓库已经不是一个简单的 Zed GPUI 旧版本副本，而是一个具有明确产品边界和发布拓扑的桌面 GPUI fork：

```text
fc-gpui facade
    -> fc-gpui-core
    -> fc-gpui-platform
        -> fc-gpui-linux / macos / windows
        -> fc-gpui-wgpu
```

本次对比基线之后，上游映射范围内共发现 49 个相关提交，其中绝大多数属于 Zed 产品、Web 后端、上游内部重构或本地已经等价覆盖的变化。真正值得迁移的内容应分为四层：

1. **立即修复的正确性问题**：本地确实存在风险，改动相对局部，应该优先进入下一批同步。
2. **公开 API 和平台体验补全**：有明确收益，但需要补测试和 facade 适配。
3. **迁移基础设施**：不是用户功能，却决定未来能否可靠追踪和验证上游同步，优先级很高。
4. **条件立项的架构能力**：包括真实线程调度、AnchoredPopup、结构化通知、外部拖放、渲染器重构等，不能直接 cherry-pick。

### 推荐优先级总表

| 优先级 | 推荐内容 | 结论 |
|---|---|---|
| P0 | WGPU 连续错误恢复计数修复 `008d54299b` | 建议立即迁移，当前本地存在实际逻辑缺陷 |
| P0 | WGPU/Linux Cosmic Text 混合 BiDi 段落修复 `c214057e08` | 建议立即迁移，当前本地 `ShapeLine::new` 路径仍可能断言崩溃 |
| P0 | `img` 显式宽高比不被 intrinsic ratio 覆盖 `59b2ebf103` | 建议立即迁移，单行行为修复，低冲突 |
| P0 | X11 Expose 后立即请求 presentation `ae99a867d7` | 建议迁移，修复被遮挡后重新暴露的空白风险 |
| P1 | Wayland 非活动窗口不更新 IME 位置 `655ed1385b` | 建议迁移，改动极小 |
| P1 | 嵌入字体零拷贝注册 `d0f797a38c` | 建议迁移，收益明确且局部 |
| P1 | Windows 非激活 PopUp 行为 `826f28eb8f` | 建议迁移，需保护本地 Overlay 行为 |
| P1 | Windows `request_attention` 只闪烁一次 `50e399332c` | 建议迁移，低风险 |
| P1 | macOS 移除私有模糊 API `06b6160d46` | 若仍支持 macOS 11，列为发布/兼容门禁；否则可降为 P2 |
| P1 | macOS/Linux 真实视觉 smoke CI | 建议补齐，当前测试证据对平台运行时覆盖不足 |
| P1 | 上游 provenance / baseline 分类门禁 | 建议立即建立，避免重复迁移和漏审 |
| P1 | `ThreadedDispatcher` test-support 适配 | 建议作为 scheduler 和真实并发测试前置 |
| P1 | 迁移后发布归档 dry-run 验证 | 建议补齐，当前 `cargo package --list` 不能证明可发布 |
| P2 | AccessKit accessibility identifier `cc053a4a6f` | 建议迁移，需检查本地 AccessKit node 生命周期 |
| P2 | hover listener layout reconciliation `38ca9106c5` | 建议在真实交互回归后迁移，改动面中等 |
| P2 | 圆角图片裁剪 `58df5a14ce` | 建议迁移，需要适配本地 `paint_image` 参数 |
| P2 | Grid `min-content/max-content` 行 API `5ccbbbd88f` | 建议作为低风险公开 API 补全，不应称为 P0 |
| P2 | Windows 路径宿主无关规范化 `2610332077` | 延期；当前补丁依赖上游独立 `path` crate，本地没有对应边界 |
| P2 | Wayland/macOS 外部文件拖放 `f52fd9ac44` + `c7aea6cbbd` + `a8491e63b5` | 仅在产品需要拖到 Finder/Firefox/Dolphin 时立项 |
| P2 | 最小异步 task benchmark | 在 ThreadedDispatcher 稳定后建立性能证据 |
| P2 | 独立 `container_query` | 有真实消费组件时迁移，不与 View 重构绑定 |
| P3 | WGPU/Metal 帧首批量上传 `be8c6f9fb3` | 性能/深度缓冲项目，不能整文件覆盖 |
| P3 | parent-native `AnchoredPopup` `546a16d64f` | 仅新增 GPUI 父 surface 模型，不替代 TrayAnchored |
| P3 | 结构化系统通知 `de827bce2f` | 需要三平台产品契约后再做 |
| P3 | subpixel/LCD 文本渲染 | 独立渲染项目，不能混入普通同步批次 |
| No-go | WebGL/WebGPU、`gpui_tokio`、`gpui_shared_string`、`gpui_util` 整体抽取、View/ViewElement 全量重构 | 当前产品边界或本地架构不需要 |

## 2. 审计方法与边界

### 2.1 对比范围

按当前同步规则，先固定两个仓库 HEAD，再审计：

```sh
git -C ../zed log --oneline \
  ec3d887507f272119d9fe146c685f0a941d0e798..HEAD -- \
  crates/gpui crates/gpui_platform crates/gpui_wgpu \
  crates/gpui_linux crates/gpui_macos crates/gpui_windows \
  crates/gpui_macros crates/gpui_util crates/util crates/sum_tree
```

本次覆盖的上游映射包括：

- `crates/gpui` -> `crates/gpui`
- `crates/gpui_platform` -> `crates/gpui-platform`
- `crates/gpui_wgpu` -> `crates/gpui-wgpu`
- `crates/gpui_linux` -> `crates/gpui-linux`
- `crates/gpui_macos` -> `crates/gpui-macos`
- `crates/gpui_windows` -> `crates/gpui-windows`
- `crates/collections`, `util`, `util_macros`, `refineable`, `sum_tree`, `http_client`, `media`
- 上游新出现的 `gpui_shared_string`、`gpui_util`、`gpui_tokio`

### 2.2 判定分类

每个上游变化按以下规则处理：

- `backport`：本地没有等价实现，且当前产品边界需要。
- `equivalent`：本地已有相同语义，即使文件路径、命名或实现不同。
- `intentional-divergence`：本地有意提供更宽的桌面能力或不同发布边界。
- `conditional`：技术上值得，但需要产品需求、性能证据、平台运行时证据或新的契约。
- `not-applicable`：服务 Zed 编辑器、Web、移动端或上游单仓库工具链，本地没有对应消费链路。

## 3. P0：应立即迁移的正确性修复

### 3.1 WGPU 连续错误恢复计数

**上游来源**：`008d54299be7a5219bac4e4dd51f11ab8bfd6197`

**本地目标**：

- `crates/gpui-wgpu/src/wgpu_renderer.rs:1190-1211`
- `WgpuRenderer::draw`
- 保留本地 `WgpuRenderer::recover`、`requested_instance_buffer_initial_size`、`AppResourceProfile` 和 `GpuResourceBudget`

审计发现本地错误计数分支先判断 `> 5`，随后才判断 `> 10`，并且清理 atlas 后没有把计数归零。结果是第 6 次连续 GPU error 后会持续走清 atlas并提前返回，`> 10` 分支不可达。

上游修复的语义是：

1. 先处理超过硬上限的失败。
2. 达到软阈值时清理 atlas。
3. 软恢复后将 `failed_frame_count` 归零。
4. 后续成功帧恢复正常绘制。

这是本次审计最明确的 P0 缺陷。它不要求同步上游整个 renderer，只需按本地资源预算和恢复状态机调整分支，并补纯状态机测试。

**验证要求**：

- 连续 1-5 次失败仍按原策略处理。
- 第 6 次触发清理后计数归零。
- 清理后的下一帧可以正常绘制。
- 超过硬上限的策略可达且行为稳定。
- Wayland、X11 的 device-lost / force-render-after-recovery smoke。

### 3.2 Cosmic Text 混合 BiDi 段落崩溃

**上游来源**：`c214057e086517920b214800725c5d16294ddf0d`

上游修复位置：

- `crates/gpui_wgpu/src/cosmic_text_system.rs::layout_line`
- `layout_line_with_separators`
- `shape_segment`
- `layout_line_no_separators`
- `contains_paragraph_separator`
- `clip_font_runs`

本地等价位置不是 `gpui-wgpu`，而是：

- `crates/gpui-linux/src/linux/text_system.rs::CosmicTextSystemState::layout_line`
- 当前约在 `:452`，内部仍直接对整段文本调用 `ShapeLine::new`

当同一段输入包含多个 BiDi paragraph，且不同段落方向不一致时，Cosmic Text 的 `ShapeLine::new` 可能触发 paragraph direction assertion。迁移时应把上游的段落切分算法移植到本地 Linux text system，保留本地的 user fallback chain、font ID 映射和 glyph metadata 语义，不要机械新增一份 `gpui_wgpu/cosmic_text_system.rs`。

**依赖**：上游增加 `unicode-bidi`，需要核对本地 `cosmic-text` 版本和 Cargo.lock。

**必须迁移的测试**：

- 混合方向段落。
- 所有 `Bidi_Class=B` 分隔符。
- 分隔符位于首尾和连续出现。
- font run 跨越 paragraph 边界。
- 无分隔符的 fast path。
- glyph byte index、位置单调性和 font run 合并。

### 3.3 `img` 显式宽高比不应被覆盖

**上游来源**：`59b2ebf103`

**本地目标**：`crates/gpui/src/elements/img.rs:391-393`

当前本地逻辑无条件执行：

```rust
style.aspect_ratio = Some(image_size.width / image_size.height);
```

应改为只在调用者未提供宽高比时设置 intrinsic ratio：

```rust
if style.aspect_ratio.is_none() {
    style.aspect_ratio = Some(image_size.width / image_size.height);
}
```

当前行为会覆盖 `aspect_square()` 或自定义 `aspect_ratio`，导致固定容器中的 portrait image + `ObjectFit::Contain` 不能正确 letterbox。该修复默认行为不变，只保护显式 API，冲突风险极低。

**验证**：迁移上游 `explicit_aspect_ratio_is_not_overridden_by_intrinsic_ratio`，在 `gpui-compat` 入口再做一次链式 API 编译和尺寸断言。

### 3.4 X11 Expose 后请求 presentation

**上游来源**：`ae99a867d7`

**本地目标**：

- `crates/gpui-linux/src/linux/x11/client.rs:84` 的 `expose_event_received`
- `process_x11_events`
- `start_refresh_loop:2064` 附近

上游删除了“先记一个 expose 标记、等 refresh loop 再处理”的路径，改为在 X11 事件批处理结束后，对仍 mapped 的窗口直接请求一次带 `require_presentation=true` 的刷新。这样，完全被遮挡导致周期刷新停止的窗口，在重新暴露后不会保持空白。

本地 X11 client 还承载 tray/global-hotkey 等桌面扩展，必须只改 windows map 和 refresh 逻辑，不能整体覆盖 `client.rs`。

**验证**：Xvfb/X11 下创建窗口、完全遮挡、等待刷新停止、揭开，断言一次 presentation；合并多次 Expose；unmapped window 不刷新。

## 4. P1：局部平台与渲染修复

### 4.1 Wayland 非活动窗口不更新 IME

**上游来源**：`655ed1385b`

**本地目标**：`crates/gpui-linux/src/linux/wayland/window.rs:1500` 的 `update_ime_position`

本地 `WaylandWindowState` 已有 `active` 字段，直接增加：

```rust
if !state.active {
    return;
}
```

这能避免多个 Wayland 窗口中，非活动窗口持续输出时改写共享 IME 候选框位置。改动不触碰 layer-shell、tray panel、input region。

### 4.2 嵌入字体零拷贝注册

**上游来源**：`d0f797a38c`

**本地目标**：`crates/gpui-linux/src/linux/text_system.rs:216-228`

本地 Borrowed 字体当前调用 `embedded_font.to_vec()`，上游改用：

```rust
db.load_font_source(cosmic_text::fontdb::Source::Binary(Arc::new(bytes)));
```

上游报告多字体场景可减少约 1.6 MiB 常驻内存。需要适配本地 Cosmic Text/fontdb 版本，确认 `Cow::Owned` 的生命周期和 `Arc` 包装都正确。

### 4.3 Windows 非激活 PopUp 行为

**上游来源**：`826f28eb8f`

**本地目标**：

- `crates/gpui-windows/src/windows/window.rs:421-424` 的 PopUp 创建样式
- `SetWindowPlacement` 路径
- `crates/gpui-windows/src/windows/events.rs:88` 起的消息处理

上游为 `focus=false` 的 GPUI popup 增加 topmost/`SW_SHOWNOACTIVATE` 语义，并通过 `WM_MOUSEACTIVATE` 修正标题栏点击激活顺序。本地已有 `WindowKind::PopUp` 和独立 `Overlay`，所以只移植 PopUp 行为，不能把 Overlay 的顶层、全屏和激活策略一起改掉。

上游的 `update_ime_enabled` 子补丁不直接适用：本地没有相同的 `query_prefers_ime_for_printable_keys` 体系。

### 4.4 Windows request attention 只闪烁一次

**上游来源**：`50e399332c`

**本地目标**：`crates/gpui-windows/src/windows/window.rs:777-795`

上游将 `FlashWindowEx` 的 count 改为一次。本地当前使用 `FLASHW_TIMERNOFG` 和持续语义，建议将 `PlatformWindow::request_attention` 改为一次闪烁，但保留本地 `Platform::request_attention(AttentionType::Informational/Critical)` 的自定义次数/持续行为，不要把两个 API 混为一谈。

### 4.5 macOS 移除旧版私有模糊 API

**上游来源**：`06b6160d46`

**本地目标**：

- `crates/gpui-macos/src/mac/window.rs:292-298` 的 `CGSMainConnectionID`、`CGSSetWindowBackgroundBlurRadius`
- `set_background_appearance`
- `blurred_view_init_with_frame` / `blurred_view_update_layer`

如果项目仍承诺 macOS 11，建议列为 P1 发布门禁；如果最低版本已提升到 macOS 12，可降为 P2。收益是减少 App Store 审核和私有符号链接风险。需要明确旧版本模糊效果是否允许降级到 NSVisualEffectView。

验证必须包括 `nm/otool` 产物符号检查和 macOS 12+ 的透明/不透明/模糊切换。

## 5. P1/P2：核心交互与 API 补全

### 5.1 AccessKit accessibility identifier

**上游来源**：`cc053a4a6f`

**本地目标**：`crates/gpui/src/elements/div.rs` 的语义 builder 和本地 AccessKit tree 更新路径。

上游新增 `accessibility_id(...)`，把稳定的 application-level ID 写入 AccessKit node 的 author ID。当前本地已有 AccessKit 基础集成、role、description 和 form control properties，但没有发现同名公开 builder。

收益是 Windows UIA `AutomationId`、macOS `AXIdentifier`、部分 Linux AT-SPI adapter 的稳定自动化定位。迁移前必须确认：

- ID 与 GPUI 内部 `ElementId` 分离。
- 布局重建后 author ID 保持稳定。
- facade 能导出方法。
- test platform 能读回 node。

### 5.2 hover listener layout reconciliation

**上游来源**：`38ca9106c5`

当前本地 `Div` 的 hover listener 主要由 MouseMove 和 MouseExit 驱动。布局移动了鼠标下方元素但鼠标没有移动时，新的 hitbox 可能表现为 hover，却不会收到 `on_hover(true)`，旧元素也不会收到对应退出。

上游在绘制/布局后按当前 hit test reconcile listener，并把状态变化延迟到 paint cycle 之后；鼠标按下待完成期间暂停 reconciliation，避免 hover-only 控件在 mouse-down / mouse-up 间被卸载。

这是中等风险的核心交互修复，应移植上游回归测试后再进入。重点是本地 `Window` 的 hitbox tree、cached view 和 callback 所有权可能与上游不同。

### 5.3 修饰键、多击绑定和 IME 顺序

相关上游提交：

- `a8cae3bd77`：多修饰键手势不再独立派发 standalone modifier。
- `dc1e815e47`：IME 输入前完成 pending key binding。

这两项不能不加区分地 cherry-pick：

- 本地 core 已有较早的 `pending_modifier` 处理，需先写事件序列测试确认 standalone modifier 是否仍可重现。
- 本地 macOS 仍是 GPUI-first 的 printable key 路径，没有上游较新的 `query_prefers_ime_for_printable_keys` 合约，因此 `dc1e815e47` 的 macOS 片段不直接适用。

建议把它们列为 P2 条件迁移：先明确本地平台实际事件序列，再只移植有失败测试证明的 hunk。

### 5.4 圆角图片裁剪

**上游来源**：`58df5a14ce`

当前本地 `Img::paint` 使用 `new_bounds` 同时作为 sprite bounds 和裁剪参考，且把 corner radii 按 `new_bounds.size` clamp。对于 `ObjectFit::Cover`，图像实际绘制区域大于元素容器时，圆角/裁剪可能不正确。

上游把元素 `bounds` 和图像 `image_bounds` 分开传给 `Window::paint_image`，对元素 bounds 应用圆角，并对可见交集做裁剪。该改动需要本地 `paint_image`、scene sprite 和 renderer 同步适配，风险高于 `59b2ebf103`，建议独立迁移并补像素/几何断言。

### 5.5 Grid 行轨道 API

**上游来源**：`5ccbbbd88f`

上游新增：

- `Styled::grid_rows_min_content`
- `Styled::grid_rows_max_content`
- `GridRows` 到 Taffy 的对应转换

本地 `styled.rs`、`style.rs`、`taffy.rs` 没有对应公开方法。它是 API/布局能力补全，不是 P0 bug。适合在 `img` 修复之后独立迁移，先确认本地现有 grid template 类型和 facade re-export。

验证应包括 Taffy 生成值、链式 builder 编译和数据密集布局的尺寸断言。

### 5.6 Windows 路径规范化

**上游来源**：`2610332077`

上游提交修改的是独立 `crates/path` 中的 `PathStyle::normalize` 测试和语义，`crates/util/src/paths.rs` 只是重新导出该类型。当前仓库没有上游 `path` crate，本地 `crates/util/src/paths.rs::PathStyle` 也没有同构的 `normalize` API。

因此不能把测试孤立复制到 `fc-gpui-util`。只有在本地出现跨宿主处理 Windows 远端路径的真实调用点时，才应先决定：抽取最小 path crate，还是在本地 `PathStyle` 上新增清晰的词法规范化 API；随后再迁移盘符、UNC、混合分隔符、`.`、`..` 和根目录边界测试。

### 5.7 滚动轴锁定

**上游来源**：`79cc17c216`

上游引入 `OngoingScroll`，让 pixel scroll gesture 锁定初始主轴，方向明显改变时才解锁；同时修改 `Div` 和 `Style::restrict_scroll_to_axis`。

本地已有滚动条拖拽和部分滚动稳定性修复，但当前 `gestures.rs` 没有对应的 ongoing axis state。该改动会影响滚动容器行为，应只在本地能复现“横向滚轮被垂直列表劫持”时迁移，不能作为安全文件直接复制。

## 6. 条件迁移的跨平台能力

### 6.1 Wayland/macOS 外部文件拖放

相关上游提交：

- `f52fd9ac44`：core/platform/macOS 外部拖放基础。
- `c7aea6cbbd`：Wayland `wl_data_source`。
- `a8491e63b5`：macOS 拖回源窗口时恢复 typed payload。

本地目前没有完整的 `ExternalDragPayload`、`PlatformWindow::start_external_drag`、`FileDropEvent::Ended` 和平台 drag ownership 链路。Wayland 现有 `wl_data_source` 主要用于 clipboard/primary selection，不等于外部文件 drag source。

这是一个完整能力项目，至少需要：

```text
core ExternalDragPayload/FileDragPaths
    -> PlatformWindow::start_external_drag
    -> Linux wl_data_source / macOS NSDraggingSource
    -> FileDropEvent::Ended
    -> active_drag ownership 恢复与清理
```

只有产品明确需要把文件拖到 Firefox、Dolphin、Finder 或其他应用时才值得启动。不能只复制 Wayland client 文件，也不能改变现有内部 drag、TrayAnchored 或 Overlay。

### 6.2 WGPU/Metal 帧首批量上传

**上游来源**：`be8c6f9fb3`

上游将 instance buffer 从“绘制时分块写入”改为“帧开始时批量上传”，同时重做 wgpu、Metal、DirectX 的 binding 和 shader 结构，解决部分 Windows flicker/driver 问题，并为 depth buffer 设计铺路。

本地不能整文件覆盖，因为本地额外维护：

- `GpuResourceBudget`
- `AppResourceProfile`
- `atlas_initial_size`
- `requested_instance_buffer_initial_size`
- renderer cache stats/trim
- device-lost recovery

如果未来有像素 flicker、深度缓冲或 `queue.write_buffer` 性能证据，应分三个平台分别适配，保留本地预算传播，并对空场景、超大场景、binding limit、resize、device lost 做 render-to-image 验证。

### 6.3 Parent-native AnchoredPopup

**上游来源**：`546a16d64f`

这是 GPUI native parent surface 的 popup 模型，主要在 Wayland 使用 `xdg_popup`、`xdg_positioner`、anchor/gravity/constraint adjustment。它不等价于本地：

- `WindowKind::Overlay`
- `WindowPosition::TrayAnchored`
- `LayerShell`

本地 tray panel 的 parent 是外部 desktop shell，不是 GPUI native surface，不能伪装成 xdg_popup。只有存在可复现的父窗口定位、焦点、关闭或跟随问题时才启动；而且应先实现 core contract 和 Wayland，明确 X11/macOS/Windows fallback，不要把它列入普通 P1 同步批次。

### 6.4 结构化系统通知

**上游来源**：`de827bce2f`

上游增加 tag/replace、action、response、dismiss 和 app identity 的部分契约。当前本地只有简单 title/body notification，并且保留 tray balloon、daemon、QuitMode 和本地通知生命周期。

真正迁移前必须先决定：

- tag 是否跨进程/重启稳定。
- unsupported action 是降级还是错误。
- dismiss 是否产生 response。
- 无窗口 daemon 的 callback 如何回到 foreground executor。
- 应用已退出时点击通知是否需要冷启动/单实例激活。

建议顺序是 fake platform contract -> Linux tag/replace -> action/response -> macOS UNUserNotificationCenter delegate -> Windows Toast/AUMID。当前不建议把它作为普通同步项。

### 6.5 scheduler / ThreadedDispatcher / benchmark

旧文档把 scheduler、生产优先级、统一时钟、View 重构、平台拆分混成一个大项目。当前上游和本地状态支持更细的分层：

#### 第一阶段：test-support ThreadedDispatcher

相关上游提交链：

```text
8886dcb0d4 -> 300972bea3 -> 38df25d54c -> 82878540b5
```

建议只迁真实线程、真实 timer、external wake、自重排和 readiness 测试，保持本地公开 `Task` 和平台 dispatcher contract 不变。目标是新增 `crates/gpui/src/platform/threaded_dispatcher.rs` 或对应本地模块，并通过 `test-support` 暴露。

#### 第二阶段：最小异步 benchmark

在 ThreadedDispatcher 稳定后，吸收 `BenchAppContext::bench_task/bench_batched_task` 的测量边界，但不整包复制上游 `BenchAppContext`、Criterion、frame profiler 和 scheduler 类型。至少应证明：

- setup/drop 不被计时。
- completion 以 task 真正完成为准。
- foreground/background/timer/external wake 可以分别测量。
- 自重排 main-thread runnable 不会把 benchmark 拖到 queue quiescence。
- frame 开始后新排队任务不被错误纳入当前 frame 的 benchmark。

#### 第三阶段：内部 scheduler 边界

只有 benchmark 能提供稳定证据，才考虑内部 `adabraka_scheduler` 或 `PlatformScheduler`。生产 priority/realtime 不应自动开启：必须有 starvation、p95/p99、功耗和用户可见首帧/托盘响应证据。

#### 明确不做

- 不直接把上游 `RunnableMeta` 替换本地公开 Task。
- 不同时改四个平台 dispatcher。
- 不把 scheduler、ViewElement、AnchoredPopup、结构化通知放在同一批次。
- 不因为上游已有 scheduler crate 就自动新增发布 crate。

### 6.6 container_query

上游 `49ad06c1b` 的 container query 是独立 Element，并不要求先完成 View/ViewElement。建议只有在本地出现第一个真实消费组件时实施：tray/settings/sidebar 等按容器尺寸响应的布局。

必须验证：窄/宽容器、resize、嵌套 scope、closure 调用次数、cached AnyView 过期结果。若最小实现被迫先重写 view/cache 管线，应停止并重新评估。

## 7. 支持库和依赖图结论

### 7.1 建议迁移

#### Windows 路径规范化 `2610332077`

见第 5.6 节。它只依赖标准库，适合在 `fc-gpui-util` 内做增量移植。

#### TypeId 专用 hasher：仅在有热点时

上游 `gpui_util`/`collections` 增加 `TypeIdHashBuilder`、`TypeIdHashMap`、`TypeIdHashSet`。当前本地没有明确的 TypeId map 热点，不建议为了 API 对齐新增 `gpui_util` crate。

只有 profiler/benchmark 证明 TypeId hashing 是实际热点时，才在本地无环模块中增加实现，并先做性能对比。

### 7.2 等价或有意不同，不建议整包迁移

- `gpui_shared_string`：本地 `crates/gpui/src/shared_string.rs` 已使用 SmolStr，并已有 `Arc<str>` 等转换；新增发布 crate 不产生运行时收益。
- `gpui_util`：上游主要为 Web 和移除 `gpui -> util -> async-process` 依赖服务；本地依赖图已经不同，新增 crate 会扩大发布矩阵。
- `gpui_tokio`：本地使用 smol executor，当前没有 Tokio 桥接产品需求。
- `collections`、`util_macros`、`gpui_macros`、`refineable`、`derive_refineable`、`http_client`、`media`：基线后的对应源码变化不足以形成迁移项。
- `sum_tree` 的 `truncate(0)` -> `clear()`：属于 Rust 版本/机械清理，不值得单独同步。
- `semantic_version`：本地仍是 GPUI 公共依赖，不能因为上游组织变化就删除。
- `perf`：上游工具化与本地可发布 `fc-gpui-perf` 是不同产品边界。

### 7.3 明确不适用的 util 改动

- `d61edb0825` 的 `merge_json_value_into` 数组语义：上游修复 Zed LSP initialization options，本地无调用链；不能静默改变已发布 util API。
- `1102219f81` Markdown fragment 定位：服务 Zed markdown preview，本地没有对应消费方。
- `8e18ab0cd7` ShellBuilder stdin 修复：本地没有上游 ShellBuilder 抽象。
- `a6a23c7b80` / `c9d1d0ddfe` util 依赖移除：服务 Zed Web/依赖瘦身，本地桌面依赖边不同。

## 8. 测试、CI、发布维护建议

### 8.1 Provenance gate

当前 `docs/sync/CURRENT.md` 的基线和 `Zed-Origin` 依赖人工维护。建议新增一个只读脚本，例如 `scripts/verify-upstream-sync.sh`，并在 CI 中校验：

- baseline commit 存在且是上游 HEAD 的祖先。
- 增量映射 crate 提交被完整列出。
- 每个上游提交都有 `backport/equivalent/deferred/not-applicable` 分类。
- `backport` 本地提交有完整 `Zed-Origin: <hash>`。
- 新审计完成后，才允许更新 `CURRENT.md` baseline。

当前区间的首批门禁应至少能识别 `8886dcb0d4`、`300972bea3`、`38df25d54c`、`82878540b5` 等 ThreadedDispatcher 链。

### 8.2 固定 Rust 和 Actions

上游当前已经使用 Rust `1.97.1`，但是否将其作为本项目 MSRV 需要单独决定。建议：

- 先在三平台验证，再决定 `rust-toolchain.toml`。
- 固定 Actions 到不可变 SHA。
- 增加 `actionlint`、`zizmor`、`shellcheck`。
- 不直接复制 Zed 的完整 workflow、Namespace runner、Node/pnpm、Postgres、xtask generator。

### 8.3 真实视觉 smoke

本地已经有跨平台 `render_to_image`、`real_visual_smoke` 和 Windows ignored smoke，但 macOS/Linux 真实运行时 CI 仍不完整。建议：

- macOS runner 执行 Metal ignored smoke。
- Linux X11 使用 Xvfb + Mesa/软件 Vulkan 执行 WGPU smoke。
- Wayland/layer-shell 仅在专用 runner 执行。
- 能力不足时显式 skip 并上传原因，不能静默假绿。

### 8.4 多包发布归档

当前 package gate 主要检查归档清单，不能证明去掉 path 依赖后上层包能解析。建议新增 dry-run 验证器：

1. 按 `util/macros -> core/renderer/backends -> platform -> facade` 顺序生成归档。
2. 在临时 local registry 安装前序包。
3. 对每个 `.crate` 解包执行 metadata/check。
4. 检查 README/LICENSE、package name、version、path dependency 和 checksum。
5. 默认不得访问 crates.io 写接口，真实发布要求显式二次确认。

## 9. 已确认等价或必须保留的本地能力

以下项目不应在下一轮被误判为缺口：

- GPU device-lost 主恢复链，含 WGPU adapter 选择和 software adapter rejection。
- `PlatformDisplay::visible_bounds` 和窗口可见区域定位。
- Wayland clipboard serial/timeout 处理。
- Wayland redundant surface commit 避免 flicker。
- headless open_window 和 atlas。
- atlas tile 回收/free list。
- Vulkan/GLES 桌面 backend 选择。
- `is_resizable/is_minimizable` 创建时 native 行为。
- DirectX `FirstElement` 驱动问题的本地等价绕开。
- AccessKit 基础 tree、role、description、form controls 和 actions。
- TestApp、headless text system、跨平台 render-to-image、visual smoke 基础设施。
- 本地 runtime profiler、feature matrix、downstream facade/rename fixtures。
- `QuitMode`、`GpuResourceBudget`、`AppResourceProfile`、GPU trim/stats。
- tray、global hotkey、notification、auto-launch、power、permission、single-instance、daemon、Overlay、LayerShell、TrayAnchored、zenity/kdialog DialogOptions。

尤其禁止用上游目录整体覆盖以下模块：

- `crates/gpui/src/app.rs`
- `crates/gpui/src/window.rs`
- `crates/gpui/src/platform.rs`
- `crates/gpui-linux/src/linux/wayland/client.rs`
- `crates/gpui-linux/src/linux/x11/client.rs`
- `crates/gpui-macos/src/mac/window.rs`
- `crates/gpui-windows/src/windows/window.rs`
- `crates/gpui-wgpu/src/wgpu_renderer.rs`

这些文件包含本地生命周期、资源预算、桌面扩展和兼容 facade 所需的 ownership，必须按单行为、单提交、单测试增量合并。

## 10. 推荐执行顺序

### Stage 0：建立同步基线与门禁

- 先完成本次 `ec3d887..4bd1993` 的分类记录。
- 添加 provenance checker 和机器可读分类清单。
- 不要在分类未完成前更新 `docs/sync/CURRENT.md` 的 baseline。
- 将旧文档中的绝对路径和拆分前命令标记为历史材料。

### Stage 1：P0 正确性批次

按独立提交实施：

1. WGPU `failed_frame_count`。
2. Cosmic Text BiDi paragraph splitting。
3. `img` explicit aspect ratio。
4. X11 Expose immediate presentation。

每个主题单独保留 `Zed-Origin`，先跑 focused tests，再跑：

```sh
cargo test --locked -p fc-gpui-core --lib --features test-support
cargo test --locked -p fc-gpui --tests --features test-support
scripts/verify-migration.sh
```

### Stage 2：P1 平台和 API

- Wayland inactive IME。
- borrowed embedded font。
- Windows popup/attention。
- macOS private blur API。
- AccessKit identifier。
- Grid row APIs。
- 圆角图片裁剪和 hover reconciliation 分开处理。

### Stage 3：测试与发布基础设施

- ThreadedDispatcher test-support。
- 异步 task benchmark。
- macOS/Linux real visual CI。
- release archive dry-run。
- provenance baseline gate。

### Stage 4：需求触发的架构项目

按独立项目启动，不与普通同步批次混合：

- external drag。
- AnchoredPopup。
- structured notifications。
- container_query。
- WGPU/Metal batch instance upload。
- scheduler 内部提取和生产 priority。
- subpixel/LCD text rendering。

## 11. 残余风险和审计限制

- 本次是源码、Git 历史和现有文档审计，没有修改源码。
- 没有执行 Windows/macOS 原生 runtime smoke，也没有在当前主机验证 Wayland compositor、Fcitx5、Metal 或 DirectX。
- `cargo metadata --no-deps` 和 `git diff --check` 已通过；未运行完整 `cargo test`，因此 P0 候选仍需要实施后的行为验证。
- 上游新提交会继续进入 `ec3d887..HEAD`，正式实施前应重新 fetch 上游并把审计 HEAD 更新为实际 commit，不应只按日期判断。
- 当前仓库已有未跟踪文件 `docs/plans/2026-08-02-migration-findings-assessment.md`，本审计未修改该文件。

## 12. 相关现有材料

- [`docs/sync/CURRENT.md`](./CURRENT.md)：当前同步规则和 crate 映射。
- [`docs/sync/ZED_SYNC_MAPPING.md`](./ZED_SYNC_MAPPING.md)：历史映射说明。
- [`docs/plans/2026-08-02-migration-findings-assessment.md`](../plans/2026-08-02-migration-findings-assessment.md)：旧 finding 复审，尚未覆盖本次 8 月 3 日之后增量。
- [`docs/todo/gpui-upstream-architecture-migration-evaluation.md`](../todo/gpui-upstream-architecture-migration-evaluation.md)：架构方向历史材料，部分单 crate 前提已失效。
