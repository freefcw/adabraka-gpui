# 004 - Scheduler / Queue 迁移任务

## 目标

评估并逐步引入上游 scheduler / priority queue 语义，统一 foreground/background/timer/block 的调度模型，同时保持当前 `Task`、`BackgroundExecutor`、`ForegroundExecutor`、`PlatformDispatcher` 和测试 API 的兼容性。

这是最高风险任务，应在 profiler 和测试能力增强之后执行。

## 上游参考

- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/platform_scheduler.rs`
- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/queue.rs`
- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/executor.rs`
- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/platform.rs`

上游主要变化：

- GPUI executor 基于 `scheduler` crate。
- `PlatformScheduler` 包装 `PlatformDispatcher`。
- `Runnable` 携带 `RunnableMeta`，包含 source location。
- dispatcher API 接收 `Priority`。
- `queue.rs` 提供 weighted priority queue。
- 支持 `Priority::RealtimeAudio` 等优先级语义。
- 上游 `Task<T>` 来自 scheduler crate，而不是当前仓库的 `async_task::Task<T>` wrapper。

## 当前仓库现状

当前已有：

- `Task<T>` 基于 `async_task::Task<T>`。
- `BackgroundExecutor::spawn` / `spawn_labeled`。
- `ForegroundExecutor::spawn`。
- `TaskLabel` 用于测试中 deprioritize。
- `PlatformDispatcher`：
  - `dispatch(runnable, Option<TaskLabel>)`
  - `dispatch_on_main_thread(runnable)`
  - `dispatch_after(duration, runnable)`
  - `park`
  - `unparker`
  - `now`
- `TestDispatcher` 已支持：
  - foreground/background 随机推进。
  - delayed timer。
  - fake clock。
  - `run_until_parked`。
  - `allow_parking`。
  - `deprioritize(TaskLabel)`。

当前缺口：

- 没有生产路径 task priority。
- background queue 没有统一 priority/weighted behavior。
- source location 主要用于 foreground local task panic，不是统一 runnable metadata。
- scheduler/test scheduler 语义没有统一抽象。
- realtime priority 没有通用模型。

## 预期效果

完成后应具备：

- background task 可以带 priority。
- executor/dispatcher adapter 可以携带 priority。
- test dispatcher 可以 deterministic 地验证 priority 行为。
- timer/block/clock 语义更接近上游。
- profiler 能从 runnable metadata 获取 source location。

## 收益

- 后台/daemon 场景可以降低低优先级任务对 UI 首帧和 tray popup 的影响。
- 测试中可以更明确地控制任务顺序。
- 为 future 上游迁移减少 executor 差异。
- 让 profiler、test_app、visual_test 的调度基线更稳定。

## 风险

- 影响面极大：`executor.rs`、`platform.rs`、`platform/*/dispatcher.rs`、`app.rs`、`test_context.rs`。
- 可能改变 task 执行顺序，引入隐蔽行为变化。
- 可能破坏平台 event loop 的线程约束。
- 当前 Adabraka 独有 daemon/tray/headless 长期运行路径可能依赖现有调度行为。
- 引入上游 `scheduler` crate 会增加依赖和 API 迁移成本。
- 如果完整切换到上游 scheduler `Task<T>`，所有 `Task<T>` 调用点都会受影响，这不是天然渐进式改动。
- `block_with_timeout` 当前依赖 `TestDispatcher` fake clock；scheduler 自带 `Clock` 后可能与现有语义冲突。

## 设计原则

- 不在第一步替换 `Task<T>`。
- 不在第一步引入上游 scheduler crate 作为强依赖。
- 先用 wrapper/adapter 承载 priority 和 scheduler-like 语义，避免第一步修改 `PlatformDispatcher` 签名。
- 保留 `TaskLabel`，直到测试迁移完成。
- 所有生产 priority 默认 `Medium`，保持现有行为近似。
- priority API 必须 opt-in。若收益不足，可移除或保持 no-op，不影响现有功能正确性。

## 迁移计划

### Step 0：行为基线

在改动前补测试：

- foreground task 在主线程运行。
- background task 可被 `run_until_parked` 推进。
- timer 按 fake clock 到期。
- `TaskLabel::deprioritize` 生效。
- `block_with_timeout` 在 test dispatcher 下行为稳定。
- `block_with_timeout` 与 `advance_clock` / delayed timer 的交互明确受测。

这些测试是后续重构安全网。

### Step 1：引入本地 `TaskPriority`

新增 enum：

- `TaskPriority::High`
- `TaskPriority::Medium`
- `TaskPriority::Low`

暂不做 `RealtimeAudio`。

扩展 API：

- 首版先提供 crate-private 或 `#[doc(hidden)]` 的 `BackgroundExecutor::spawn_with_priority(priority, future)`。
- `BackgroundExecutor::scoped_priority` 只有在首版验证后再添加，避免过早扩大 public API。

保留：

- `spawn` 等价于 `Medium`
- `spawn_labeled` 继续用于测试

### Step 2：引入 executor/dispatcher wrapper

不要把第一版目标设为直接修改 `PlatformDispatcher` 签名。当前接口是：

```rust
fn dispatch(&self, runnable: Runnable, label: Option<TaskLabel>);
fn dispatch_on_main_thread(&self, runnable: Runnable);
```

直接目标是在 executor 层保留 priority metadata，并通过 wrapper/adapter 转发到现有 dispatcher：

```rust
struct DispatcherPriorityAdapter {
    dispatcher: Arc<dyn PlatformDispatcher>,
}
```

adapter 内部提供：

```rust
fn dispatch(&self, runnable: Runnable, priority: TaskPriority, label: Option<TaskLabel>);
fn dispatch_on_main_thread(&self, runnable: Runnable, priority: TaskPriority);
```

迁移方式：

- production adapter 首版可忽略 priority，转发到现有 dispatcher。
- `TestDispatcher` 可以通过 adapter 或 `TaskLabel` metadata 实际使用 priority，先验证收益。
- 等 API 稳定后，再决定是否把 priority 下沉到 `PlatformDispatcher` trait。
- 该路线与上游 `PlatformScheduler` wrapper 模式更接近，后续若引入 scheduler crate，改动面更小。

### Step 3：引入 priority queue

参考上游 `queue.rs`，但先做本地最小版本：

- high/medium/low 三队列。
- `try_pop`。
- `pop`。
- weighted 或 strict priority 需要明确选择。

建议：

- test dispatcher 使用 strict 或 deterministic weighted，便于测试。
- production background dispatcher 可先按平台能力忽略 priority，再逐步改。
- 在实现前估算改动量：当前 `TestDispatcher` 约 300 行，上游 `TestScheduler` 更复杂，首版 priority queue 应控制在局部替换 background queue，不重写整个 test scheduler。

### Step 4：替换 test dispatcher background queue

当前 `TestDispatcher` 使用：

- `background: Vec<Runnable>`
- `deprioritized_background: Vec<Runnable>`

目标：

- priority queues。
- `TaskLabel` deprioritize 映射为 low priority 或单独 delayed queue。
- 保留随机化以暴露 race，但权重可控。

测试：

- high priority 更早执行。
- low priority 不会 starvation。
- deprioritized label 仍晚于普通 task。

### Step 5：评估引入上游 scheduler crate

在本地 priority 和 profiler 稳定后，再判断是否需要完整 `scheduler` crate。

评估标准：

- 当前 executor 是否仍难以维护。
- 是否需要 `SessionId`、`Clock`、`Timer`、`TestScheduler`。
- 是否有上游后续 patches 强依赖 scheduler。

如果决定引入：

- 先 adapter 到当前 `PlatformDispatcher`。
- 保持 public `Task` alias 或 wrapper 兼容。
- 分平台逐个验证。

## 验证方案

### 改动前基线命令

```bash
cargo test -p adabraka-gpui executor
cargo test -p adabraka-gpui dispatcher
cargo test -p adabraka-gpui --lib --features test-support
cargo check -p adabraka-gpui --no-default-features --features wgpu
```

### Step 1/2 验证

新增测试：

- `spawn_with_priority_defaults_to_existing_behavior`
- `foreground_priority_does_not_break_main_thread_constraint`
- `background_priority_is_observable_in_test_dispatcher`
- `spawn_labeled_deprioritize_still_works`
- `priority_adapter_can_be_disabled_or_noop_without_behavior_change`

### Step 3/4 验证

新增测试：

- high priority task runs before low priority in deterministic mode。
- low priority eventually runs。
- delayed timer respects fake clock even with priority queues。
- `run_until_parked` drains all priority queues。
- `block_with_timeout` respects fake clock with delayed timers。

### 回退策略

如果 Step 1-3 后发现 priority 收益不足或复杂度过高：

- 保留 `TaskPriority::Medium` 默认行为，停止下沉到 production dispatcher。
- 因首版 `spawn_with_priority` 是 crate-private 或 `#[doc(hidden)]`，可以隐藏/移除而不产生稳定 API 破坏。
- 因为现有调用默认 Medium 且 priority 为 opt-in，回退不应影响功能正确性。
- 已新增的基线测试继续保留，作为 executor 行为安全网。

### 平台 smoke

macOS：

```bash
cargo test -p adabraka-gpui --lib --features test-support
```

Linux：

```bash
cargo test -p adabraka-gpui --lib --features test-support,wayland,x11
cargo check -p adabraka-gpui --no-default-features --features wgpu
```

Windows：

```powershell
cargo test -p adabraka-gpui --lib --features test-support
```

### 行为验证

手动跑：

- 打开窗口。
- 显示/隐藏 tray popup。
- 触发 timer animation。
- 触发 background asset/image load。
- 观察 profiler 中高/中/低 priority task 是否按预期。

## 完成标准

- 现有 public executor API 兼容。
- priority API 可用但默认行为不变。
- `TestDispatcher` 能验证 priority。
- `TaskLabel` 旧语义保留。
- 没有一次性引入完整 scheduler 导致大范围破坏。
- 明确记录是否继续推进完整 scheduler crate；如果不推进，说明本地 adapter 的保留边界。

## 不建议事项

- 不建议直接把上游 `executor.rs` 覆盖当前文件。
- 不建议立刻把 `Task<T>` 替换成 scheduler crate 的 `Task<T>`。
- 不建议第一批加入 realtime audio priority。
- 不建议在没有 profiler 和测试基线前改 production platform dispatcher 行为。

## 后续扩展

- 引入 `RunnableMeta`，统一 source location。
- profiler 从 runnable meta 采集 timing。
- 用 scheduler crate 替换本地 adapter。
- 支持 realtime priority 或专用线程。

---

## 执行与验证增强

以下是对原始计划的补充，针对具体实现路径和验证覆盖进行增强。

### 上游 Task 类型替换的影响面

上游 `executor.rs` 已经 re-export scheduler 的类型：

```rust
pub use scheduler::{FallibleTask, ForegroundExecutor as SchedulerForegroundExecutor, Priority, Task};
```

这意味着上游的 `Task<T>` 已经不是 `async_task::Task<T>` 的包装，而是 scheduler crate 自己的类型。如果当前仓库最终要引入 scheduler：

- 所有 `use adabraka_gpui::Task` 的代码都会受影响
- `Task::detach()`、`Task::ready()`、await 行为可能有细微差异
- `Scope` 的实现需要完全重写（当前基于 `async_task::spawn`）

这不是渐进可做的——要么不引入 scheduler 的 Task，要么一次性替换。因此 Step 5 的评估标准应包含"替换 Task 类型的全局影响评估"。

### Step 2 推荐方案：Wrapper 层而非修改 trait

不要把修改 `PlatformDispatcher` trait 作为首版方案。可能路径包括：
- A) 直接在 `dispatch()` 签名加 priority 参数
- B) 新增 `dispatch_with_priority()` 方法带默认实现

还有第三种更安全的路径：

**推荐方案：在 executor 层处理 priority，不改 PlatformDispatcher trait**

当前 `TaskLabel` 是 `NonZeroUsize` 纯标识符，无法直接携带 priority 信息。有两种扩展方式：

- **方式 a**：扩展 `TaskLabel` 为 `struct TaskLabel { id: NonZeroUsize, priority: TaskPriority }`，dispatch 时 TestDispatcher 检查 label 中的 priority 字段。
- **方式 b**：在 executor 层维护一个 `HashMap<TaskLabel, TaskPriority>` 侧表，spawn 时注册 priority，TestDispatcher 查表获取。

方式 a 更简单直接，推荐首选。示意：

```rust
impl BackgroundExecutor {
    pub(crate) fn spawn_with_priority<R: Send + 'static>(
        &self,
        priority: TaskPriority,
        future: impl Future<Output = R> + Send + 'static,
    ) -> Task<R> {
        // 构建携带 priority 的 label
        let label = TaskLabel::new_with_priority(priority);
        self.spawn_internal(Box::pin(future), Some(label))
    }
}
```

这种方式的优势：
- 零破坏性：`PlatformDispatcher` trait 不变，所有平台实现不需要改
- 只有 `TestDispatcher` 需要感知 priority（检查 label 中的 priority 信息）
- 如果后续引入 scheduler，替换路径更清晰（直接替换 executor 实现）
- 与上游最终方案（`PlatformScheduler` 包装层）方向一致

此方案的局限：
- 生产环境的平台 dispatcher 无法真正按 priority 调度
- 但这正是 Step 5 评估要解决的问题——在没确定需要真实 priority 调度前，不应提前改接口

**推荐选择此方案**，除非有明确场景需要生产 dispatcher 感知 priority。

### Step 0 完整基线测试清单

当前 `TestDispatcher` 的 `tick` 方法使用随机选择，`run_until_parked` 是 `while self.tick(false) {}`。基线测试需要覆盖：

```rust
// 基本调度行为
#[gpui::test]
async fn baseline_foreground_task_runs_on_main_thread(cx: &mut TestAppContext)

#[gpui::test]
async fn baseline_background_task_completes_after_run_until_parked(cx: &mut TestAppContext)

#[gpui::test]
async fn baseline_timer_fires_after_advance_clock(cx: &mut TestAppContext)

#[gpui::test]
async fn baseline_deprioritize_delays_labeled_task(cx: &mut TestAppContext)

#[gpui::test]
async fn baseline_block_with_timeout_respects_fake_clock(cx: &mut TestAppContext)

// Adabraka 特有场景
#[gpui::test]
async fn baseline_headless_app_tasks_complete_without_window(cx: &mut TestAppContext)

#[gpui::test]
async fn baseline_scoped_tasks_complete_before_scope_returns(cx: &mut TestAppContext)

// 确定性验证
#[gpui::test(iterations = 20)]
async fn baseline_same_seed_produces_same_task_order(cx: &mut TestAppContext)
```

这些测试在任何 scheduler 改动前必须存在且通过，作为安全网。

### Step 1 的 TaskPriority 与现有 TaskLabel 的关系

当前 `TaskLabel` 已用于 `deprioritize()`。引入 `TaskPriority` 后两者的关系：

```rust
/// TaskPriority 是显式的调度优先级。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskPriority {
    High,
    #[default]
    Medium,
    Low,
}

/// TaskLabel 仍用于测试中按标签引用 task。
/// deprioritize(label) 等价于将该 label 的 task 降为 Low priority。
```

映射规则：
- `spawn()` → `Medium` priority
- `spawn_labeled(label)` → `Medium` priority + label
- crate-private `spawn_with_priority(High, ...)` → `High` priority
- `deprioritize(label)` → 将匹配 label 的 task 从当前 priority 降为 `Low`

### Step 3/4 TestDispatcher 改造

当前 `TestDispatcher` 的背景队列：

```rust
struct TestDispatcherState {
    background: Vec<Runnable>,              // 普通任务
    deprioritized_background: Vec<Runnable>, // 被 deprioritize 的任务
}
```

引入 priority 后变为：

```rust
struct TestDispatcherState {
    high_background: Vec<Runnable>,
    medium_background: Vec<Runnable>,     // 原 background
    low_background: Vec<Runnable>,        // 原 deprioritized_background
}
```

`tick()` 的选择逻辑需要从当前的"普通 > deprioritized"变为"high > medium > low"，但保留随机性以暴露竞态：

```rust
// 简化的 priority-aware tick 逻辑
fn tick(&self) -> bool {
    // 1. 处理到期的 delayed task
    // 2. 计算各 priority 队列的长度
    // 3. 使用加权随机选择：
    //    - high 权重 = high_len * 4
    //    - medium 权重 = medium_len * 2
    //    - low 权重 = low_len * 1
    // 4. 从选中的队列随机取一个 runnable 执行
}
```

这比 strict priority（高全部跑完才跑中）更适合测试，因为它能暴露依赖优先级顺序的隐蔽 bug。

### 回退策略

如果 Step 1-3 做完后发现 priority 收益不大：

- `TaskPriority` enum 和 crate-private `spawn_with_priority` 可保留（零成本抽象，不影响性能）
- TestDispatcher 的 priority queue 可以通过配置退化为原始行为：

```rust
impl TestDispatcher {
    /// 设置是否启用 priority-aware 调度。
    /// 禁用时行为等同于改动前（所有 task 等权重随机执行）。
    pub fn set_priority_scheduling(&self, enabled: bool) { ... }
}
```

- 所有现有调用默认 `Medium`，行为不变
- 可以在评估后完全移除 priority 逻辑而不影响功能正确性

### Gate 测试

Step 1（TaskPriority enum + spawn_with_priority）：

```rust
#[gpui::test]
async fn spawn_with_priority_defaults_to_medium_behavior(cx: &mut TestAppContext)

#[gpui::test]
async fn spawn_with_priority_high_is_observable_in_test_dispatcher(cx: &mut TestAppContext)

#[gpui::test]
async fn spawn_labeled_deprioritize_still_works_after_priority_addition(cx: &mut TestAppContext)
```

Step 3/4（priority queue in TestDispatcher）：

```rust
#[gpui::test(iterations = 30)]
async fn high_priority_task_runs_before_low_in_deterministic_mode(cx: &mut TestAppContext)

#[gpui::test(iterations = 30)]
async fn low_priority_task_eventually_runs(cx: &mut TestAppContext)

#[gpui::test]
async fn delayed_timer_respects_fake_clock_with_priority_queues(cx: &mut TestAppContext)

#[gpui::test]
async fn run_until_parked_drains_all_priority_queues(cx: &mut TestAppContext)
```

注意 `iterations = 30`：由于 TestDispatcher 使用随机选择，需要多次运行验证统计趋势。

### block_with_timeout 交互验证

注意：在 test 模式下，`block_with_timeout` 并不使用真实计时器超时。它的实现是通过 `gen_block_on_ticks()` 生成一个随机 tick 预算，预算耗尽后返回 `Err(remaining_future)`。因此验证的重点不是"超时是否按 Duration 生效"，而是"priority queue 改变后 tick 循环是否仍能正常终止"：

```rust
#[gpui::test]
async fn block_with_timeout_returns_bounded_with_priority_queues(cx: &mut TestAppContext) {
    let executor = cx.executor();
    // pending future 永远不会完成，block_with_timeout 应在 tick 预算耗尽后返回 Err
    let result = executor.block_with_timeout(
        Duration::from_millis(100),
        futures::future::pending::<()>()
    );
    assert!(result.is_err(), "should return Err after tick budget exhaustion");
}

#[gpui::test]
async fn block_with_timeout_completes_ready_future_with_priority_queues(cx: &mut TestAppContext) {
    let executor = cx.executor();
    // 可完成的 future 应正常返回 Ok
    let task = executor.spawn(async { 42 });
    let result = executor.block_with_timeout(Duration::from_millis(100), task);
    assert_eq!(result.unwrap(), 42);
}
```

### 验证脚本

```bash
#!/bin/bash
# scripts/verify-004.sh
set -e
echo "=== 004 - Scheduler / Queue ==="

echo "[1/6] Compile checks..."
cargo check -p adabraka-gpui
cargo check -p adabraka-gpui --no-default-features
cargo check -p adabraka-gpui --no-default-features --features wgpu

echo "[2/6] Baseline tests (must pass before AND after changes)..."
cargo test -p adabraka-gpui --lib --features test-support -- baseline_

echo "[3/6] Priority tests..."
cargo test -p adabraka-gpui --lib --features test-support -- spawn_with_priority
cargo test -p adabraka-gpui --lib --features test-support -- priority

echo "[4/6] Dispatcher tests..."
cargo test -p adabraka-gpui --lib --features test-support -- dispatcher
cargo test -p adabraka-gpui --lib --features test-support -- executor

echo "[5/6] Frozen tests (regression guard)..."
cargo test -p adabraka-gpui --lib --features test-support -- app::test
cargo test -p adabraka-gpui --lib --features test-support -- elements::list
cargo test -p adabraka-gpui --lib --features test-support -- keymap

echo "[6/6] Full lib test..."
cargo test -p adabraka-gpui --lib --features test-support

echo "=== 004 ALL PASSED ==="
```
