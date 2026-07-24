# Zed GPUI 增量审计（自 24f62484 以来）

日期：2026-07-01  
上游仓库：`/Users/hejun/work/my/zed`  
当前仓库：`/Users/hejun/work/my/adabraka-gpui`  
上游基准：`24f62484e936aa355c72f2009313bbe2898a9fd5`（2026-04-29，`Support latest MCP protocol version`）

## 结论

自 `24f62484` 以来，Zed 的 GPUI 相关改动很多，但当前仓库已经人工吸收了其中一批 5 月高优先级修复，因此不应按目录或按 crate 整块同步。

执行前判断最值得引入的是小而明确的稳定性补丁：Wayland 剪贴板读取超时、atlas tile 空间释放、macOS fallback 字体修复、IME candidate 定位、列表滚动 pending scroll 修复、tooltip 卡住修复、Windows immovable hit-test、Wayland 初始 `app_id`。

AccessKit、benchmark/profiler 二期、web 平台和 scheduler 相关改动价值存在，但它们属于架构级迁移，不建议混入 bugfix 批次。

## 大纲

1. **分析范围**：说明本次纳入的 Zed `gpui*` crate、基准提交和路径映射原则。
2. **已覆盖项**：列出当前仓库已经包含或已有等价实现的上游修复，避免重复同步。
3. **建议引入清单**：按 P0/P1/P2 排序列出值得引入的上游提交、当前缺口、风险和验证方式。
4. **架构级专题**：单独评估 AccessKit、benchmark/profiler、scheduler、web backend 等大规模改动。
5. **不建议同步项**：标记低价值或 Zed 业务绑定的改动，避免同步噪音。
6. **执行批次**：把候选补丁拆成 P0 稳定性、P1 输入交互、P2 文本/API、架构专题四批。
7. **后续原则**：约束每个同步提交的粒度、测试、平台扩展保护和验证顺序。

## 执行路线图

| 批次 | 目标 | 上游提交 | 当前动作 |
| --- | --- | --- | --- |
| A / P0 | 稳定性与资源修复 | `bda5ac3626`、`0d8a4d4292`、`486cf9ef3c`、`61ad9ebfcd`、`35eaeb94a7` | 优先实现，必须带回归测试或手动验证 |
| B / P1 | 输入与交互修复 | `737f55a1a1`、`126c0ee41a`、`137e677a05`、`9ac117693b`、`507d043d96`、`b3bc83b57e` | 第二批实现，重点覆盖 IME、tooltip、Windows hit-test |
| C / P2 | 列表、文本和 API 补齐 | `7fd5ea4bf3`、`a1d2ef6514`、`8036a3c74b`、`34cd17ff5e` | 在核心稳定后引入，按组件测试验证 |
| D / 架构 | a11y / profiler / scheduler / web | `1d029c5ff5`、`297c4a4d78`、`39f7849a0f` 等 | 单独设计，不混入 bugfix 批次 |

## 本轮同步结果（2026-07-01）

本报告生成后，已按“一个上游主题一个本地提交”的原则完成 A/B/C 批次的同步；D 批次仍按架构专题单独迁移。

| 批次 | 上游提交 | 本地提交 | 结果 |
| --- | --- | --- | --- |
| A | `0d8a4d4292` | `ceff3e0` | atlas tile 删除时释放 allocator 空间 |
| A | `35eaeb94a7` | `4f2bb3b` | Wayland 首次 commit 前设置 `app_id` |
| A | `486cf9ef3c` | `702266b` | macOS system fallback descriptors 实际 append |
| A | `61ad9ebfcd` | `e3df6aa` | macOS fallback 继承 primary 字重/样式 |
| A | `bda5ac3626` | `c431697` | Wayland clipboard / DnD pipe 读取超时 |
| B | `9ac117693b` | `eb8bee4` | Windows immovable 窗口保留 caption button hit-test |
| B | `737f55a1a1` | `0c429f1` | IME candidate 锚到当前视觉行起点 |
| B | `126c0ee41a` | `5d326c0` | Wayland 非文本输入状态禁用 IME |
| B | `137e677a05` | `cc2af91` | Wayland 多窗口只由 active window 更新 IME 状态 |
| B | `507d043d96` | `283960a` | 鼠标离开 origin 时刷新 tooltip 可见性 |
| B | `b3bc83b57e` | `3dad8b3` | tooltip show delay 可配置 |
| C | `8036a3c74b` | `2e8d74c` | NBSP/NNBSP/NBH 作为 non-breaking glue 处理 |
| C | `7fd5ea4bf3` | `351d091` | 用户滚动后 rebase list pending scroll |
| C | `a1d2ef6514` | `bbdf299` | `ListState` viewport 查询 API |
| C | `34cd17ff5e` | `0f5217a` | `BoxShadow` builder API 与宏 preset 使用 builder |

验证已覆盖可在本机执行的默认 macOS `cargo check -p adabraka-gpui --lib`，以及对应纯 Rust / test-support 单测。Linux Wayland 与 Windows 目标的完整交叉检查受本机缺少平台 C toolchain 限制，需在对应平台补跑。

## 分析范围

纳入上游路径：

- `crates/gpui`
- `crates/gpui_linux`
- `crates/gpui_macos`
- `crates/gpui_macros`
- `crates/gpui_platform`
- `crates/gpui_shared_string`
- `crates/gpui_tokio`
- `crates/gpui_util`
- `crates/gpui_web`
- `crates/gpui_wgpu`
- `crates/gpui_windows`

上游在该范围内约有 93 个相关提交，114 个文件变化。由于当前仓库保留单 crate 平台层，而 Zed 已拆分多个 `gpui_*` crate，本审计按功能映射判断，不按路径直接覆盖。

## 已经包含或基本覆盖

| 上游主题 | 当前状态 | 当前证据 | 处理建议 |
| --- | --- | --- | --- |
| Wayland clipboard serial 使用最近 serial（`61e23fdb51`） | 已有 | `crates/gpui/src/platform/linux/wayland/serial.rs:55`、`crates/gpui/src/platform/linux/wayland/client.rs:933` | 不重复引入 |
| Wayland redundant surface commit flicker（`923f315f26`） | 已有 | `crates/gpui/src/platform/linux/wayland/window.rs:125`、`crates/gpui/src/platform/linux/wayland/window.rs:1573` | 不重复引入 |
| Wayland Mailbox present mode（`980a294292`） | 已有 | `crates/gpui/src/platform/linux/wayland/window.rs:201` | 不重复引入 |
| Linux wgpu recovery 拒绝软件 renderer（`008d54299b`） | 已有 | `crates/gpui/src/platform/wgpu/wgpu_context.rs:36`、`crates/gpui/src/platform/wgpu/wgpu_renderer.rs:1691` | 不重复引入 |
| `PlatformDisplay::visible_bounds()` | 已有 | `crates/gpui/src/platform.rs:439`、`crates/gpui/src/platform/window_positioner.rs:14` | 不重复引入 |
| `Rgba::alpha()`（`c899d5b590`） | 已有 | `crates/gpui/src/color.rs:579` | 不重复引入 |
| inset shadow（`eb944cfd7a`） | 已有 | `crates/gpui-macros/src/styles.rs:550`、`crates/gpui-macros/src/styles.rs:560` | 不重复引入 |
| `BoxShadow` builder API（`34cd17ff5e`） | 未见等价 builder | `crates/gpui/src/style.rs:361` 仍只有结构体 | P2 ergonomic，可选 |
| `buffer_font_fallbacks` 接线（`1c16e13a2b`） | 已有本地等价路径 | `crates/gpui/src/platform/linux/text_system.rs:662` 有回归测试 | 暂不重复引入 |

## 建议引入清单

### P0. Wayland clipboard 读取超时

- 上游提交：`bda5ac3626`，`linux: Fix Wayland clipboard reads blocking indefinitely`
- 上游影响：`gpui_linux/src/linux/platform.rs`、`gpui_linux/src/linux/wayland/clipboard.rs`、`gpui_linux/src/linux/wayland/client.rs`
- 当前缺口：Wayland clipboard 仍通过阻塞式 `read_fd` 读 pipe。
- 当前证据：`crates/gpui/src/platform/linux/platform.rs:1059`、`crates/gpui/src/platform/linux/wayland/clipboard.rs:91`、`crates/gpui/src/platform/linux/wayland/client.rs:2127`
- 价值：避免 Firefox/外部应用挂起或半写入剪贴板时把 GPUI event loop 卡死。
- 风险：低到中。需把上游 `read_fd_with_timeout` 映射进当前 `platform/linux/platform.rs`，并替换 Wayland clipboard 和相关读取点。
- 验证：增加超时 pipe 单测；运行 `cargo test -p adabraka-gpui --lib --features test-support,wayland clipboard`。

### P0. Atlas tile 空间释放

- 上游提交：`0d8a4d4292`，`gpui: Free atlas tile space when removing tiles`
- 上游影响：`gpui_macos/src/metal_atlas.rs`、`gpui_wgpu/src/wgpu_atlas.rs`、`gpui_windows/src/directx_atlas.rs`
- 当前缺口：`remove` 只减少 refcount 或只在整张 texture 空闲时回收，未对单个 tile 调用 allocator deallocate。
- 当前证据：`crates/gpui/src/platform/mac/metal_atlas.rs:62`、`crates/gpui/src/platform/wgpu/wgpu_atlas.rs:135`、`crates/gpui/src/platform/windows/directx_atlas.rs:101`
- 价值：修复图片/纹理反复加载释放时 atlas 空间不能复用导致资源增长。
- 风险：中。三个后端实现相似但不完全一致，需分别补回归测试。
- 验证：移植上游 `test_remove_deallocates_tile_space_for_reuse` 思路；运行对应 atlas 单测，至少 `cargo test -p adabraka-gpui atlas`。

### P0. macOS fallback 字体级联修复

- 上游提交：`486cf9ef3c`，`Fix macOS system font fallback cascade append`
- 上游提交：`61ad9ebfcd`，`gpui_macos: Initialize fallback fonts with primary font weight and style`
- 上游影响：`gpui_macos/src/open_type.rs`
- 当前缺口：`append_system_fallbacks` 使用未消费的 `.map(...)`，系统 fallback 描述符不会 append；用户 fallback 也不继承 primary font 的 weight/style。
- 当前证据：`crates/gpui/src/platform/mac/open_type.rs:110`、`crates/gpui/src/platform/mac/open_type.rs:122`
- 价值：直接影响 macOS CJK/emoji/多语言 fallback 字体选择，尤其 bold/italic fallback。
- 风险：中。CoreText FFI 代码需谨慎处理 CF 对象生命周期。
- 验证：增加 `append_system_fallbacks` 不为空的可测封装或 macOS 字体 fallback 集成测试；运行 macOS `cargo test -p adabraka-gpui mac::text_system`。

### P1. IME candidate 视觉行锚点

- 上游提交：`737f55a1a1`，`gpui: Anchor IME candidate window to the start of the visual line`
- 相关后续：`126c0ee41a`、`137e677a05`（Wayland IME enable/多窗口处理）
- 上游影响：`gpui/src/platform.rs`、`gpui_linux/src/linux/wayland/window.rs`、`gpui_linux/src/linux/wayland/client.rs`
- 当前缺口：`selected_bounds` 仍以 selection end 为候选框位置；Wayland `get_ime_area` 仍只取 marked range start。
- 当前证据：`crates/gpui/src/platform.rs:1173`、`crates/gpui/src/platform/linux/wayland/window.rs:1156`
- 价值：避免中文/日文输入过程中 candidate 窗口随预编辑光标逐字横向跳动。
- 风险：中。涉及输入法行为，需在 macOS/Windows/Wayland 分别手动验证。
- 验证：新增 `compute_ime_candidate_bounds` 纯函数测试；Wayland 手动验证多窗口 IME。

### P1. List pending scroll rebase

- 上游提交：`7fd5ea4bf3`，`gpui: Fix list scroll events being reverted by pending scroll`
- 可选 API：`a1d2ef6514`，`item_is_above_viewport` / `item_is_below_viewport`
- 当前缺口：`reset()` 未清 `pending_scroll`；用户滚动与 remeasure 的 pending scroll 可能互相覆盖。
- 当前证据：`crates/gpui/src/elements/list.rs:331`、`crates/gpui/src/elements/list.rs:710`、`crates/gpui/src/elements/list.rs:1348`
- 价值：改善聊天、日志、agent streaming 列表在内容变高时的滚动平滑性。
- 风险：中。List 状态机敏感，必须带回归测试。
- 验证：移植上游 pending scroll/scroll wheel 测试；运行 `cargo test -p adabraka-gpui elements::list --features test-support`。

### P1. Tooltip stuck 与 show delay 配置

- 上游提交：`507d043d96`，`gpui: Fix stuck tooltips after mouse leaves origin`
- 上游提交：`b3bc83b57e`，`gpui: Make tooltip show delay configurable`
- 当前缺口：tooltip show delay 是硬编码常量；visible tooltip 状态不在 mouse move 时主动检查源元素是否已离开。
- 当前证据：`crates/gpui/src/elements/div.rs:48`、`crates/gpui/src/elements/div.rs:2815`
- 价值：独立 GPUI app 比 Zed 更容易暴露 tooltip 卡住问题；配置 show delay 也更符合组件库需求。
- 风险：低到中。主要在 `div.rs` / `text.rs` API 增量。
- 验证：新增 tooltip 离开源元素后隐藏的交互测试；运行 `cargo test -p adabraka-gpui tooltip --features test-support`。

### P1. Windows immovable window hit-test

- 上游提交：`9ac117693b`，`gpui_windows: Fix immovable windows skipping hit testing entirely`
- 当前缺口：当前 `handle_hit_test_msg` 在 `!self.is_movable` 时直接返回，导致 immovable window 的 close/min/max hit-test 也被跳过。
- 当前证据：`crates/gpui/src/platform/windows/events.rs:894`
- 价值：修复无拖拽窗口上的自绘标题栏按钮不可用。
- 风险：低。只需保留 `Drag` 受 `is_movable` 限制，按钮 hit-test 不受影响。
- 验证：Windows 下手动验证 immovable + hidden titlebar 的 close/min/max；若可 mock，增加 hit-test 单测。

### P1. Wayland 初始 `xdg_toplevel` app_id

- 上游提交：`35eaeb94a7`，`gpui: Fix xdg_toplevel app_id set to None at first commit on Wayland`
- 当前缺口：`WindowParams` 已携带 `app_id`，但 `create_xdg_toplevel_role` 没有在首个 `surface.commit()` 前调用 `toplevel.set_app_id`。
- 当前证据：`crates/gpui/src/window.rs:1110`、`crates/gpui/src/platform/linux/wayland/window.rs:491`
- 价值：KWin/Mutter 等窗口规则可在首帧正确识别 app id。
- 风险：低。
- 验证：Wayland 下设置 app_id 打开普通窗口，检查 compositor 规则或 `WAYLAND_DEBUG`。

### P2. Non-breaking glue characters 不换行

- 上游提交：`8036a3c74b`，`Add non breaking glue characters as non-wrapping characters`
- 当前缺口：`is_word_char` 未把 `U+202F`、`U+00A0`、`U+2011` 作为 non-wrapping glue。
- 当前证据：`crates/gpui/src/text_system/line_wrapper.rs` 当前缺少这些字符匹配。
- 价值：低风险文本排版修复。
- 风险：低。
- 验证：增加 line wrapper 单测覆盖 NBSP/NNBSP/NBH。

### P2. BoxShadow builder API

- 上游提交：`34cd17ff5e`，`gpui: Add builder API for BoxShadow`
- 当前状态：已有 inset shadow 和 `Rgba::alpha`，但未见 `BoxShadow` builder。
- 当前证据：`crates/gpui/src/style.rs:361`
- 价值：API ergonomics，不是稳定性修复。
- 风险：低。
- 验证：宏/样式编译测试。

## 架构级，建议单独专题

### AccessKit accessibility

- 上游提交：`1d029c5ff5`，`gpui: Accesskit support`
- 规模：约 19 个文件、1873 行新增。
- 2026-07-01 审计时状态：当时仓库只有 macOS 权限查询 API，尚无 GPUI element/window 级 AccessKit 树。
- 2026-07-10 后当前状态：AccessKit core、element/window tree、macOS/Linux/Windows adapter、示例和运行时禁用开关已经分别通过 `bcef0db`、`2a5f592`、`5fa6f9c`、`fd7fe81`、`016e86b`、`a738f78` 等提交完成；`accessibility` 也是 GPUI 默认 feature。
- 已完成证据：`crates/gpui/src/window/a11y.rs`、`crates/gpui/src/elements/div.rs`、`crates/gpui/src/platform/{mac,linux,windows}` 和 `crates/gpui/examples/a11y.rs`。
- 2026-07-24 bounded increment：`aria_description`、`aria_keyshortcuts` 和 `Window::debug_a11y_tree_json` 已按 GPUI-only 范围迁入，并有 AccessKit node/JSON focused tests。
- 剩余增量：上游 `2268045a119030735f762e5afaf59da0bda869f4` 中更广的 landmarks/menu、focus provenance 和 tab-group 行为仍未迁入。
- 建议：不要重复实施 AccessKit core；后续将 landmarks/menu 作为独立行为批次，不与本次描述、快捷键和 debug-tree API 混合。

### Benchmark / profiler / scheduler 后续

- 上游提交：`aa6f03bedd`、`297c4a4d78`、`39f7849a0f`、`48511e0b9c`、`c30d18b10d` 等。
- 当前状态：当前仓库已有本地 profiler、test app、visual test、scheduler 方向的迁移文档和首版实现。
- 建议：继续沿 `docs/todo/gpui-upstream-architecture-migration-evaluation.md` 的分阶段路线推进。
- 不建议：不要直接用上游 executor/scheduler 文件覆盖当前本地实现。

### gpui_web / web examples

- 上游相关：`gpui_web` 大量变更、`9a992ed33c` web examples build。
- 当前状态：当前仓库目标是 desktop GPUI + Adabraka 平台扩展，没有 web crate 映射。
- 建议：除非明确要支持 web 后端，否则不引入。

## 不建议同步或低价值项

| 上游主题 | 判断 | 原因 |
| --- | --- | --- |
| Zed 业务 UI 间接触发的 GPUI 小改 | 不直接引入 | 多数是 editor/agent/picker 业务需要，独立 GPUI 收益不明确 |
| README / web example 修复 | 暂缓 | 当前 README 已是 Adabraka 定制版，直接覆盖会丢本地定位 |
| typos / `Ok` 到 `OK` | 不优先 | 无框架稳定性收益 |
| `async-process` 移除 | 可后续依赖专项 | 当前仓库依赖体系不同，需先看 Cargo feature 影响 |
| AccessKit 一锅端 | 不混入 bugfix | 大范围 public API / platform dependency 变化 |

## 推荐执行批次

### 批次 A：P0 稳定性与资源修复

1. Wayland clipboard timeout（`bda5ac3626`）
2. Atlas tile deallocate（`0d8a4d4292`）
3. macOS fallback append + weight/style（`486cf9ef3c`、`61ad9ebfcd`）
4. Wayland initial app_id（`35eaeb94a7`）

建议验证：

```bash
cargo test -p adabraka-gpui atlas
cargo test -p adabraka-gpui --lib --features test-support
cargo check -p adabraka-gpui --no-default-features --features wgpu,wayland
```

### 批次 B：输入和交互修复

1. IME candidate bounds（`737f55a1a1`）
2. Wayland IME enable/multi-window 后续（`126c0ee41a`、`137e677a05`）
3. Windows immovable hit-test（`9ac117693b`）
4. Tooltip stuck + delay（`507d043d96`、`b3bc83b57e`）

建议验证：

```bash
cargo test -p adabraka-gpui tooltip --features test-support
cargo test -p adabraka-gpui --lib --features test-support,wayland,x11
cargo check -p adabraka-gpui --target x86_64-pc-windows-msvc
```

### 批次 C：列表和文本排版

1. List pending scroll rebase（`7fd5ea4bf3`）
2. List viewport helper APIs（`a1d2ef6514`，可选）
3. Non-breaking glue chars（`8036a3c74b`）
4. BoxShadow builder API（`34cd17ff5e`，可选）

建议验证：

```bash
cargo test -p adabraka-gpui elements::list --features test-support
cargo test -p adabraka-gpui line_wrapper
cargo test -p adabraka-gpui --lib --features test-support
```

### 批次 D：架构专题

1. AccessKit accessibility
2. 上游 bench/profiler 二期差异复核
3. scheduler / queue 与当前本地 priority 实现对齐

建议另开文档和分支，不和 bugfix 批次混合。

## 后续执行原则

- 每个上游主题单独提交，提交信息带上 Zed commit hash。
- 不覆盖 `app.rs`、`window.rs`、`platform.rs` 整文件，按函数/trait 手动合并。
- 平台实现保留 Adabraka 的 tray、global hotkey、daemon、resource profile、layer-shell 扩展。
- 每个 bugfix 至少带一个最小回归测试；没有自动化条件的平台修复，文档记录手动验证步骤。
- 优先跑 changed-area 测试，再跑 `cargo test -p adabraka-gpui --lib --features test-support`。
