# GPUI 上游架构能力迁移评估与总计划

日期：2026-05-17

## 背景

前一轮上游差异审计中，明确 bugfix 类缺口已经陆续迁入当前分支，包括：

- `PlatformDisplay::visible_bounds()`
- macOS prompt 默认焦点修正
- Linux wgpu recovery 拒绝软件渲染器
- Linux `buffer_font_fallbacks`
- `ElementId` 额外转换 API

剩余最大差异不再是单点 bugfix，而是 Zed 上游 GPUI 后续引入的一组架构能力：

- `app/headless_app_context.rs`
- `app/test_app.rs`
- `app/visual_test_context.rs`
- `platform/visual_test.rs`
- `platform_scheduler.rs`
- `profiler.rs`
- `queue.rs`

这组能力不应按文件机械搬运。当前仓库已经有自己的 daemon、tray、resource profile、headless、test platform、Linux wgpu 单 crate 版本和平台扩展；直接替换上游实现会破坏现有边界。更合适的方式是先评估效果和收益，再拆成可验证的小任务逐步迁移。

## 当前仓库已有能力

当前并不是从零开始：

- `Application::headless()` 已支持 headless mode。
- `TestAppContext` 已提供 `#[gpui::test]` 的测试上下文。
- `VisualTestContext` 已存在，用于测试窗口、绘制、输入与 bounds inspection。
- `platform/test/**` 已有 `TestPlatform`、`TestWindow`、`TestDispatcher`、`TestDisplay`。
- `executor.rs` 已提供 `BackgroundExecutor`、`ForegroundExecutor`、`Task`、`TaskLabel`、timer、test clock、`run_until_parked`。
- 当前仓库已有 `AppResourceProfile`、GPU cache trimming、text cache stats 等 Adabraka 独有资源控制能力。

因此迁移目标不是“补文件”，而是：

1. 让测试更稳定、更容易覆盖真实渲染和文本 shaping。
2. 让任务调度的优先级、时间、阻塞、测试推进更清晰。
3. 让性能、任务、frame、cache 行为可观测。
4. 保留当前仓库的单 crate 平台实现和产品扩展。

## 上游能力分组

| 分组 | 上游文件 | 主要效果 | 当前替代物 | 建议 |
| --- | --- | --- | --- | --- |
| 观测与诊断 | `profiler.rs` | 记录 task timing，支持增量采集和序列化 | 局部 `profiling` span、日志、cache stats | 优先做轻量接入 |
| 测试 API | `test_app.rs` | 更干净的测试入口、自动 flush effects、窗口检查和输入模拟 | `TestAppContext` | 可作为 wrapper 增量实现 |
| headless 测试 | `headless_app_context.rs` | 跨平台 headless app，支持真实 text system，可选 renderer/screenshot | `Application::headless()`、`TestAppContext` | 先补真实 text shaping 场景 |
| visual test | `visual_test_context.rs`、`platform/visual_test.rs` | 真实渲染截图、确定性调度、时间控制 | `VisualTestContext` + mocked test platform | 中期评估，不先做 snapshot baseline |
| 调度 | `platform_scheduler.rs` | 用 scheduler crate 包装平台 dispatcher，统一 task/timer/block | `executor.rs` + `PlatformDispatcher` | 最后做，先设计兼容层 |
| 优先级队列 | `queue.rs` | 多优先级 background queue、weighted pop、realtime 分流 | 当前 dispatcher queue 无通用 priority | 随 scheduler 做，不单独引入 |

## 统一开关与 cfg 策略

为避免四个任务各自引入不同 gating 方式，统一采用以下策略：

| 能力 | 编译策略 | 运行时策略 | 理由 |
| --- | --- | --- | --- |
| Profiler | 始终编译 | `AtomicBool` 全局开关，默认关闭 | 避免 feature flag 导致条件编译分叉，便于线上临时启用诊断 |
| TestApp / HeadlessTestApp | `#[cfg(any(test, feature = "test-support"))]` | 无额外开关 | 与现有 `TestAppContext` 一致，不进入普通生产 API |
| Visual Test | `#[cfg(any(test, feature = "test-support"))]` | 运行时能力探测，不支持则 skip | 真实渲染依赖平台、显示服务和 GPU，不应仅靠编译期判断 |
| Priority API | 始终编译 | 默认 `Medium`，现有调用行为不变 | 作为 executor API 扩展时保持 opt-in，便于评估和回退 |

## `profiling` crate 与新 profiler 的关系

当前仓库已经在多处使用 `#[profiling::function]` 和 `profiling::scope!`。新计划中的 `profiler.rs` 不是替代它，而是补充另一层观测：

- `profiling` crate：函数级 tracing / span 插桩，适合 Tracy、Superluminal 等后端分析具体调用栈和耗时。
- 新 `profiler.rs`：运行时 task-level timing 采集，记录任务 source location、线程、开始/结束时间，支持增量拉取。

两者应共存：函数级 tracing 用来深入分析热点，task-level profiler 用来回答“哪些 async task 在什么时候运行、排队和完成”。

## 预期收益

### 1. 对 bugfix 迁移的收益

后续从上游迁平台、渲染、文本、输入 bugfix 时，可以更容易新增回归测试。尤其是：

- cached view + Input 类生命周期问题。
- list scrolling 和 async layout 交错问题。
- text fallback、font metrics、shaping 差异。
- window lifecycle、focus、hover、tooltip、timer 行为。

### 2. 对 Adabraka 产品方向的收益

Adabraka 相比上游 GPUI 更强调后台/托盘/资源 profile/长期运行。观测和测试基础设施能直接服务这些方向：

- 用 profiler 观察 tray popup 首帧耗时、hidden window idle behavior、GPU/text cache trimming。
- 用 headless/test app 覆盖 daemon without windows、window show/hide、tray anchor positioning。
- 用 deterministic scheduler 降低 async UI 测试偶现失败。

### 3. 对工程维护的收益

- 后续同步上游时减少“凭肉眼判断”的风险。
- 将平台相关 smoke test 和核心 UI regression test 分离。
- 给 resource profile 提供可量化验证入口。

## 主要风险

### 1. scheduler 是底层手术

上游现在依赖 `scheduler` crate，并且 `PlatformDispatcher` 接口已经改为携带 `Priority` 和 `RunnableMeta`。更重要的是，上游已经 `pub use scheduler::Task`，这意味着 `Task<T>` 不再只是当前仓库这种 `async_task::Task<T>` wrapper。

当前仓库仍基于 `async_task::Runnable`、`TaskLabel` 和自有 `TestDispatcher`。如果直接替换：

- 会改动 `executor.rs`、`platform.rs`、所有平台 dispatcher。
- 会影响 `App::spawn`、`Window::spawn`、`AsyncApp`、test dispatcher。
- 会影响所有持有、返回、await、detach `Task<T>` 的代码。
- 可能破坏 daemon/tray/headless 的长期运行行为。

结论：scheduler/queue 只能作为最后一批，先做设计和兼容验证。完整 scheduler 引入不是天然渐进式任务，必须先通过 wrapper/adapter 降低切换面。

### 2. visual test 跨平台稳定性有限

真实截图测试会受到字体、DPI、GPU backend、系统主题、抗锯齿影响。当前仓库跨 macOS/Windows/Linux 的平台扩展更多，不能一开始就要求严格 pixel-perfect baseline。

结论：先把 visual test 用于 smoke 和结构化截图，再考虑局部 snapshot。

### 3. 上游 crate 边界不同

上游 GPUI 已拆分为 `gpui`、`gpui_platform`、`gpui_wgpu`、`gpui_linux`、`gpui_macos`、`gpui_windows` 等。当前仓库仍是单 crate 内嵌平台层。迁移时要按功能映射，不按路径覆盖。

具体影响：

- 上游 `PlatformDispatcher` / `RunnableMeta` 等接口可能已经位于拆分后的平台抽象中，当前仓库需要映射回 `crates/gpui/src/platform.rs`。
- 上游 `PlatformScheduler` 引入的依赖和 import 路径不能直接照搬。
- 上游 visual/headless 代码可能假设平台 crate 边界清晰，当前仓库需要保留内嵌平台实现和 Adabraka 扩展。
- 任何迁移步骤都应先列出“上游路径 -> 当前路径”的映射，避免误判 API 所属层级。

## 推荐迁移顺序

### Phase 0：评估和基线

目标：建立现状和验证基线，不改架构。

产物：

- 本总计划。
- 分任务计划：
  - `001-profiler-observability.md`
  - `002-test-app-headless.md`
  - `003-visual-test-platform.md`
  - `004-scheduler-queue.md`

验证：

- `cargo test -p fc-gpui-core window_positioner`
- `cargo check -p fc-gpui-core --no-default-features --features wgpu`
- 记录当前 `cargo fmt --check` 是否受既有无关格式化影响。

### Phase 1：Profiler / Observability

优先级最高。原因是收益清晰、侵入性可控，还能服务后续 scheduler 和 visual test 迁移。

策略：

- 不直接依赖上游 scheduler。
- 先引入轻量 task timing collector。
- 首版只从现有 `BackgroundExecutor::spawn_internal`、`ForegroundExecutor::spawn` 采集 task timing；timer、frame、render、cache 作为后续扩展。
- 始终编译，使用 `AtomicBool` 运行时开关，默认关闭。

成功标准：

- 可以采集 task start/end/location。
- 可以导出增量 timing delta。
- overhead 可控，默认关闭。

详细计划见 `001-profiler-observability.md`。

### Phase 2：TestApp / Headless 测试增强

目标是改善测试 ergonomics，不替换 `TestAppContext`。

策略：

- 在现有 `TestAppContext` 之上提供薄 wrapper，如 `TestApp`。
- 自动 flush effects 和 redraw。
- 提供清晰的 window handle inspection。
- 增强 headless 测试对真实 `PlatformTextSystem` 的支持。

成功标准：

- 新 API 可以覆盖一个非平凡窗口测试。
- 旧 `#[gpui::test]` 不受影响。
- 可写 text shaping/headless lifecycle 测试。

详细计划见 `002-test-app-headless.md`。

### Phase 3：Visual Test 平台

目标是把“真实渲染 smoke test”和“mocked structure test”分层。

策略：

- 保留现有 `VisualTestContext`。
- 增加平台能力探测：是否支持真实 renderer、是否支持 screenshot。
- macOS 可优先试点，Linux/Windows 只做 smoke 或 headless renderer adapter。
- 不一开始做大规模 pixel baseline。

成功标准：

- 可以打开屏幕外坐标窗口、推进调度、等待 frame、捕获截图或结构化 render artifact。
- CI 不因字体/DPI/GPU 差异产生大量 flake。

详细计划见 `003-visual-test-platform.md`。

### Phase 4：Scheduler / Queue

最后做。目标不是一次性替换 executor，而是逐步引入上游 scheduler 语义。

策略：

- 先定义当前 `PlatformDispatcher` 与上游 `Scheduler` 的映射。
- 优先采用 wrapper/adapter 模式，而不是第一步修改 `PlatformDispatcher` 签名。
- 加入 priority 概念，但保持现有 API 兼容。
- 将 `TaskLabel` 的测试优先级语义映射到 priority 或 test scheduler。
- 引入 queue 前先通过 executor/dispatcher adapter 验证 priority 行为，不要求第一步修改所有平台 dispatcher。

成功标准：

- 现有 task/timer/test API 兼容。
- priority task 可测试。
- test scheduler 能 deterministic 推进 foreground/background/timer。
- 不破坏 headless/daemon/tray 长期运行。

详细计划见 `004-scheduler-queue.md`。

## 总体验证矩阵

| 验证层 | 命令/方式 | 覆盖内容 |
| --- | --- | --- |
| 基础编译 | `cargo check -p fc-gpui-core --no-default-features --features wgpu` | 默认 host 编译、wgpu feature |
| 基础测试 | `cargo test -p fc-gpui-core window_positioner` | 快速 smoke，覆盖窗口定位 |
| test-support | `cargo test -p fc-gpui-core --lib --features test-support` | 更完整基线，覆盖测试上下文、executor、window tests |
| Linux 专项 | Linux 环境跑 `cargo test -p fc-gpui-core --lib --features test-support,wayland,x11` | Wayland/X11 dispatcher/window |
| macOS 专项 | 本机跑 visual/headless smoke | AppKit/Metal/截图/字体 |
| Windows 专项 | Windows runner 跑 display/device smoke | dispatcher、DirectX、monitor lifecycle |
| 性能基线 | profiler 开关下运行 sample app | overhead、首帧、cache trim |

## 不做事项

这轮计划不包含：

- 直接拆 crate 对齐上游 `gpui_*`。
- 一次性替换 `Task` 类型。
- 大规模 pixel-perfect visual snapshot。
- web/wasm 支持。
- 上游应用层 Zed editor/agent UI 相关改动。

## 决策建议

建议按以下顺序执行：

1. 先实现 `001-profiler-observability.md` 的最小可用版本。
2. 再实现 `002-test-app-headless.md`，用新测试 API 覆盖 profiler 和 headless text 场景。
3. 视 CI 和平台资源决定是否推进 `003-visual-test-platform.md`。
4. 等前三项稳定后，再启动 `004-scheduler-queue.md` 的设计和试点。

这样能先拿到观测和测试收益，同时避免一开始就改 executor/scheduler 这种高风险底层路径。
