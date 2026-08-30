# GPUI 迁移问题复审与后续方向判断

> 复审日期：2026-08-02
> 复审分支：`develop/0.9`
> 复审基线：`27afdb82f5c6de68af9ff959532767fc93a8beb5`
> 二次核查基线：`3fb949e3daac3f7a8b5f3b3e73c8be5e692b662d`
> 文档定位：这是对 2026-08-02 状态的历史复审和设计备忘，不是当前可直接执行的优先级清单。当前同步边界和启动门禁以 [`docs/sync/CURRENT.md`](../sync/CURRENT.md) 为准；较新的增量判断见 [`ZED_UPSTREAM_MIGRATION_REVIEW_2026-08-10.md`](../sync/ZED_UPSTREAM_MIGRATION_REVIEW_2026-08-10.md)。

## 0. 权威性、来源与证据边界

原始十项 finding 的逐字文本和外部评审链接没有随仓库材料入库，因此本文不能声称可独立还原原评审的完整措辞。下文的“十项 finding”是对当时十个主题的归纳；状态判断应通过当前仓库证据复核，而不是把编号本身当作权威来源。

| # | 本文归纳主题 | 当前可核查的仓库依据 |
|---|---|---|
| 1 | 公开 derive 路径 | [`upstream-aligned-crate-migration-goal.md`](./upstream-aligned-crate-migration-goal.md) Phase 1、[`upstream-aligned-crate-migration-tdd.md`](./upstream-aligned-crate-migration-tdd.md)、四类 downstream fixture |
| 2 | 平台注入与 facade 边界 | [`upstream-aligned-crate-migration-goal.md`](./upstream-aligned-crate-migration-goal.md) Phase 2–3、[`CURRENT.md`](../sync/CURRENT.md) 的永久 facade 决策 |
| 3 | DesktopServices | [`upstream-aligned-crate-migration-goal.md`](./upstream-aligned-crate-migration-goal.md) 的 Plan Rewrite Notes 和 non-goals |
| 4 | QuitMode 与结构化通知 | [`CURRENT.md`](../sync/CURRENT.md) 的 Structured system notifications 门禁、[`2026-08-10 审计`](../sync/ZED_UPSTREAM_MIGRATION_REVIEW_2026-08-10.md) §6.4 |
| 5 | parent-native popup 与 Dialog | [`CURRENT.md`](../sync/CURRENT.md) 的 Parent-anchored native popup 门禁、[`2026-08-10 审计`](../sync/ZED_UPSTREAM_MIGRATION_REVIEW_2026-08-10.md) §6.3 |
| 6 | 公开 feature 表面 | [`CHANGELOG.md`](../../CHANGELOG.md)、[`scripts/verify-features.sh`](../../scripts/verify-features.sh)、GPUI manifests |
| 7 | container query 与 View/ViewElement | [`CURRENT.md`](../sync/CURRENT.md) 的 View API/container query 门禁、[`2026-08-10 审计`](../sync/ZED_UPSTREAM_MIGRATION_REVIEW_2026-08-10.md) §6.6、[`gpui-upstream-architecture-migration-evaluation.md`](../todo/gpui-upstream-architecture-migration-evaluation.md) |
| 8 | scheduler | [`004-scheduler-queue.md`](../todo/004-scheduler-queue.md)、[`2026-08-10 审计`](../sync/ZED_UPSTREAM_MIGRATION_REVIEW_2026-08-10.md) §6.5、当前 executor/dispatcher/tests/benchmarks |
| 9 | web/mobile 产品边界 | [`CURRENT.md`](../sync/CURRENT.md) 的 Web/mobile 门禁 |
| 10 | 资源预算与桌面差异 | [`CHANGELOG.md`](../../CHANGELOG.md)、[`resource-profiles.md`](../resource-profiles.md)、crate migration goal 中必须保留的本地能力 |

如果以后找到原始评审来源，应把逐字 finding、来源 URL/commit 和本表编号补齐；在此之前，本文只记录本地复审结论，不替代当前同步规则。

## 1. 总结

按本文件对十个主题的归纳，复审基线上的状态为：

- **完全解决：4 项**：1、6、9、10
- **部分解决或采用替代方案：4 项**：2、4、5、8
- **未实现且复审时主动延期/否决：2 项**：3、7

需要注意：这不是说未完成项技术上无法解决，而是它们受到不同类型的约束：

- Finding 3（DesktopServices）：主要受限于收益证据和所有权模型，不是技术阻塞。
- Finding 4（结构化通知部分）：受限于三平台通知协议、激活回调和应用生命周期差异。
- Finding 8（scheduler）：受限于 executor/dispatcher 全链路改造、生产优先级语义和性能风险。
- Finding 5、7 值得拆开处理，不能作为两个整体大任务直接实施。

## 2. 十项 finding 在复审基线上的状态

| # | 状态 | 复审基线判断 |
|---|---|---|
| 1 | 已解决 | derive 使用动态 crate 解析，标准依赖、rename、underscore alias 和 facade/core 混合依赖均有 downstream fixture |
| 2 | 替代方案完成 | `with_platform`、facade 和平台 crate 拆分完成；`Application::new/headless` 被明确保留为永久 facade API，而不是弃用删除 |
| 3 | 未实现，主动否决 | 没有 `DesktopServices`/`DesktopAppExt`；能力仍由 `App` 直接转发到宽 `Platform` trait |
| 4 | 部分解决 | `QuitMode` 和 core 生命周期所有权完成；结构化通知、action、dismiss、response 尚未实现 |
| 5 | 部分解决 | 保留 Overlay/TrayAnchored；缺少 parent-native `AnchoredPopup` 和窗口级 Dialog 语义 |
| 6 | 已解决 | optional dependency feature 泄漏已治理；默认图片格式缩到 GIF/JPEG/PNG/WebP |
| 7 | 未实现，延期 | `container_query` 缺失；仍使用旧的 Entity/AnyView/Component 模型，没有统一 `ViewElement` |
| 8 | 部分解决，延期 | 有 test-only priority/fake-clock 原型；没有完整 `RunnableMeta`、生产优先级、realtime、统一时钟和 timer-resolution |
| 9 | 已解决为产品门禁 | web/mobile 没有进入当前交付边界，只有产品目标、demo、CI target 和 owner 明确后才启动 |
| 10 | 已解决 | 保留资源预算和桌面差异能力；GPU policy 已从 `WindowParams` 移至 application/platform/renderer 配置 |

## 3. 已解决项的证据摘要

### 3.1 Finding 1：公开 derive 路径

核心实现：

- `crates/gpui-macros/Cargo.toml` 使用 `proc-macro-crate`。
- `crates/gpui-macros/src/gpui_macros.rs` 同时解析 `fc-gpui` 和 `fc-gpui-core`。
- 处理 package 名、lib target 名和 dependency rename 的差异。
- facade/core 同时作为直接依赖时，支持通过 `[package.metadata.gpui-macros]` 显式选择。

真实 downstream fixtures：

- `tests/downstream-compat`
- `tests/downstream-renamed-compat`
- `tests/downstream-underscore-alias`
- `tests/downstream-core-with-facade-dev`

### 3.2 Finding 6：公开 feature 表面

当前默认图片格式仅为：

- GIF
- JPEG
- PNG
- WebP

其他格式通过独立 `image-format-*` feature opt-in。实现依赖使用 `dep:` 或语义 feature 聚合，`scripts/verify-features.sh` 维护公开 feature 白名单并在 CI 中执行。

### 3.3 Finding 9：web/mobile 门禁

当前没有 web/mobile platform crate、公开 feature、应用组合入口或 CI target。`docs/sync/CURRENT.md` 明确要求非桌面平台由产品目标触发。

WGPU 中可能存在 WASM substrate，但它不等于可交付的 GPUI web 后端。

### 3.4 Finding 10：Adabraka 差异与资源预算

继续保留：

- `AppResourceProfile`
- GPU cache trim/stats API
- Overlay
- Tray
- Global hotkey
- Permissions
- Single-instance
- Daemon/QuitMode

`WindowParams` 已不再传输 atlas 和 instance buffer 内存策略。GPU 预算通过：

```text
Application::with_resource_profile
    -> Platform::configure_gpu_resources
        -> backend renderer creation/configuration
```

传递。

## 4. Finding 2：注入边界的替代方案

当前已经建立：

- core `Application::with_platform`
- `fc-gpui-platform` 组合 facade
- `fc-gpui` 公共兼容 facade
- Linux/macOS/Windows/WGPU 独立 crate
- 锁步 GPUI crate 版本

原建议要求在旧版本弃用并删除 `Application::new/headless`，但当前项目做出了相反的正式决策：

- `Application::new/headless` 永久保留在公共 facade。
- core 不负责平台选择。
- `with_platform` 是高级注入入口。

因此依赖环和平台选择问题已经解决，但没有按原弃用路线执行。

## 5. Finding 3：DesktopServices 为什么没有实施

### 5.1 不是技术阻塞

可以定义：

```rust
pub struct DesktopServices {
    pub tray: Arc<dyn TrayService>,
    pub input: Arc<dyn DesktopInputService>,
    pub integration: Arc<dyn SystemIntegration>,
    pub notifications: Arc<dyn NotificationService>,
}
```

并通过 `Global` 注册、通过 `DesktopAppExt` 暴露。

### 5.2 当前主要限制

#### 容易成为第二套 Platform

如果 service 内部仍然只是调用 `Platform::set_tray_icon` 等方法，则调用链变成：

```text
App -> DesktopAppExt -> DesktopServices -> Service -> Platform
```

这只增加转发层，没有减少平台变化面。

#### 服务与 event loop/App 生命周期耦合

Tray、hotkey 和 notification callback 依赖：

- OS event loop
- foreground executor
- `App` 生命周期
- callback 注销与重新注册
- headless/test platform
- 应用退出时的销毁顺序

将服务放入 `Global` 前必须明确创建时机、唯一所有者和 callback 回到 `App` 的路径。

#### 迁移期会出现双重 ownership

如果旧 `Platform` API 和新 service API 长期并存，测试、backend 状态和 callback 注册可能出现两个真相来源。

### 5.3 重新启动条件

仅在以下证据出现时实施：

- `Platform` 因 tray/hotkey/notification/permission 频繁变化而持续制造跨 backend 修改。
- fake desktop capability 测试明显受限于完整 `Platform` mock。
- 至少两个 downstream 需要替换、装饰或独立注入桌面能力。
- 服务层能删除 `Platform` 中对应能力，而不是永久增加一层转发。

### 5.4 实施原则

- 只拆成 3–4 个粗粒度深服务。
- 禁止“一方法一 trait”。
- 旧 `App` API 作为兼容包装，但内部必须切换到 service。
- 完成迁移后从 `Platform` 删除对应方法，避免双轨。
- 如果最终只产生样板转发，应撤销该抽象。

## 6. Finding 4：结构化通知的限制

`QuitMode` 已完成；本节只讨论未完成的结构化通知。

期望契约可能包括：

```rust
pub struct SystemNotification {
    pub tag: Option<SharedString>,
    pub title: SharedString,
    pub body: SharedString,
    pub actions: Vec<NotificationAction>,
}

pub enum NotificationResponse {
    Activated,
    Action(SharedString),
    Dismissed,
}
```

### 6.1 平台差异

#### Linux

需要接入 D-Bus notification action/close signals，并处理 notification daemon 能力差异。不同 daemon 对 action、replace-id、close reason 的支持不一致。

#### macOS

需要长期持有 `UNUserNotificationCenter` delegate，处理 permission、category/action identifier、bundle identity，以及点击通知后的激活或冷启动。

#### Windows

当前 tray balloon 无法提供完整 action/tag/dismiss/response。需要 Windows Toast、AppUserModelID、COM/WinRT、activation callback，部分场景还需要 Start Menu shortcut 或 packaged identity。

### 6.2 跨平台生命周期问题

Notification response 到达时，应用可能：

- 正常运行
- 无窗口 daemon 运行
- 正在退出
- 已退出，需要冷启动

因此需要稳定路径：

```text
OS callback
    -> backend notification registry
        -> foreground executor
            -> App callback / single-instance activation
```

### 6.3 推荐分阶段实现

1. 建立 core contract 和 fake backend。
2. 保留 `show_notification(title, body)` 作为 simple wrapper。
3. 第一阶段只提供 tag/replace 和 dismiss。
4. 第二阶段增加 action/response。
5. 最后处理冷启动 activation。

在实施前必须确定：

- unsupported action 是返回错误还是降级。
- tag 是否跨进程/重启稳定。
- dismiss 是否必须产生 response。
- 应用未运行时点击通知如何进入应用。

## 7. Finding 8：scheduler 的限制

当前生产接口仍然是：

```rust
fn dispatch(&self, runnable: Runnable, label: Option<TaskLabel>);
```

`TaskPriority` 主要在 test dispatcher 中生效。Linux/macOS 基本忽略 label；Windows 固定使用高优先级 thread pool，label 主要用于日志。

### 7.1 当前已保留的 metadata 与仍缺失的统一边界

当前 `async_task` 调度闭包会捕获 `Option<TaskLabel>`，task 每次 wake/reschedule 时都会重新进入同一个 `dispatcher.dispatch(runnable, label)`，因此已实现的 `TaskPriority` 不会因为 wake 而丢失。`TimedFuture` 也会跨 poll 保留 source location 和 profiling timing handle。

这不等于所有 metadata 已经统一，也不等于生产平台执行了 caller-selected priority：

- `TestDispatcher` 和 test-support `ThreadedDispatcher` 会按 high/medium/low 选择队列。
- Linux 和 macOS production dispatcher 忽略 `TaskLabel`。
- Windows 保留 label 用于日志，但所有 background work 仍固定提交到 high-priority thread pool。
- task kind、dispatcher-visible source location、独立 profiling ID、queued-at timing 等统一字段仍不存在。

因此，`RunnableMeta` 可以作为未来统一和上游对齐的候选，但**不能仅以“当前 metadata 在 wake 后丢失”为理由强制引入**。是否需要新的 runnable envelope，应先由 dispatcher 必须读取的字段和回归测试决定。

### 7.2 复审基线之后已补充的 test-support 基础

当前 HEAD 已经包含：

- `ThreadedDispatcher`：真实 worker threads、真实 timer、external wake、bounded main-thread drain，以及 high/medium/low test-support 队列。
- `crates/gpui/benches/async_tasks.rs`：foreground completion、background completion 和 64-task batch 的 Criterion baseline。

这些是 completion/throughput 基线，不是 production scheduler，也不满足 p95/p99、starvation/fairness、timer/external-wake benchmark 或可失败 CI threshold。

#### 所有 dispatcher 必须同时适配

迁移会影响：

- core executor
- test dispatcher
- test-support `ThreadedDispatcher`
- Linux dispatcher
- macOS dispatcher
- Windows dispatcher
- foreground/timer/blocking 路径
- fake clock
- profiler

#### Production priority 需要真实队列语义

不仅是增加枚举，还需要：

- 多队列或 priority queue
- starvation prevention
- timer task priority
- wake/reschedule 保持 priority
- shutdown 时 pending task 行为

#### Realtime 需要 OS 专用实现

- Linux 调度权限
- macOS QoS/realtime policy
- Windows MMCSS/thread priority

同时必须限制 realtime task 中的锁、分配、日志和阻塞 I/O。

#### 统一时钟和 timer resolution 是独立风险

统一 `Instant` 会影响 timeout、animation、fake clock、profiler 和 debounce。Windows timer resolution 还需要引用计数 guard，避免长时间高分辨率导致耗电。

### 7.3 推荐迁移顺序

1. 先增加 self-wake/reschedule 回归测试，证明现有 label/priority、source location 和 profiling timing state 的保留行为。
2. 列出必须对 dispatcher 可见、但现有 scheduling closure 或 wrapped future 无法承载的字段；只有存在明确缺口时才选择 `RunnableMeta` 或更小的 metadata envelope。
3. 扩充 `ThreadedDispatcher` benchmark：p95/p99、starvation/fairness、timer、external wake 和 cancellation；保持它为 test-support 基础设施。
4. 逐平台实现 caller-selected production priority，并验证 wake/reschedule、timer 和 shutdown 语义。
5. 独立加入 dedicated/realtime execution。
6. 最后迁统一 `Instant` 和 timer-resolution guard。

禁止将 scheduler 与平台目录/crate 拆分绑定在同一批次。

### 7.4 前置条件

- 在现有 Criterion completion/throughput baseline 之外增加 task dispatch p95/p99 benchmark。
- starvation/fairness 测试。
- delayed timer + priority 组合测试。
- 三平台至少有 compile/runtime validation。
- CI 中存在可失败的性能回归门槛，而不是只打印 benchmark 数据。

## 8. Finding 5 是否值得解决

### 8.1 判断

**parent-native `AnchoredPopup` 有明确架构价值，但当前仍是条件立项；窗口级 `Dialog` 继续暂缓。**

启动条件以 [`CURRENT.md`](../sync/CURRENT.md) 为准：必须先出现可复现的 parent-native popup 定位、焦点、关闭或跟随问题，并能指出将被替换的现有 workaround 或真实 downstream consumer。在该证据出现前，不把它列为普通 P1 实施项。

Adabraka 的 tray、daemon、overlay 和小工具窗口产品形态，天然需要：

- popup 相对父窗口定位
- 正确焦点和激活
- 点击外部关闭
- 父窗口移动/关闭联动
- Wayland parent surface、positioner 和 dismissal

当前 `TrayAnchored` 只表达屏幕坐标，不能表达 GPUI parent popup 的协议和生命周期。

### 8.2 必须保留两条模型

```text
GPUI AnchoredPopup
    parent = GPUI native surface
    Wayland = xdg_popup

TrayAnchored panel
    parent = 外部 desktop shell
    没有 GPUI parent surface
    不得伪装成 xdg_popup
```

建议目标：

```rust
pub enum WindowKind {
    Normal,
    PopUp,
    Floating,
    Overlay,
    AnchoredPopup(AnchoredPopupOptions),
    LayerShell(LayerShellOptions),
}
```

`AnchoredPopup` 是 additive variant，不替代或删除现有公开 `PopUp`。两者和其他窗口模型的边界为：

| 模型 | parent/owner | 主要语义 |
|---|---|---|
| `PopUp` | 当前没有显式 GPUI parent | 现有平台兼容 popup；激活和层级由平台及 `WindowOptions::focus` 决定，不表达 native anchored-popup 协议 |
| `Floating` | 平台现有 floating 语义 | 浮动在普通窗口之上，但不表达 Wayland `xdg_popup` 协议 |
| `AnchoredPopup` | 必须是仍存活的 GPUI native window | parent-relative anchor、lifecycle、dismiss 和原生 owner/child 协议 |
| `Overlay` | 无 parent 要求 | 跨窗口或全屏上层 overlay |
| `TrayAnchored` | 外部 desktop shell tray anchor | 屏幕/显示器定位；不是 GPUI parent surface |

任何未来移除或合并 `PopUp` 的提议都必须单独提供兼容/弃用计划和 `CHANGELOG.md` 迁移映射。

`AnchoredPopupOptions` 至少包含：

- parent window handle
- anchor rect
- anchor
- gravity
- constraint adjustment
- focus/dismiss policy

### 8.3 暂缓 WindowKind::Dialog

当前已有系统消息框 `App::show_dialog(DialogOptions)`。只有出现下列真实需求时再增加 GPUI modal window：

- macOS sheet
- Windows owner HWND/modal taskbar semantics
- GPUI 自绘 modal child window
- 父窗口关闭时必须联动销毁

如果新 `Dialog` 只是 `Floating` 的别名，则不值得增加。

## 9. Finding 7 是否值得解决

Finding 7 必须拆成两个任务。

### 9.1 container_query：条件 P2，当前延期

只有出现第一个真实消费组件，或某个必须迁移的上游修复被它阻塞时才启动；不能仅凭 API 对齐把它放入近期固定队列。

它改动面相对小，并直接支持：

- tray/settings/sidebar 小尺寸布局
- 同一组件在不同容器宽度下复用
- 减少父组件传 breakpoint
- 减少组件读取全窗口尺寸或依赖 Global 状态

第一 proof point 应验证：

1. query 使用容器尺寸而不是 window 尺寸。
2. 容器尺寸变化触发重新布局。
3. 嵌套 query scope 不串。
4. cached view 不复用过期 query 结果。

如果最小 container query 必须先重写大部分 View/cache 管线，应暂停并重新评估耦合。

### 9.2 View/ViewElement：禁止立即全量迁移，原型必须有退出机制

当前模型：

```text
Entity<V: Render> -> Element
AnyView           -> Element
RenderOnce        -> Component<C>
```

长期统一为 `ViewElement` 的潜在收益：

- 降低上游 View API 补丁的人工转换成本。
- 统一 stateful/stateless identity、cache、layout、prepaint 和 paint 路径。
- 降低修复只落到 Entity 或 RenderOnce 一侧的风险。

但迁移影响：

- `view.rs`
- `element.rs`
- derive macros
- `IntoElement`
- `AnyView`
- `Entity<T>`
- cached view
- inspector identity
- downstream generic bounds

最危险的结果是旧 `Component` 与新 `ViewElement` 永久双轨。只有后续同步成本已经形成可量化证据时，才允许在隔离分支或 internal/test-only gate 下做兼容原型，只迁 2–3 个内部组件，并用后续两次上游同步量化是否减少映射文件和人工转换。

两次同步后必须二选一，不允许无限期共存：

- **接受**：指定 owner、里程碑和兼容 adapter 生命周期，排期完成全量内部迁移，并在完成时删除旧 `Component`/兼容路径。
- **拒绝**：回退已迁原型组件，删除 `ViewElement` prototype/compatibility path，保留当前单一实现。

## 10. 当前权威门禁下的优先级

本文不再为 `AnchoredPopup`、`container_query` 或 `ViewElement` 创建无条件近期 P1。当前顺序必须先服从 [`CURRENT.md`](../sync/CURRENT.md) 和较新的增量审计：

### 当前固定优先事项

- 保持跨平台 CI、正确性修复、发布归档和 runtime smoke 门禁可执行。
- 先处理有失败测试、可复现行为或明确 release contract 的工作。

### 条件 P2：container_query

- 启动证据：命名的 downstream consumer，或被 container query 阻塞的必须迁移上游修复。
- 作为独立 core capability 移植，不与 ViewElement 迁移绑定。

### 条件 P3：AnchoredPopup

- 启动证据：可复现的 parent-native 定位、焦点、关闭或跟随问题，以及将被替换的 workaround/consumer。
- 立项后覆盖 GPUI parent handle、anchor/gravity/constraint、focus/dismiss、Wayland `xdg_popup`、macOS child/panel 和 Windows owner HWND。
- 不修改 `TrayAnchored` 的外部 shell 模型，也不替代 `PopUp`。

### 当前 No-go：ViewElement 全量迁移

- 只有同步成本数据证明当前 View 差异持续产生显著人工成本时，才允许 internal/test-only 小范围原型。
- 原型必须按 §9.2 在两次同步后接受并全量迁移，或拒绝并完全删除；不存在永久兼容双轨选项。

### 条件 P2/P3：结构化通知

先确定最低跨平台契约，再按 tag/dismiss、action/response、冷启动三个阶段实施。

### P3：窗口级 Dialog

仅由真实 modal/sheet 产品需求触发。

### 独立运行时项目：Scheduler

只有性能基线、metadata contract 和三平台验证准备好后启动，不与上述 UI capability 批次合并。

## 11. 明确禁止的做法

- 不要把 `TrayAnchored` 重命名或合并为 `AnchoredPopup`。
- 不要用 `Floating` 加几个字段假装实现 Wayland popup。
- 不要为了 container query 先迁完整 View API。
- 不要让 `Component<C>` 与 `ViewElement<C>` 永久双轨。
- 不要创建只转发到 `Platform` 的 DesktopServices。
- 不要一次同时迁窗口协议、View identity/cache 和 scheduler。
- 不要在缺少 benchmark gate 时宣称 scheduler 性能无回退。

## 12. 验证与停止条件

### 立项后的 First proof points

1. AnchoredPopup core contract：closed/stale parent 在 native popup 创建前确定性返回错误；关闭 live parent 会 dismiss/destroy child；焦点变化不改变声明的 parent。
2. AnchoredPopup Wayland smoke：跟随父窗口、点击外部关闭，并确认使用 `xdg_popup`/`xdg_positioner`。
3. AnchoredPopup macOS smoke：验证 native child/panel 关系、parent move/close linkage 和 focus/dismiss 行为。
4. AnchoredPopup Windows smoke：验证 owner HWND、owner-close linkage、activation/focus，以及不产生独立 taskbar entry。
5. Container query demo：同一窗口内，窄容器与宽容器呈现不同布局，并覆盖 resize、嵌套 scope 和 cached result 失效。
6. ViewElement 原型：迁少量组件，记录两次上游同步的映射文件数量，并在决策点执行“全量迁移或完全删除”。

### Falsifiers

以下证据出现时应暂停或降级：

- downstream 没有 parent popup 或容器响应布局需求时，不得启动对应项目。
- AnchoredPopup 无法替换任何现有 workaround。
- container query 必须依赖完整 View 重写。
- 两次上游同步表明 View 差异没有造成显著人工成本。
- ViewElement 原型导致 cached view 或绘制性能明显回退。
- 两次同步后无法证明 ViewElement 原型收益时，必须删除原型和 compatibility path，而不是继续双轨。
- DesktopServices 只增加转发样板，没有减少 `Platform` 变化。

## 13. 验证记录

初次审计基线 `af488d99f81b1bc292820bf65bd849a0bdee3c69` 上：

- 本地 `scripts/verify-migration.sh` 通过。
- core 205 tests 通过。
- `scripts/verify-features.sh` 通过。
- 四类 downstream fixture 通过。
- 当时 GitHub Linux jobs 因 `OwnedMenu` import 缺失而失败。

随后：

- `c8aa2a2` 修复 Linux `OwnedMenu` import。
- `27afdb8` 清理 downstream fixture dead-code warnings。
- 复审当时 HEAD `27afdb8` 的 GitHub `GPUI feature matrix` 全部成功：run `30710300409`。

复审基线之后、本文入库之前又补充了：

- `644aff1`：test-support `ThreadedDispatcher`，覆盖真实线程、timer、external wake、bounded reschedule 和 priority queues。
- `b6b6936`：基于 `ThreadedDispatcher` 的 async completion/throughput Criterion benchmark。
- `2353e22`、`6d166da`、`b18557e`：真实视觉 smoke、lint/visual gate 和 feature contract gate。

二次核查基线 `3fb949e` 确认这些能力已经存在，因此本文 scheduler inventory 和优先级已按当前事实修正；当前可执行方向仍以 `CURRENT.md` 和 2026-08-10 审计为准。

仍需注意：

- `cargo-semver-checks` 需要在发布前单独运行。
- 当前没有可执行失败的 scheduler/render performance threshold。
- tray/hotkey/notification/single-instance 等真实 OS 行为仍需要平台 runtime smoke 或人工产品回归。

## 14. 收益账单

| 动作 | 当前代价 | 获得的具体收益 | 收益出现时机 |
|---|---:|---|---|
| 条件立项独立 AnchoredPopup | 三平台窗口协议实现与测试 | 解决 GPUI parent popup 的定位、焦点、层级和关闭语义 | 出现可复现问题且能替换现有 workaround 后 |
| 保留独立 TrayAnchored | 继续维护独立 tray 路径 | 避免错误地给外部 shell tray panel 套用 `xdg_popup` | Wayland tray 场景 |
| 条件移植 container query | 少量 core layout 状态和测试 | 组件不再依赖 window breakpoint 或父级手工传尺寸 | 出现第一个窄/宽容器真实消费组件后 |
| 有退出门禁的 ViewElement 小范围原型 | 兼容层和迁移实验 | 用真实数据判断是否减少上游 View 补丁转换 | 同步成本证据成立后；两次同步内接受或删除 |
| 暂缓 WindowKind::Dialog | 放弃短期 API 完整感 | 避免创造与 `Floating` 无实质区别的变体 | 直到真实 modal/sheet 需求出现 |
| Scheduler 独立迁移 | 额外 benchmark、平台适配和阶段验证 | 避免平台拆分与运行时语义同时回归 | 性能基线和 metadata contract 建立后 |

## 最终方向

当前固定投入方向是：

```text
修复/保持跨平台 CI、正确性、发布与 runtime smoke 门禁
    -> 出现真实 container consumer 时：container_query
    -> 出现可复现 parent-native popup 问题时：AnchoredPopup
    -> 同步成本证据成立时：ViewElement internal/test-only 原型
         -> 两次同步后全量迁移或完全删除
```

以上箭头是条件门禁，不是已经承诺的近期排期。DesktopServices、结构化通知和完整 scheduler 都可以解决，但必须分别满足“抽象收益证据”“跨平台产品契约”和“性能/运行时基线”后再启动。窗口级 Dialog 不应为了 API 对齐而实施，应由真实 modal/sheet 产品需求触发。
