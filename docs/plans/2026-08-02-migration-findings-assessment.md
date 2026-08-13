# GPUI 迁移问题复审与后续方向判断

> 日期：2026-08-02  
> 当前分支：`develop/0.9`  
> 当前基线：`27afdb82f5c6de68af9ff959532767fc93a8beb5`  
> 用途：记录对原十项迁移 finding 的复审、未完成项的约束，以及窗口模型和 View 能力的后续优先级判断，供未来上游同步、版本规划和架构决策参考。

## 1. 总结

严格按照原十项 finding 的目标判断，当前状态为：

- **完全解决：4 项**：1、6、9、10
- **部分解决或采用替代方案：4 项**：2、4、5、8
- **未实现且当前主动延期/否决：2 项**：3、7

需要注意：这不是说未完成项技术上无法解决，而是它们受到不同类型的约束：

- Finding 3（DesktopServices）：主要受限于收益证据和所有权模型，不是技术阻塞。
- Finding 4（结构化通知部分）：受限于三平台通知协议、激活回调和应用生命周期差异。
- Finding 8（scheduler）：受限于 executor/dispatcher 全链路改造、生产优先级语义和性能风险。
- Finding 5、7 值得拆开处理，不能作为两个整体大任务直接实施。

## 2. 十项 finding 状态

| # | 状态 | 当前判断 |
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
- `crates/gpui-macros/src/gpui_macros.rs` 同时解析 `adabraka-gpui` 和 `adabraka-gpui-core`。
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
- `adabraka-gpui-platform` 组合 facade
- `adabraka-gpui` 公共兼容 facade
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

### 7.1 主要技术限制

#### Metadata 必须跟随 runnable 的每次 reschedule

Priority、source location、profiling ID 和 task kind 不能只在首次 spawn 时传递，否则 future 被 wake 后会丢失语义。因此需要类似 `Runnable<RunnableMeta>` 的绑定模型。

#### 所有 dispatcher 必须同时适配

迁移会影响：

- core executor
- test dispatcher
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

### 7.2 推荐迁移顺序

1. 只引入 `RunnableMeta`，保持旧生产调度行为。
2. 先迁 test dispatcher，验证 wake 后 metadata 不丢失。
3. 逐平台实现 production priority。
4. 独立加入 dedicated/realtime execution。
5. 最后迁统一 `Instant` 和 timer-resolution guard。

禁止将 scheduler 与平台目录/crate 拆分绑定在同一批次。

### 7.3 前置条件

- task dispatch p95 benchmark。
- starvation/fairness 测试。
- delayed timer + priority 组合测试。
- 三平台至少有 compile/runtime validation。
- CI 中存在可失败的性能回归门槛，而不是只打印 benchmark 数据。

## 8. Finding 5 是否值得解决

### 8.1 判断

**值得优先解决 parent-native `AnchoredPopup`，暂缓窗口级 `Dialog`。**

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
    Floating,
    Overlay,
    AnchoredPopup(AnchoredPopupOptions),
    LayerShell(LayerShellOptions),
}
```

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

### 9.1 container_query：值得近期移植

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

### 9.2 View/ViewElement：战略上值得，但不应立即全量迁移

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

最危险的结果是旧 `Component` 与新 `ViewElement` 永久双轨。因此应先做兼容原型，只迁 2–3 个内部组件，并用后续两次上游同步量化是否减少映射文件和人工转换。

## 10. 推荐优先级

在保持 CI 全绿的前提下：

### P1：AnchoredPopup

- GPUI parent handle
- anchor/gravity/constraint
- focus/dismiss contract
- Wayland 真正 `xdg_popup`
- macOS child/panel semantics
- Windows owner HWND popup
- 不修改 TrayAnchored 的外部 shell 模型

### P1：container_query

作为独立 core capability 移植，不与 ViewElement 迁移绑定。

### P2：ViewElement 兼容原型

- 保留现有 downstream 写法。
- 内部尝试统一进入 ViewElement。
- 只迁少量内部组件。
- 用上游同步成本和缓存/绘制性能决定是否全迁。

### P2/P3：结构化通知

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

### First proof points

1. GPUI parent popup demo：跟随父窗口、点击外部关闭、Wayland 使用 `xdg_popup`。
2. Container query demo：同一窗口内，窄容器与宽容器呈现不同布局。
3. ViewElement 原型：迁少量组件，并记录下一次上游同步的映射文件数量。

### Falsifiers

以下证据出现时应暂停或降级：

- downstream 没有 parent popup 或容器响应布局需求。
- AnchoredPopup 无法替换任何现有 workaround。
- container query 必须依赖完整 View 重写。
- 两次上游同步表明 View 差异没有造成显著人工成本。
- ViewElement 原型导致 cached view 或绘制性能明显回退。
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
- 当前 HEAD `27afdb8` 的 GitHub `GPUI feature matrix` 全部成功：run `30710300409`。

仍需注意：

- `cargo-semver-checks` 需要在发布前单独运行。
- 当前没有可执行失败的 scheduler/render performance threshold。
- tray/hotkey/notification/single-instance 等真实 OS 行为仍需要平台 runtime smoke 或人工产品回归。

## 14. 收益账单

| 动作 | 当前代价 | 获得的具体收益 | 收益出现时机 |
|---|---:|---|---|
| 增加独立 AnchoredPopup | 三平台窗口协议实现与测试 | 解决 GPUI parent popup 的定位、焦点、层级和关闭语义 | 第一个菜单、popover 或父窗口 popup |
| 保留独立 TrayAnchored | 继续维护独立 tray 路径 | 避免错误地给外部 shell tray panel 套用 `xdg_popup` | Wayland tray 场景 |
| 移植 container query | 少量 core layout 状态和测试 | 组件不再依赖 window breakpoint 或父级手工传尺寸 | 第一个窄/宽容器复用组件 |
| ViewElement 小范围原型 | 兼容层和迁移实验 | 用真实数据判断是否减少上游 View 补丁转换 | 后续一到两次上游同步 |
| 暂缓 WindowKind::Dialog | 放弃短期 API 完整感 | 避免创造与 `Floating` 无实质区别的变体 | 直到真实 modal/sheet 需求出现 |
| Scheduler 独立迁移 | 额外 benchmark、平台适配和阶段验证 | 避免平台拆分与运行时语义同时回归 | 性能基线和 metadata contract 建立后 |

## 最终方向

近期最值得投入的是：

```text
修复/保持跨平台 CI
    -> AnchoredPopup
    -> container_query
    -> ViewElement 小范围原型与量化
```

DesktopServices、结构化通知和完整 scheduler 都可以解决，但必须分别满足“抽象收益证据”“跨平台产品契约”和“性能/运行时基线”后再启动。窗口级 Dialog 不应为了 API 对齐而实施，应由真实 modal/sheet 产品需求触发。
