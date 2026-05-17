# 003 - Visual Test 平台迁移任务

## 目标

在保留当前 `VisualTestContext` 的基础上，建立“真实渲染 smoke test”和“mocked 结构测试”的分层方案。优先支持屏幕外坐标窗口、确定性调度、截图或 render artifact 捕获；不在首版引入大规模 pixel-perfect snapshot。

## 上游参考

- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/app/visual_test_context.rs`
- `/Users/hejun/codespace/my/agenttray/zed/crates/gpui/src/platform/visual_test.rs`

上游能力：

- macOS 真实渲染 visual test context。
- 真实平台 + 可控 `TestDispatcher`。
- `VisualTestPlatform` 采用代理模式：大部分 `Platform` 方法委托给真实平台，dispatcher/time/test-control 相关方法走 `TestDispatcher`。
- 屏幕外坐标窗口。
- 可推进模拟时间。
- 可捕获真实截图。

## 当前仓库现状

当前已有：

- `VisualTestContext`，作为测试版 `Window` + `App`。
- `TestPlatform` 和 `TestWindow`，用于 mock rendering。
- Linux wgpu、macOS Metal、Windows DirectX 等真实平台 renderer。
- Adabraka 独有 overlay/tray/window positioning/layer-shell 能力。

当前缺口：

- mock visual context 无法发现真实 renderer/GPU/compositor 问题。
- 缺少统一 screenshot capture 测试入口。
- 缺少“屏幕外坐标但真实绘制”的平台封装。
- 没有 visual test 与 CI 环境能力探测。

## 预期效果

完成后应具备：

- 可以在支持的平台打开屏幕外坐标窗口并真实渲染。
- 可以推进 test dispatcher 和 simulated clock。
- 可以捕获截图或结构化 render artifact。
- 对不支持真实 visual test 的环境自动 skip 或退化为 mock test。

## 收益

- 覆盖真实平台 renderer smoke，例如 Metal/DirectX/wgpu present。
- 验证窗口定位、visible bounds、tooltip、popup、text fallback 等可见行为。
- 为后续上游渲染 bugfix 迁移提供验证环境。
- 对 Adabraka tray/overlay/layer-shell 这类产品能力尤其有价值。

## 风险

- pixel-perfect 截图很容易因字体、DPI、GPU、系统主题产生 flake。
- macOS/Windows/Linux 的屏幕外坐标窗口行为不同。
- CI 可能没有 GPU、显示服务或必要权限。
- 真实 visual test 运行时间比 mock test 更长。

## 设计原则

- 首版只做 smoke，不做严格 snapshot。
- 所有真实 visual test 必须可按平台能力 skip。
- 保留 mock `VisualTestContext`，不替换。
- 截图比较先做基本属性：非空、尺寸、关键像素/区域存在，而不是全图 diff。
- 平台差异用 trait/adapter 收敛。
- macOS 首选上游已验证的 off-screen coordinates 方案，例如把窗口放到 `(-10000, -10000)` 附近。它不是“真正离屏渲染”，而是让 compositor 仍渲染窗口但不干扰用户。
- Linux wgpu/headless 后续可评估真正 headless surface，但不要把 macOS 方案误写成通用离屏渲染。

## 迁移计划

### Step 1：能力探测

新增测试 helper：

- `VisualTestCapabilities`

候选字段：

- `real_renderer`
- `screenshot_capture`
- `offscreen_positioned_window`
- `deterministic_clock`
- `pixel_snapshot_stable`

不同平台返回：

- macOS：优先支持真实 renderer + screenshot。
- Linux：依赖 Wayland/X11/wgpu/headless 环境，先 smoke。
- Windows：先 smoke，截图后续补。
- CI 无显示：skip 真实 visual test。

探测信号：

- macOS：Metal device /真实平台创建成功。
- Linux：`DISPLAY` 或 `WAYLAND_DISPLAY` 存在，或明确启用 headless renderer。
- Windows：DirectX device /窗口创建 smoke 成功。
- CI：通过 ignored tests 或专用 GPU runner 显式启用真实 visual tests。

### Step 2：真实 visual test context 试点

候选类型：

- `RealVisualTestContext`
- 或 `VisualTestAppContext`

不要直接替换现有 `VisualTestContext`。

核心 API：

- `new_if_supported() -> Option<Self>`
- `open_offscreen_positioned_window`
- `advance_clock`
- `run_until_parked`
- `draw`
- `capture_screenshot`

实现策略：

- 参考上游 `VisualTestPlatform` 代理模式。
- 真实平台负责窗口、renderer、系统能力。
- `TestDispatcher` 负责 deterministic task scheduling、fake clock、`run_until_parked`。
- 对无法安全委托的 Adabraka 扩展 API，先显式转发或标记 unsupported，不要静默丢失。

### Step 3：截图验证策略

首版断言：

- screenshot width/height 符合窗口大小。
- buffer 非空。
- 至少一个区域非透明/非全黑。
- text/button 简单 UI 有可见像素变化。

暂不做：

- 全图 golden baseline。
- 跨平台统一像素 diff。
- 复杂抗锯齿阈值。

### Step 4：与现有 VisualTestContext 合并 API

目标是让测试代码尽量相似：

- mock context 用于 layout/bounds/interaction。
- real context 用于 renderer/screenshot smoke。

可以抽出共同 helper：

- window creation
- draw/wait frame
- input simulation

### Step 5：添加代表性 smoke tests

首批测试：

- simple div renders nonblank。
- text renders nonblank。
- popup/window positioning 不越界。
- tray/overlay/layer-shell 只在对应平台手动或 feature gated 测试。

## 验证方案

### 自动测试

```bash
cargo test -p adabraka-gpui visual_test --features test-support
cargo test -p adabraka-gpui --lib --features test-support
```

自动测试只跑 mock 和 capability detection；真实 renderer 测试默认可 skip。

CI 策略：

- 默认 CI 只跑 mock visual tests 和 capability detection。
- 真实 renderer tests 标记 `#[ignore]`。
- 需要单独 GPU runner 或本机手动执行 ignored tests。
- Linux runner 必须显式提供 `DISPLAY` / `WAYLAND_DISPLAY` 或 headless renderer 配置，否则 skip。

### macOS 手动 smoke

```bash
cargo test -p adabraka-gpui --test real_visual_smoke --features test-support -- --ignored
```

验证：

- 屏幕外坐标窗口不干扰用户。
- simple div 场景可通过真实 renderer present。
- 窗口 bounds 与屏外坐标配置一致。
- 多次运行稳定。

### Linux 手动 smoke

需要 Linux 环境和 wgpu/Wayland 或 X11：

```bash
cargo test -p adabraka-gpui real_visual --features test-support,wayland,x11 -- --ignored
```

验证：

- 有显示服务时能跑。
- headless/CI 无显示时优雅 skip。

### Windows 手动 smoke

验证 DirectX renderer 和窗口创建：

```powershell
cargo test -p adabraka-gpui --test real_visual_smoke --features test-support -- --ignored
```

## 当前状态

进行中（2026-05-17）：

- 已新增 `VisualTestCapabilities::detect()`，用于报告 `real_renderer`、`screenshot_capture`、`offscreen_positioned_window`、`deterministic_clock`。
- macOS 当前报告真实 renderer 和屏幕外坐标窗口可用；`screenshot_capture` 在 `render_to_image` 落地前保持 false。Linux/FreeBSD 根据 `DISPLAY` / `WAYLAND_DISPLAY` 探测真实 renderer；Windows 当前报告真实 renderer 可用。
- 已新增 capability detection 自动测试，不创建真实窗口、不触发 GPU smoke。
- 已新增 mock visual render artifact：`TestWindow::draw(scene)` 会记录场景结构，`TestAppWindow::visual_render_artifact()` 可读取最近一次 mock draw 的 primitive 统计。
- 已新增 macOS `RealVisualTestContext` / `VisualTestPlatform` 试点，并通过 `harness = false` 的 `real_visual_smoke` 在主线程打开屏外坐标窗口、绘制 simple div、调用真实 renderer present。
- 验证脚本：`scripts/verify-003.sh`。

当前 screenshot smoke 尚未落地，原因：

- 上游 `VisualTestAppContext::capture_screenshot` 依赖 `Window::render_to_image()` / `PlatformWindow::render_to_image()`。
- 当前仓库的 `PlatformWindow` trait 没有 `render_to_image` 方法，macOS/Windows/Linux renderer 也没有统一的 scene-to-image 读回入口。
- `real_visual_smoke` 已覆盖真实 renderer present，但不读取像素，因此不能替代 screenshot nonblank 断言。

下一步建议拆成独立小切片：

1. 评估 `PlatformWindow::render_to_image` 是否能以默认 unsupported 方法引入，并只在 macOS Metal 先实现。
2. 添加 `real_visual_screenshot_* --ignored` smoke，unsupported 环境返回 skip。
3. 再评估 Linux wgpu headless surface 或 readback 路径。

已验证：

```bash
cargo test -p adabraka-gpui --lib --features test-support -- visual_test_capabilities
cargo test -p adabraka-gpui --lib --features test-support -- visual_test_mock_render_artifact
cargo test -p adabraka-gpui --test real_visual_smoke --features test-support
cargo test -p adabraka-gpui --test real_visual_smoke --features test-support -- --ignored
```

结果：通过。当前 003 已完成 capability detection、mock structural artifact、macOS real renderer smoke；screenshot smoke 仍待后续切片实现。

## 完成标准

- 真实 visual test capability detection 可用。
- 至少 macOS 或一个平台可以跑真实 renderer smoke。
- unsupported 环境不失败，只 skip。
- mock `VisualTestContext` 现有测试不受影响。
- 文档说明 visual test 分层：mock、real smoke、snapshot。
- 文档明确 macOS 使用屏幕外坐标方案，不声称真正 offscreen surface。

## 后续扩展

- 局部 golden snapshot，只覆盖稳定组件。
- screenshot diff 阈值工具。
- profiler 集成：截图测试同时采集 frame timing。
- Linux wgpu canvas-pixel 检查。

---

## 执行与验证增强

以下是对原始计划的补充，针对具体实现路径和验证覆盖进行增强。

### 上游 VisualTestPlatform 代理模式参考

上游 `platform/visual_test.rs`（262 行）已实现成熟的代理模式。核心思路：

```
VisualTestPlatform
├── 渲染相关方法 → 委托给真实 MacPlatform（Metal rendering）
├── 调度相关方法 → 委托给 TestDispatcher（确定性推进）
├── 时间相关方法 → 委托给 TestDispatcher 的 fake clock
└── 窗口创建 → 委托给真实 MacPlatform，但用屏外坐标
```

当前仓库的实现应参考此模式，而不是从头设计。关键点：

1. `VisualTestPlatform` 实现 `Platform` trait
2. 大部分方法直接转发给真实平台
3. `dispatch` / `dispatch_on_main_thread` / `dispatch_after` / `now` 转发给 `TestDispatcher`
4. 窗口创建使用真实平台但传入屏外坐标

### 屏幕外坐标窗口的实际含义

上游使用的是**屏幕外坐标**（如 -10000, -10000），而非真正的离屏渲染。区别：

| 方式 | 说明 | 平台支持 |
| --- | --- | --- |
| 屏外坐标 | 窗口存在于 compositor 中但不可见，仍被真实渲染 | macOS/Windows/Linux 均支持 |
| Headless surface | 渲染到内存 buffer，无 compositor 参与 | Linux wgpu 可能支持 |
| 真正不可见窗口 | 系统级不可见窗口 | macOS 不支持此概念 |

推荐首版使用**屏外坐标方式**（与上游一致），因为：
- macOS compositor 仍会渲染窗口内容，截图可行
- 不需要特殊平台 API
- 已在上游验证稳定

注意：在 Linux Wayland 下，compositor 可能不渲染屏外窗口。需要使用 `wl_subsurface` 或特定 wgpu headless surface。这是 Linux 平台的后续问题，首版 macOS 优先。

### Step 1 能力探测的具体实现

```rust
/// 运行时检测当前环境支持哪些 visual test 能力。
pub struct VisualTestCapabilities {
    pub real_renderer: bool,
    pub screenshot_capture: bool,
    pub offscreen_positioned_window: bool,
    pub deterministic_clock: bool,
}

impl VisualTestCapabilities {
    pub fn detect() -> Self {
        Self {
            // macOS: 总是支持真实渲染（Metal）
            // Linux: 需要检查 wgpu adapter 可用性
            // Windows: 需要检查 DirectX device
            // CI 无 GPU: 不支持
            real_renderer: Self::detect_real_renderer(),
            screenshot_capture: cfg!(target_os = "macos"), // 首版只 macOS
            // macOS 已通过上游验证；Windows/Linux 需要真实平台 smoke 后再置 true。
            offscreen_positioned_window: cfg!(target_os = "macos"),
            deterministic_clock: true, // TestDispatcher 总是支持
        }
    }

    fn detect_real_renderer() -> bool {
        #[cfg(target_os = "macos")]
        {
            // 检查是否有 Metal device
            // 注意：macOS CI 通常有 Metal（即使无显示器）
            true
        }
        #[cfg(target_os = "linux")]
        {
            // 检查 DISPLAY 或 WAYLAND_DISPLAY 环境变量
            std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
        }
        #[cfg(target_os = "windows")]
        {
            true // Windows 通常有 DirectX
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            false
        }
    }
}
```

使用方式（在测试中）：

```rust
#[test]
#[ignore] // 默认不跑，需要 GPU 环境
fn real_visual_test_example() {
    let caps = VisualTestCapabilities::detect();
    if !caps.real_renderer {
        eprintln!("skipping: no real renderer available");
        return;
    }
    // ... 真实渲染测试
}
```

### Step 5 断言标准量化

"nonblank" 的具体量化标准：

```rust
/// 验证截图非空白的最小断言集合。
fn assert_screenshot_nonblank(image: &RgbaImage, label: &str) {
    // 1. 尺寸正确
    assert!(image.width() > 0 && image.height() > 0,
        "{}: screenshot has zero dimensions", label);

    // 2. 至少 1% 的像素有非零 alpha（非完全透明）
    let total_pixels = (image.width() * image.height()) as usize;
    let opaque_pixels = image.pixels().filter(|p| p[3] > 0).count();
    assert!(opaque_pixels > total_pixels / 100,
        "{}: only {}/{} pixels are non-transparent", label, opaque_pixels, total_pixels);

    // 3. 不是纯色填充（至少有 2 种不同的颜色值）
    let first_pixel = image.pixels().next().unwrap();
    let has_variation = image.pixels().any(|p| p != first_pixel);
    assert!(has_variation,
        "{}: screenshot is solid color, likely not rendered", label);
}
```

对于更具体的 UI 验证（如文本渲染），可以检查特定区域：

```rust
/// 检查指定矩形区域内有可见内容。
fn assert_region_has_content(image: &RgbaImage, x: u32, y: u32, w: u32, h: u32) {
    let region_pixels: Vec<_> = image.pixels()
        .enumerate()
        .filter(|(i, _)| {
            let px = (*i as u32) % image.width();
            let py = (*i as u32) / image.width();
            px >= x && px < x + w && py >= y && py < y + h
        })
        .map(|(_, p)| p)
        .collect();
    let has_content = region_pixels.iter().any(|p| p[3] > 0 && (p[0] > 0 || p[1] > 0 || p[2] > 0));
    assert!(has_content, "region ({},{})-({}x{}) has no visible content", x, y, w, h);
}
```

### 依赖关系澄清

原始方案说"依赖 TestApp/Headless 更稳后"，但实际分析后发现 003 的核心依赖是：

1. **TestDispatcher 的 deterministic clock 能力** — 已存在且稳定
2. **窗口创建和 draw 能力** — 已通过 `TestAppContext::add_window` 稳定
3. **平台渲染 pipeline** — 已通过现有的 macOS Metal / Linux wgpu 路径稳定

因此 003 可以独立于 002 开始。不需要等 `TestApp` wrapper 完成，因为 visual test context 本身就是独立实现。

依赖修正：需要的是 `TestDispatcher` 稳定（已满足），而非 `TestApp` 稳定。

### Gate 测试

Step 1（能力探测）完成时：

```rust
#[test]
fn visual_test_capabilities_detect_does_not_panic()

#[test]
fn visual_test_capabilities_reports_deterministic_clock_true()

#[test]
#[cfg(target_os = "macos")]
fn visual_test_capabilities_macos_has_real_renderer()
```

Step 2（真实 visual test context 试点）完成时：

```rust
#[test]
#[ignore]
#[cfg(target_os = "macos")]
fn real_visual_context_opens_offscreen_positioned_window()

#[test]
#[ignore]
#[cfg(target_os = "macos")]
fn real_visual_context_advance_clock_works()
```

Step 3（截图验证）完成时：

```rust
#[test]
#[ignore]
#[cfg(target_os = "macos")]
fn real_visual_screenshot_is_nonblank()

#[test]
#[ignore]
#[cfg(target_os = "macos")]
fn real_visual_screenshot_matches_window_size()
```

### 验证脚本

```bash
#!/bin/bash
# scripts/verify-003.sh
set -e
echo "=== 003 - Visual Test Platform ==="

echo "[1/5] Compile checks..."
cargo check -p adabraka-gpui
cargo check -p adabraka-gpui --no-default-features
cargo check -p adabraka-gpui --no-default-features --features wgpu

echo "[2/5] Capability detection tests (always run)..."
cargo test -p adabraka-gpui --lib --features test-support -- visual_test_capabilities

echo "[3/5] Mock visual tests (no GPU required)..."
cargo test -p adabraka-gpui --lib --features test-support -- visual_test

echo "[4/5] Frozen tests (regression guard)..."
cargo test -p adabraka-gpui --lib --features test-support -- app::test
cargo test -p adabraka-gpui --lib --features test-support -- executor

echo "[5/5] Full lib test..."
cargo test -p adabraka-gpui --lib --features test-support

echo ""
echo "(Optional) Real renderer smoke (macOS with GPU):"
echo "  cargo test -p adabraka-gpui --test real_visual_smoke --features test-support -- --ignored"

echo "=== 003 ALL PASSED ==="
```
