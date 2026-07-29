# 001 - Profiler / Observability 迁移任务

## 目标

引入一套轻量、默认关闭、可增量采集的 GPUI 运行时观测能力。首版只覆盖 async task timing；timer、frame、layout/render/cache 观测作为后续扩展，为 scheduler、visual test、资源 profile 验证提供数据基础。

## 上游参考

- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/profiler.rs`

上游 `profiler.rs` 主要能力：

- 记录 `TaskTiming`：source location、start、end。
- 按线程维护 ring buffer。
- 提供 `ThreadTaskTimings` 和序列化结构。
- `ProfilingCollector` 支持只采集上次之后新增的 timing delta。
- 支持把 source location 序列化为 file/line/column。

## 与现有 `profiling` crate 的关系

当前仓库已有 `#[profiling::function]` 和 `profiling::scope!`，例如 window、text system、wgpu atlas 等路径。新 profiler 与它们不是替代关系：

- `profiling` crate：函数级 tracing，偏编译期插桩和外部 profiler 后端。
- 本任务的 profiler：运行时 task-level timing，偏 async task 生命周期、线程和增量采集。

实现时可以在关键路径同时保留 `profiling` span，并额外记录 task timing。不要为了新 profiler 移除现有 `profiling` 标记。

## 当前仓库现状

当前仓库已有：

- `executor.rs` 中 `BackgroundExecutor`、`ForegroundExecutor`、`Task`。
- `spawn_local_with_source_location` 已经记录 foreground task spawn location，用于 panic 诊断。
- resource profile 和 cache stats：
  - `TextSystem::layout_cache_stats`
  - GPU/cache trimming 相关实现
- 局部 `profiling::function` 标记，例如 Linux text system。

当前缺口：

- 没有统一 task timing collector。
- 没有按线程/任务位置聚合耗时。
- 没有增量导出机制。
- 没有把 frame/render/layout/cache 行为串起来观察的入口。

## 预期效果

完成后应具备：

- 可以开启 profiling，观察 foreground/background task 耗时。
- 可以定位慢 task 的 source location。
- 可以增量拉取 timing delta，供日志、debug UI 或测试断言使用。
- 可以为 resource profile 验证提供可量化数据。

## 收益

- 后续 scheduler 迁移有真实基线，不是凭感觉判断。
- 能定位 tray popup 首帧慢、hidden window idle task、cache trim 后首帧抖动等问题。
- 能验证 profiler 自身 overhead 是否可接受。
- 能辅助测试 flake：看任务是否在预期顺序和时机运行。

## 风险

- 如果每个 task 都无条件记录，可能增加锁竞争和分配。
- 如果 API 暴露过早，后续 scheduler 迁移会产生兼容负担。
- 如果和上游 scheduler 强绑定，会把本任务拖入高风险范围。

## 设计原则

- 默认关闭。
- 始终编译，使用 `AtomicBool` 全局运行时开关控制。
- 不引入上游 scheduler 依赖作为前置条件。
- 数据结构可参考上游，但接入点适配当前 `executor.rs`。
- API 先 `#[doc(hidden)]` 或 crate-private，稳定后再决定是否公开。
- 定义 `type ProfilingInstant = std::time::Instant`，把后续迁移到 `scheduler::Instant` 的成本集中在一处。

## 迁移计划

### Step 1：引入数据模型

新增候选文件：

- `crates/gpui/src/profiler.rs`

最小结构：

- `TaskTiming`
- `ThreadTaskTimings`
- `SerializedLocation`
- `SerializedTaskTiming`
- `SerializedThreadTaskTimings`
- `ThreadTimingsDelta`
- `ProfilingCollector`

与上游差异：

- `Instant` 通过 `type ProfilingInstant = std::time::Instant` 间接引用。若后续完整引入 scheduler，可集中改为 `scheduler::Instant`。
- `SharedString` 使用当前仓库类型。
- ring buffer 大小受 `AppResourceProfile` 或固定 debug 常量控制，先不做复杂配置。
- 全局开关使用 `AtomicBool`，例如 `profiler::set_enabled(bool)` / `profiler::is_enabled()`。

线程存储策略：

- 使用 `thread_local!` 保存当前线程的 `Arc<parking_lot::Mutex<ThreadTimingState>>`。
- `ThreadTimingState` 内部保存 ring buffer，例如 `VecDeque<TaskTiming>`、线程名、`total_pushed`。
- 全局 registry 保存 `Weak<parking_lot::Mutex<ThreadTimingState>>`，collector 采集时升级 weak 并短暂加锁读取快照。
- background thread pool 线程数不固定时，不要求稳定线程集合；线程退出后对应 weak entry 会自然失效，collector 跳过即可。

### Step 2：接入 executor

修改候选文件：

- `crates/gpui/src/executor.rs`

接入点：

- `BackgroundExecutor::spawn_internal`
- `ForegroundExecutor::spawn`
- `BackgroundExecutor::timer`

实现方式：

- 在 task future 外包一层 timing guard。
- start 在第一次 poll 或 task 创建时记录，需要明确语义。
- end 在 future 完成或 drop 时记录。
- source location 优先复用现有 `ForegroundExecutor::spawn` / `spawn_local_with_source_location` 已取得的 caller location，避免建立第二条 `#[track_caller]` 链路。
- background task 目前没有同等 source-location wrapper，可给 `spawn_internal` 增加 `#[track_caller]` 传参，但要保持 public API 行为不变。
- `record_start` 必须返回 `TaskTimingHandle`，由 `TimedFuture` 持有；完成或 drop 时用 handle 结束对应 timing，不能只用 source location 匹配 start/end。

建议语义：

- `start` 记录 task 第一次 poll 的时间，而不是 spawn 时间。
- 另行保留 `queued_at` 可作为后续扩展，不在首版做。
- task 被 drop/cancel 时，允许 `end` 记录 drop 时间，并在状态字段中区分 completed/cancelled；如果首版不加状态，也应在文档里说明 `end` 只代表 task wrapper 结束。
- 同一个 callsite 可以同时 spawn 多个 task；测试必须覆盖同一 location 的并发 task 不会互相覆盖 timing。

### Step 3：提供采集 API

候选 API：

- `App::collect_profiler_timings(&mut ProfilingCollector) -> Vec<ThreadTimingsDelta>`
- 或 crate-private `profiler::global_timings()`

建议先从 crate-private 做起，只给测试和 debug 工具用。

### Step 4：接入 frame/cache 关键点

候选位置：

- `Window::draw`
- scene build / paint / layout 阶段
- `TextSystem::layout_cache_stats`
- GPU resource trimming

首版只做 task timing；frame/cache 作为第二个 commit。

## 验证方案

### 单元测试

新增测试覆盖：

- profiling 关闭时不记录 timing。
- profiling 开关可运行时打开/关闭，不依赖 feature flag。
- profiling 开启时，background task 完成后有 start/end。
- foreground task source location 非空。
- `ProfilingCollector::collect_unseen` 第二次调用只返回新增数据。
- ring buffer wrap 后不会 panic，cursor 落后时返回仍保留的数据。
- thread-local buffer 在多个 background 线程上采集不 panic，退出线程被跳过。
- 同一 source location 并发 task 的 start/end 通过 handle 正确配对。

### 集成测试

使用 `TestAppContext`：

- spawn 多个 foreground/background task。
- `run_until_parked`。
- 收集 timings。
- 断言至少包含任务 source location 和非零 duration。

### 命令

```bash
cargo test -p adabraka-gpui-core profiler
cargo test -p adabraka-gpui-core --lib --features test-support profiler
cargo check -p adabraka-gpui-core --no-default-features --features wgpu
```

### 手动验证

- 在 sample app 或最小窗口 app 中开启 profiler。
- 打开/关闭窗口、显示 tray popup、触发 cache trim。
- 确认日志或 debug dump 中可以看到 task timing。

## 当前状态

已完成首版实现（2026-05-17）：

- 新增 `crates/gpui/src/profiler.rs`，包含 task timing 数据模型、线程本地 ring buffer、全局 registry、运行时开关和增量 `ProfilingCollector`。
- `BackgroundExecutor::spawn` / `spawn_labeled`、`ForegroundExecutor::spawn`、`BackgroundExecutor::timer` 已接入 `TimedFuture`，记录 first-poll 到 completed/cancelled 的 timing。
- profiler 默认关闭；开启后通过 `gpui::profiler::set_enabled(true)` 和 `ProfilingCollector` 采集增量 timing。
- 未引入 scheduler crate，未替换 `Task<T>` 或 `PlatformDispatcher` API。
- 验证脚本：`scripts/verify-001.sh`。

已验证：

```bash
cargo test -p adabraka-gpui-core profiler
cargo test -p adabraka-gpui-core --lib --features test-support -- profiler
cargo check -p adabraka-gpui-core --no-default-features --features wgpu
```

结果：全部通过。

## 完成标准

- profiling 默认关闭，不改变现有行为。
- 开启后能采集 task timing。
- 有最小测试覆盖增量采集。
- 不引入 scheduler crate 作为强依赖。
- 不改变 public `Task` / executor API，除非有明确兼容说明。
- 明确记录与现有 `profiling` crate 的互补关系。

## 后续扩展

- 将 profiler 数据展示到 debug overlay。
- 与 `AppResourceProfile` 对接，采集 cache trim 前后指标。
- scheduler 迁移后把 timing source 从 executor wrapper 下沉到 scheduler runnable meta。

---

## 执行与验证增强

以下是对原始计划的补充，针对具体实现路径和验证覆盖进行增强。

### Step 1 Gate 测试

Step 1 产出 `profiler.rs` 数据模型，完成时必须通过以下测试（在 `profiler.rs` 内 `#[cfg(test)] mod tests` 中实现）：

```rust
#[test] fn task_timing_records_start_and_end()
#[test] fn thread_task_timings_append_and_retrieve()
#[test] fn ring_buffer_wraps_without_panic()
#[test] fn collector_returns_only_unseen_delta()
#[test] fn collector_after_wrap_returns_remaining_data()
#[test] fn serialized_location_captures_file_line_column()
```

这些测试独立于 executor，纯数据结构验证。

### Step 2 实现路径

当前 `spawn_internal` 实现：

```rust
fn spawn_internal<R: Send + 'static>(&self, future: AnyFuture<R>, label: Option<TaskLabel>) -> Task<R> {
    let dispatcher = self.dispatcher.clone();
    let (runnable, task) = async_task::spawn(future, move |runnable| dispatcher.dispatch(runnable, label));
    runnable.schedule();
    Task(TaskState::Spawned(task))
}
```

推荐的 timing 注入方式：在 future 外包一层 `TimedFuture`，并用 handle 关联 start/end。

```rust
struct TimedFuture<F> {
    inner: F,
    location: &'static Location<'static>,
    started: bool,
    timing: Option<TaskTimingHandle>,
}

impl<F: Future> Future for TimedFuture<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if !this.started {
            this.started = true;
            this.timing = profiler::record_start(this.location);
        }
        let result = unsafe { Pin::new_unchecked(&mut this.inner).poll(cx) };
        if result.is_ready() {
            if let Some(timing) = this.timing.take() {
                profiler::record_end(timing, TaskTimingStatus::Completed);
            }
        }
        result
    }
}

impl<F> Drop for TimedFuture<F> {
    fn drop(&mut self) {
        if let Some(timing) = self.timing.take() {
            profiler::record_end(timing, TaskTimingStatus::Cancelled);
        }
    }
}
```

关键设计决策：

- `record_start` 在第一次 poll 时调用（task 真正开始执行），不是 spawn 时。
- 全局开关关闭时，`record_start`/`record_end` 内部通过 `AtomicBool::load(Relaxed)` 短路返回，开销约 1 条原子读指令。
- 复用 `ForegroundExecutor::spawn` 已有的 `#[track_caller]` 获取 location，不需要额外 caller chain。
- `spawn_local_with_source_location` 已经有 `Location::caller()` 捕获逻辑，可作为参考。

Step 2 Gate 测试：

```rust
#[gpui::test]
async fn profiler_disabled_records_nothing(cx: &mut TestAppContext)

#[gpui::test]
async fn profiler_enabled_records_background_task_timing(cx: &mut TestAppContext)

#[gpui::test]
async fn profiler_enabled_records_foreground_task_location(cx: &mut TestAppContext)

#[gpui::test]
async fn profiler_timing_start_is_first_poll_not_spawn(cx: &mut TestAppContext)
```

### Step 3 API 暴露

采集 API 放在 `#[cfg(any(test, feature = "test-support"))]` 下：

```rust
#[cfg(any(test, feature = "test-support"))]
pub fn profiler_collect_timings(collector: &mut ProfilingCollector) -> Vec<ThreadTimingsDelta> { ... }
```

这样测试可以调用，生产代码不暴露不稳定接口。未来稳定后再决定是否公开。

### 性能开销验证

profiler 的核心价值之一是"overhead 可控"。验证方式：

```rust
#[ignore = "performance smoke; too noisy for default CI"]
#[test]
fn profiler_overhead_is_acceptable() {
    use std::time::Instant;
    let mut cx = TestAppContext::single();
    let executor = cx.executor();

    // 关闭 profiler，spawn 10000 task
    profiler::set_enabled(false);
    let start = Instant::now();
    for _ in 0..10_000 {
        executor.spawn(async {}).detach();
    }
    cx.run_until_parked();
    let baseline = start.elapsed();

    // 开启 profiler，spawn 10000 task
    profiler::set_enabled(true);
    let start = Instant::now();
    for _ in 0..10_000 {
        executor.spawn(async {}).detach();
    }
    cx.run_until_parked();
    let with_profiler = start.elapsed();

    // 额外开销应 < 50%（宽松阈值，覆盖 CI 波动）
    assert!(with_profiler < baseline * 3 / 2,
        "profiler overhead too high: baseline={:?}, with_profiler={:?}", baseline, with_profiler);
}
```

注意：此测试在 CI 环境可能波动较大，只作为 ignored/manual smoke，不作为默认 gate。如果在真实环境中超出，需要优化 thread-local 写入路径。

### 并发安全验证

profiler 涉及多线程 thread-local 写入和全局 collector 读取：

```rust
#[gpui::test(iterations = 50)]
async fn profiler_concurrent_spawns_do_not_corrupt_data(cx: &mut TestAppContext) {
    profiler::set_enabled(true);
    // 并发 spawn 多个 background task
    // collect timings
    // 验证每条 timing 的 start <= end，location 非空
}
```

### 验证脚本

```bash
#!/bin/bash
# scripts/verify-001.sh
set -e
echo "=== 001 - Profiler / Observability ==="

echo "[1/5] Compile checks..."
cargo check -p adabraka-gpui-core
cargo check -p adabraka-gpui-core --no-default-features
cargo check -p adabraka-gpui-core --no-default-features --features wgpu

echo "[2/5] Profiler unit tests..."
cargo test -p adabraka-gpui-core --lib --features test-support -- profiler

echo "[3/5] Profiler integration tests..."
cargo test -p adabraka-gpui-core --lib --features test-support -- profiler_enabled
cargo test -p adabraka-gpui-core --lib --features test-support -- profiler_disabled
cargo test -p adabraka-gpui-core --lib --features test-support -- profiler_concurrent

echo "[4/5] Frozen tests (regression guard)..."
cargo test -p adabraka-gpui-core --lib --features test-support -- app::test
cargo test -p adabraka-gpui-core --lib --features test-support -- executor
cargo test -p adabraka-gpui-core --lib --features test-support -- elements::list

echo "[5/5] Full lib test..."
cargo test -p adabraka-gpui-core --lib --features test-support

echo "(Optional) Profiler overhead smoke:"
echo "  cargo test -p adabraka-gpui-core --lib --features test-support -- profiler_overhead --ignored"

echo "=== 001 ALL PASSED ==="
```
