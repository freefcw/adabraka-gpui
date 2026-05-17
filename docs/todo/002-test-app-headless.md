# 002 - TestApp / Headless 测试能力迁移任务

## 目标

在现有 `TestAppContext` 之上提供更易用、更稳定的测试 API，并补强 headless 场景下真实 text shaping / font metrics 的测试能力。此任务不替换当前 `#[gpui::test]` 流程。

## 上游参考

- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/app/test_app.rs`
- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/app/headless_app_context.rs`

上游 `TestApp` 主要能力：

- 更干净的测试入口。
- 更新后自动 flush effects。
- 简化 window 创建和检查。
- 提供输入模拟 helper。

上游 `HeadlessAppContext` 主要能力：

- 跨平台 headless app context。
- 基于 `TestPlatform` 保持确定性调度。
- 允许注入真实 `PlatformTextSystem`，用于真实 glyph metrics/shaping。
- 可选 renderer factory，用于截图捕获。

## 当前仓库现状

当前已有：

- `TestAppContext::single`
- `TestAppContext::build`
- `TestAppContext::add_window`
- `TestAppContext::add_empty_window`
- `VisualTestContext`
- `Application::headless`
- `current_platform(true)` 和 Linux headless client

当前缺口：

- `TestAppContext` API 偏底层，测试常常需要手动 `update`、`run_until_parked`、draw。
- 没有上层 wrapper 统一“update 后 flush effects”的习惯。
- headless 场景不方便注入真实平台 text system。
- 测试窗口检查和输入模拟分散在不同 context 上。
- Adabraka 特有 tray 测试能力需要明确暴露或保留直达 `TestAppContext` 的路径。

## 预期效果

完成后应具备：

- 新测试可以用 `TestApp` 更少样板代码创建窗口、更新 view、检查状态。
- 旧测试仍可使用 `TestAppContext`，无需迁移。
- 可以写 headless text shaping 测试，使用真实 macOS/DirectWrite/Cosmic text system。
- 可以覆盖 daemon without windows、window lifecycle、timer/effect flush。

## 收益

- 提升后续迁移上游 bugfix 时写回归测试的效率。
- 降低测试中忘记 flush effects / run_until_parked 的概率。
- 为 visual test 前置准备统一的 window/test API。
- 对 Adabraka 的 headless/tray/daemon 模式更有价值。

## 风险

- 如果 wrapper 自动 flush 语义不清，会掩盖真实生命周期问题。
- 如果新旧测试 API 并存但职责不清，长期维护成本会上升。
- 真实 text system 测试在不同平台可用性不同，需要 cfg 管理。

## 设计原则

- `TestApp` 是 wrapper，不替换 `TestAppContext`。
- 自动 flush 行为必须文档化，必要时提供 `update_without_flush`。
- headless text system 注入只用于测试和专项验证，不改变生产 `Application::headless()` 默认行为。
- 所有新增 API 先 gated 在 `test-support` 或 `#[cfg(test)]`。
- `PlatformTextSystem` 仍是 crate-private trait。外部 integration tests 不应直接依赖它；headless text system 测试优先放在 crate 内部测试模块，或只在 `#[cfg(test)]` 下提供受限 re-export。
- `TestApp` 首版不强制拆分 `test_context.rs`。如果实现稳定且文件继续膨胀，再拆到 `app/test_app.rs`，避免迁移阶段产生额外 churn。
- Adabraka tray/daemon/resource-profile 专属测试能力必须保留：要么在 `TestApp` 暴露 wrapper，要么文档明确这些场景继续直接使用 `TestAppContext`。

## 迁移计划

### Step 1：设计 `TestApp` wrapper

候选新增类型：

- `TestApp`
- `TestAppWindow<V>`

候选位置：

- 继续放在 `crates/gpui/src/app/test_context.rs`
- 或拆出 `crates/gpui/src/app/test_app.rs`

建议首版先放在 `test_context.rs` 或最小新文件中，不做大规模移动。等 API 稳定后再拆文件，避免额外 churn。

最小 API：

- `TestApp::new()`
- `TestApp::with_seed(u64)`
- `TestApp::open_window`
- `TestApp::update`
- `TestApp::flush_effects`
- `TestApp::run_until_parked`
- `TestAppWindow::update`
- `TestAppWindow::read`
- `TestAppWindow::draw`

### Step 2：自动 flush 策略

每次 `TestApp::update` 后：

- 调用 app update。
- 调用 `flush()` 推进 foreground/background task，并调度必要的窗口 redraw。

定义：

- `update` = `cx.update(...)` + `flush()`。
- `flush` = 不执行新的 app update，只推进 test dispatcher 到 parked，并完成必要的窗口 refresh/draw cycle。
- `TestAppContext::refresh()` 只负责调度窗口 redraw；`TestApp::flush()` 是更高层组合动作，包含 task 推进和 redraw。

需要保留手动模式：

- `update_without_flush`
- `flush`

原因：有些测试需要断言 flush 前状态。

### Step 3：输入模拟 helper

先封装常用输入：

- key down / key up / keystroke
- mouse move / down / up
- modifiers change
- clipboard read/write mock

不要一开始覆盖所有平台事件。

### Step 4：Headless text system 注入

新增 headless test builder：

- `HeadlessTestApp::with_text_system(Arc<dyn PlatformTextSystem>)`
- 或 `TestApp::headless_with_text_system(...)`

注意 `PlatformTextSystem` 当前是 crate-private trait。首版可以放在 crate 内测试模块使用，暂不公开。

外部测试策略：

- crate 内部单元测试可以直接构造真实 `PlatformTextSystem`。
- `tests/` integration tests 不直接暴露 `PlatformTextSystem`，除非新增 `#[cfg(test)]` re-export。
- 如果需要 public test-support API，先暴露更窄的 builder，例如 `HeadlessTextSystemKind::PlatformDefault`，避免泄漏内部 trait。

可覆盖场景：

- font fallback
- glyph metrics
- line layout
- no-window app lifecycle

### Step 5：迁移少量现有测试作为样例

选择 2-3 个低风险测试迁移：

- window positioning
- text layout/headless
- cached input 或 list behavior 后续回归测试

不做大规模测试重写。

## 验证方案

### 单元测试

新增测试：

- `TestApp::update` 会自动 run effects。
- `update_without_flush` 不自动推进 task。
- `TestAppWindow::read/update` 能访问 view。
- `TestAppWindow::draw` 后可读 bounds/debug 信息。

### Headless 测试

新增平台条件测试：

- macOS：可用 `MacTextSystem` 做真实 text metrics。
- Linux：可用 `CosmicTextSystem` 和内置字体做 layout。
- Windows：可用 DirectWrite text system，若 CI 不支持则手动 smoke。

### 命令

```bash
cargo test -p adabraka-gpui test_app
cargo test -p adabraka-gpui headless
cargo test -p adabraka-gpui --lib --features test-support
cargo check -p adabraka-gpui --no-default-features --features wgpu
```

### 手动验证

- 写一个最小 test-only view，包含 button/input/timer。
- 用新 `TestApp` 模拟输入并断言状态变化。
- 用 headless text system 断言 layout/glyph fallback。

## 当前状态

已完成首版（2026-05-17）：

- 已新增 `TestApp` wrapper，覆盖 `new`、`with_seed`、`update`、`update_without_flush`、`flush` / `flush_effects`、`open_window`、`raw_context` escape hatch。
- 已新增 `TestAppWindow`，覆盖 `handle`、`root`、`read`、`update`、`draw`、`flush`。
- 已保留 `TestAppContext` 原有入口；`TestApp` 仅作为上层便利 API，不替换现有 `#[gpui::test]` 流程。
- 已通过 `raw_context()` / `raw_context_mut()` 和 tray helper 保留 Adabraka tray/daemon 专属测试路径。
- `TestPlatform` 支持测试内注入真实 `PlatformTextSystem`，macOS / Linux crate 内部测试可覆盖真实 headless text layout。
- 验证脚本：`scripts/verify-002.sh`。

已验证：

```bash
cargo test -p adabraka-gpui --lib --features test-support -- test_app
cargo test -p adabraka-gpui --lib --features test-support -- headless
```

结果：全部通过；macOS 当前覆盖 `headless_mac_text_layout_produces_nonzero_metrics`，Linux 对应覆盖 `headless_cosmic_text_layout_produces_nonzero_metrics`。

## 完成标准

- 旧 `TestAppContext` 测试不需要改。
- 新 `TestApp` 至少覆盖窗口创建、update/read/draw、flush。
- 有文档说明何时用 `TestApp`、何时直接用 `TestAppContext`。
- tray、daemon、resource profile 相关测试路径不退化。
- 至少一个 headless text shaping 测试通过。

## 后续扩展

- 更完整的 input simulation DSL。
- 与 profiler 集成，测试里可断言某类 task 已完成。
- 为 visual test 共享 `TestAppWindow` inspection API。

---

## 执行与验证增强

以下是对原始计划的补充，针对具体实现路径和验证覆盖进行增强。

### flush 语义的精确定义

"自动 flush effects"需要明确对应到当前代码的哪些操作。基于现有 `TestAppContext` 实现，flush 的参考定义：

```rust
impl TestApp {
    /// update 后自动 flush：推进所有 pending task 并刷新窗口。
    pub fn update<R>(&mut self, f: impl FnOnce(&mut App) -> R) -> R {
        let result = self.cx.update(f);
        self.flush();
        result
    }

    /// 精确的 flush 语义：
    /// 1. run_until_parked - 推进所有 foreground + background task 直到无可运行任务
    /// 2. refresh - 调度所有窗口 redraw
    /// 3. run_until_parked - 处理 redraw 可能产生的新 task
    pub fn flush(&mut self) {
        self.cx.run_until_parked();
        let _ = self.cx.refresh();
        self.cx.run_until_parked();
    }

    /// 不自动 flush 的 update，用于需要断言中间状态的场景。
    pub fn update_without_flush<R>(&mut self, f: impl FnOnce(&mut App) -> R) -> R {
        self.cx.update(f)
    }
}
```

关键注意事项：

- flush 调用两次 `run_until_parked` 是必要的，因为 window redraw 可能 spawn 新 task。
- `flush` 依赖现有 `TestAppContext::refresh()`，不要在计划里引入未确认存在的 `window_context(...).refresh()` 伪 API。
- 如果存在无限循环 task（task A spawn task B，B spawn A），`flush` 会一直运行。首版不引入 tick 上限以免改变 test dispatcher 语义；测试超时应被视为被测逻辑存在无限 task 循环。

### Adabraka 特有 API 暴露策略

当前 `TestAppContext` 已有以下 Adabraka 特有方法：

- `tray_icon() -> Option<Vec<u8>>`
- `tray_icon_rendering_mode() -> TrayIconRenderingMode`
- `simulate_tray_icon_click_event(event)`

`TestApp` 作为 wrapper，应通过以下方式暴露这些能力：

```rust
impl TestApp {
    /// 委托给内部 TestAppContext
    pub fn tray_icon(&self) -> Option<Vec<u8>> { self.cx.tray_icon() }
    pub fn tray_icon_rendering_mode(&self) -> TrayIconRenderingMode { self.cx.tray_icon_rendering_mode() }
    pub fn simulate_tray_icon_click_event(&self, event: TrayIconClickEvent) {
        self.cx.simulate_tray_icon_click_event(event);
    }

    /// 或者提供 escape hatch 访问底层 TestAppContext
    pub fn raw_context(&self) -> &TestAppContext { &self.cx }
    pub fn raw_context_mut(&mut self) -> &mut TestAppContext { &mut self.cx }
}
```

建议优先提供 `raw_context()` escape hatch，而不是逐一包装所有方法。这样新 TestApp 聚焦于 flush/window/update 语义改进，Adabraka 特有功能通过 escape hatch 访问。

### Headless text system 测试的放置位置

`PlatformTextSystem` 是 `pub(crate)` trait（位于 `platform.rs:770`），外部 integration test 无法直接使用。

解决方案：headless text system 测试放在 crate 内部：

```
crates/gpui/src/text_system/headless_tests.rs  (新文件)
  #[cfg(any(test, feature = "test-support"))]
```

或者在现有 `text_system/line_layout.rs` 的 `#[cfg(test)] mod tests` 中增加条件编译的 headless 测试：

```rust
#[cfg(test)]
mod tests {
    // 现有测试...

    #[cfg(target_os = "macos")]
    mod headless_mac {
        use crate::platform::mac::MacTextSystem;
        #[test]
        fn headless_mac_text_metrics_match_real_system() { ... }
    }
}
```

### Step 1-2 Gate 测试

Step 1（TestApp wrapper）完成时：

```rust
#[gpui::test]
async fn test_app_update_auto_flushes_effects(cx: &mut TestAppContext)

#[gpui::test]
async fn test_app_update_without_flush_preserves_pending_state(cx: &mut TestAppContext)

#[gpui::test]
async fn test_app_open_window_returns_usable_handle(cx: &mut TestAppContext)

#[gpui::test]
async fn test_app_window_read_returns_view_state(cx: &mut TestAppContext)
```

Step 2（自动 flush）完成时：

```rust
#[gpui::test]
async fn test_app_flush_processes_spawned_tasks(cx: &mut TestAppContext)

#[gpui::test]
async fn test_app_flush_triggers_window_redraw(cx: &mut TestAppContext)

#[gpui::test]
async fn test_app_multiple_updates_each_flush(cx: &mut TestAppContext)
```

Step 4（Headless text system）完成时：

```rust
#[test]
#[cfg(target_os = "macos")]
fn headless_mac_text_layout_produces_nonzero_metrics()

#[test]
#[cfg(target_os = "linux")]
fn headless_cosmic_text_layout_produces_nonzero_metrics()
```

### 退化和异常场景测试

原始方案只覆盖正常路径，补充以下异常场景：

```rust
#[gpui::test]
async fn test_app_update_after_window_close_does_not_panic(cx: &mut TestAppContext)

#[gpui::test]
async fn test_app_flush_with_no_windows_succeeds(cx: &mut TestAppContext)

#[gpui::test]
async fn test_app_headless_open_window_returns_error_or_handle(cx: &mut TestAppContext)
```

### 何时用 TestApp vs TestAppContext 的判断指南

在 TestApp 文档注释或独立文档中说明：

| 场景 | 推荐 | 原因 |
| --- | --- | --- |
| 新测试，需要窗口 + view 交互 | `TestApp` | 自动 flush 减少样板代码 |
| 需要精确控制 task 推进时机 | `TestAppContext` | 直接访问 `run_until_parked`、`tick` |
| 需要多个 app context 协作 | `TestAppContext` | `#[gpui::test]` 宏原生支持多 cx 参数 |
| Adabraka 特有功能（tray/daemon） | `TestApp` + `raw_context()` 或直接 `TestAppContext` | escape hatch 访问 |
| 性能敏感、需要最小开销 | `TestAppContext` | 无额外 flush 开销 |

### 验证脚本

```bash
#!/bin/bash
# scripts/verify-002.sh
set -e
echo "=== 002 - TestApp / Headless ==="

echo "[1/5] Compile checks..."
cargo check -p adabraka-gpui
cargo check -p adabraka-gpui --no-default-features
cargo check -p adabraka-gpui --no-default-features --features wgpu

echo "[2/5] TestApp unit tests..."
cargo test -p adabraka-gpui --lib --features test-support -- test_app

echo "[3/5] Headless tests..."
cargo test -p adabraka-gpui --lib --features test-support -- headless

echo "[4/5] Frozen tests (regression guard)..."
cargo test -p adabraka-gpui --lib --features test-support -- app::test
cargo test -p adabraka-gpui --lib --features test-support -- executor
cargo test -p adabraka-gpui --lib --features test-support -- text_system

echo "[5/5] Full lib test..."
cargo test -p adabraka-gpui --lib --features test-support

echo "=== 002 ALL PASSED ==="
```
