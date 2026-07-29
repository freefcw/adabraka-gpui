# GPUI Linux/WGPU Rendering Root Cause Analysis

## 结论先行

当前 Linux 下“布局基本正确、文本/图标大多正确，但圆角背景、边框、分隔线、渐变填充、toggle track 等非文本组件大量异常”的首要嫌疑，不应继续放在应用层布局，也不应先放在 Wayland compositor 或透明窗口上。

更确定、更底层的问题是：

> Rust 侧发送给 WGPU storage buffer 的 scene primitive 结构体，已经比 `shaders.wgsl` 中手写的结构体大很多；WGPU shader 仍按旧布局读取 `Background` / `Quad` / path vertex。结果是 GPU 按错误 offset 和错误 array stride 读取 quads，导致非文本 primitive 被错读、裁剪到窗口外、颜色/圆角/边框/渐变参数错乱。

这能同时解释几个非常关键的现象：

- CPU 布局正确，因为 Taffy/layout 在 CPU 侧完成，错误发生在 GPU shader 读取 primitive 数据时。
- 文本和图标相对稳定，因为 text 走 `MonochromeSprite`/atlas 路径，不走已经明显失配的 `Quad`/`Background` 路径。
- 简单无圆角、无边框的 solid quad 有时能显示，因为 shader 读取的前几个字段仍然大致对齐，且默认 color stop 字段多为 0，误读后仍可能落入“无边框无圆角 fast path”。
- 圆角、边框、渐变、progress fill、toggle track 失败概率高，因为这些字段恰好位于 `Background` 后面或依赖多 stop/transform/blend mode，当前 WGSL 读到的不是 Rust 真正写入的字段。
- “滚动/交互后变好”更像是错误 scene/batch/viewport 组合发生变化后的偶然表现，而不是应用状态真的改变；在 ABI 失配修复前，透明/帧调度实验的结论都不可靠。

## 重要澄清：macOS 并不是用系统控件绘制应用内容

macOS 路径确实更“系统集成”：

- 窗口是 AppKit `NSWindow`/`NSPanel`。
- surface 是 `CAMetalLayer`。
- present/vsync 由 Metal + `CVDisplayLink` 这套 Apple 原生栈支撑。

但 GPUI 的按钮、卡片、背景、边框、文字等内容仍然是 GPUI 自绘。macOS 稳定的原因不是“系统 UI 控件替 GPUI 画好了这些组件”，而是 macOS renderer 使用 Metal，并通过 cbindgen 生成的 shader binding 让 shader 结构与 Rust scene 结构保持一致；Linux WGPU 这边目前没有同等的 ABI 生成/校验机制。

## 证据 1：`Background` 结构已经明显失配

Rust 当前定义：

```rust
pub struct Background {
    pub(crate) tag: BackgroundTag,
    pub(crate) color_space: ColorSpace,
    pub(crate) solid: Hsla,
    pub(crate) gradient_angle_or_pattern_height: f32,
    pub(crate) colors: [LinearColorStop; 4],
    pub(crate) stop_count: u32,
    pub(crate) center: [f32; 2],
    pub(crate) radius: [f32; 2],
}
```

对应位置：

- `crates/gpui/src/color.rs`

WGPU shader 当前定义：

```wgsl
struct Background {
    tag: u32,
    color_space: u32,
    solid: Hsla,
    gradient_angle_or_pattern_height: f32,
    colors: array<LinearColorStop, 2>,
    pad: u32,
}
```

对应位置：

- `crates/gpui-wgpu/src/shaders.wgsl`

差异：

| 字段 | Rust | WGPU WGSL |
| --- | --- | --- |
| color stops | `colors[4]` | `colors[2]` |
| stop count | 有 | 无 |
| radial/conic center | 有 | 无 |
| radial radius | 有 | 无 |
| tag 语义 | `3=RadialGradient`, `4=ConicGradient` | 注释仍写 `3=Checkerboard` |
| 估算 size | 128 bytes | 72 bytes |

影响：

- `Quad.background` 后面的 `border_color`、`corner_radii`、`border_widths` 在 shader 中会从错误 offset 读取。
- 多 stop linear gradient、radial/conic gradient 在 WGPU 下没有正确 ABI 承载。
- Path rasterization vertex 里也包含 `Background`，所以 path gradient 也受影响。

## 证据 2：`Quad` 结构同样失配，而且会导致 array stride 错位

Rust 当前定义：

```rust
pub(crate) struct Quad {
    pub order: DrawOrder,
    pub border_style: BorderStyle,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub background: Background,
    pub border_color: Hsla,
    pub corner_radii: Corners<ScaledPixels>,
    pub border_widths: Edges<ScaledPixels>,
    pub continuous_corners: u32,
    pub _pad_before_transform: u32,
    pub transform: TransformationMatrix,
    pub blend_mode: u32,
    pub _pad_end: u32,
}
```

WGPU shader 当前定义：

```wgsl
struct Quad {
    order: u32,
    border_style: u32,
    bounds: Bounds,
    content_mask: Bounds,
    background: Background,
    border_color: Hsla,
    corner_radii: Corners,
    border_widths: Edges,
}
```

差异：

| 字段 | Rust | WGPU WGSL |
| --- | --- | --- |
| `Background` | 128 bytes | 72 bytes |
| `continuous_corners` | 有 | 无 |
| `transform` | 有 | 无 |
| `blend_mode` | 有 | 无 |
| vertex position | Rust 提供 transform | WGPU `vs_quad` 不使用 transform |
| content mask | Rust 可 transform | WGPU `distance_from_clip_rect` 不使用 transform |
| 估算 size | 256 bytes | 160 bytes |

最关键的是 array stride：

- Rust 写入 `Vec<Quad>` 的连续内存时，每个 quad 约 256 bytes。
- WGSL `array<Quad>` 读取时，每个 quad 约 160 bytes。
- `b_quads[1]` 会从第 1 个 Rust quad 的中间开始读。
- 后续 quad 的 `bounds`、`content_mask`、`background` 全部可能是上一 quad 的颜色/圆角/transform 字节。

这不是单个字段颜色不对，而是整个 batch 的实例流从第 2 个 quad 开始系统性错位。

## 证据 3：Windows/macOS shader 已经是新语义，WGPU 是旧语义

Windows HLSL 中的 `Background` 和 `Quad` 已包含：

- `colors[4]`
- `stop_count`
- `center`
- `radius`
- `continuous_corners`
- `transform`
- `blend_mode`

macOS Metal shader 也使用：

- `quad.transform`
- `quad.background.colors[0..3]`
- `quad.background.stop_count`
- `quad.blend_mode`
- `quad.continuous_corners`

这说明并不是 GPUI 抽象层缺少这些能力，而是 Linux WGPU shader 没有同步 Rust scene ABI 和其他平台 shader 语义。

## 证据 4：构建脚本已经暴露了流程缺口

`crates/gpui/build.rs` 顶部有一个很直接的 TODO：

```rust
//TODO: consider generating shader code for WGSL
```

macOS 会用 cbindgen 从 Rust 类型生成 `scene.h`，再让 Metal shader include 这份 header。WGPU/WGSL 没有这个保护，只靠手写结构体同步；当 `Background`、`Quad` 演进后，Linux shader 漏同步就会形成 ABI 漂移。

## 为什么这个问题和用户症状高度一致

### 1. Toggle 只剩白色/黑色方块

toggle track 是一个外层圆角 quad：

- 固定宽高
- `rounded_full`
- `bg(track_bg)`
- `border_1`
- child 是 knob quad

如果 WGPU shader 错读外层 track 的 `bounds`、`corner_radii`、`border_widths`、`border_color`，外层 track 很容易完全消失。child knob 是另一个 simple solid quad，有机会因为字段误读后仍落入 solid fast path 而显示，于是用户看到“只有一个小方块/白方块”。

这比“flex 布局 shrink 了 track”更能解释：

- track 改成极端紫色仍不显示；
- border 改成黄色仍不显示；
- knob 颜色能变；
- 同一个 widget 在多个设置页面一致失败；
- macOS 正常、Linux 异常。

### 2. 1px separator 消失

1px separator 是普通 quad。如果它不是 batch 的第一个 quad，或者它所在 batch 前面已有若干 quads，错误 stride 会让它的 bounds/content mask 被错读。即使 separator 的 Rust layout 高度正确，GPU 也可能把它画到错误位置、画成透明、或直接被 clip 掉。

### 3. Progress fill / gradient 直到滚动才出现

progress fill 依赖：

- rounded clipped container；
- child gradient quad；
- relative width；
- multi-stop gradient。

它同时踩中 `Quad` 和 `Background` 的高风险字段。滚动/交互改变 scene 中的可见 primitive 集合、batch 顺序、content mask、脏视图重绘路径后，某些 quad 可能短暂“幸运”地读到可见参数。这种变化不能证明 frame scheduling 是根因；在 ABI 修复前，所有 repaint 相关现象都可能是二阶效应。

### 4. 文本/图标通常可见

文本走 `MonochromeSprite`，不是 `Quad.background`。这解释了为什么 UI “内容似乎还在”，但背景、border、gradient 这些非文本 primitive 大面积异常。

## 透明窗口 / frame scheduling 仍是问题，但优先级应后移

原调查文档提出的透明/CSD/frame hypotheses 仍然有价值，但它们更像放大器，而不是当前最先要修的根因。

### Wayland 创建时强制 transparent

Wayland window 初始化 WGPU surface 时：

```rust
WgpuSurfaceConfig {
    size: options.bounds.to_device_pixels(1.0).size,
    transparent: true,
    preferred_present_mode: None,
}
```

即使 `WindowOptions::default()` 的 `window_background` 是 `Opaque`，Wayland renderer 也先以透明 surface 创建。

### CSD 会强制透明语义

Wayland 的透明判断：

```rust
self.decorations == WindowDecorations::Client
    || self.background_appearance != WindowBackgroundAppearance::Opaque
```

因此 `titlebar: None` / client-side decorations 下，即使应用请求 opaque，也会进入透明 surface / no opaque region 路径。

### 主 render pass 每帧清 transparent

WGPU renderer 主 pass：

```rust
load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
```

这在“scene primitive 偶尔没画出来”的情况下会让缺失更加明显。

### expose/frame 可以只 present 旧 scene

`Window::on_request_frame` 中：

```rust
if invalidator.is_dirty() || request_frame_options.force_render {
    window.draw(cx);
    window.present();
} else if needs_present {
    window.present();
}
```

这本身不一定错误，但当 surface 生命周期、透明合成、device recovery 或错误 batch 数据同时存在时，只 present 旧 scene 会让问题看起来和窗口移动/expose 强相关。

结论：这些点应该在 WGPU ABI 修复后重新验证。现在直接调 alpha/clear/force_render，容易得到不稳定或误导性的结果。

## 已添加的验证工具

### 1. 静态 ABI probe

新增：

```text
docs/tools/wgpu_shader_layout_probe.py
```

运行：

```bash
python3 docs/tools/wgpu_shader_layout_probe.py
```

当前输出摘要：

```text
Background:
  rust expected size: 128 bytes
  wgsl current size:  72 bytes
  verdict: MISMATCH

Quad:
  rust expected size: 256 bytes
  wgsl current size:  160 bytes
  verdict: MISMATCH
```

这个 probe 的目的不是最终 CI 测试，而是把“shader ABI 是否失配”从主观视觉判断变成一个可以立即复现的诊断点。

### 2. Linux WGPU rendering probe example

新增：

```text
crates/gpui/examples/linux_rendering_probe.rs
```

并注册为 Cargo example：

```text
crates/gpui/Cargo.toml
```

运行：

```bash
cargo run -p adabraka-gpui --example linux_rendering_probe
```

这个窗口集中放了四组 primitive：

- repeated rounded/bordered quads：暴露 storage-buffer stride 错位；
- toggle track：复现“只有 knob，track/border 消失”；
- rounded clipped multi-stop gradient progress：复现 progress fill；
- 1px separator + rounded cards：复现 separator/card background。

编译验证：

```bash
cargo check -p adabraka-gpui --example linux_rendering_probe
```

已通过。

## 推荐修复路线

### Phase 1：先修 WGPU scene/shader ABI

优先级最高。目标是让 WGPU shader 的 storage-buffer 结构与 Rust 实际写入完全一致。

需要同步：

1. `Background`
   - `colors: array<LinearColorStop, 4>`
   - `stop_count: u32`
   - `center: vec2<f32>`
   - `radius: vec2<f32>`
   - tag 语义改为 `0=Solid, 1=LinearGradient, 2=PatternSlash, 3=RadialGradient, 4=ConicGradient`

2. `Quad`
   - 补 `continuous_corners`
   - 补 transform padding / `TransformationMatrix`
   - 补 `blend_mode`
   - `vs_quad` 改用 `to_device_position_transformed`
   - clip 改用 `distance_from_clip_rect_transformed`
   - fragment 传入/使用 `blend_mode`
   - continuous corner 逻辑对齐 Metal/HLSL

3. `PathRasterizationVertex`
   - 因为包含 `Background`，同步 Background 后必须重新检查 stride。
   - path gradient 函数要支持 4 stop / radial / conic，至少不要按旧结构读错。

4. 验证所有 storage-buffer structs
   - `Shadow`
   - `Underline`
   - `MonochromeSprite`
   - `PolychromeSprite`
   - `PathSprite`
   - `SurfaceParams`

### Phase 2：加结构布局保护，避免再次漂移

可选方案按可靠性排序：

1. 从 Rust scene 类型生成 WGSL struct，呼应 `build.rs` 的 TODO。
2. 定义显式 GPU ABI 类型，如 `GpuQuad` / `GpuBackground`，给 Rust 和 WGSL 共用 schema。
3. 在 CI 增加静态 probe，至少检查关键字段和估算 size。
4. 给所有传 GPU 的 instance struct 增加安全初始化和 layout test，避免 padding/stride 漂移。

当前 `WgpuRenderer::instance_bytes` 直接把 Rust struct 原始内存发给 GPU：

```rust
std::slice::from_raw_parts(instances.as_ptr() as *const u8, std::mem::size_of_val(instances))
```

这要求 Rust struct 与 shader struct 绝对一致。只要继续使用这种传输方式，就必须有自动化 layout 保护。

### Phase 3：再验证透明/CSD/frame scheduling

ABI 修复后，重新跑：

```bash
python3 docs/tools/wgpu_shader_layout_probe.py
cargo run -p adabraka-gpui --example linux_rendering_probe
```

如果仍有“窗口移动后消失 / 滚动后恢复”，再做这些实验：

1. Wayland opaque/server-side decorations
   - `window_decorations: Some(WindowDecorations::Server)`
   - `window_background: WindowBackgroundAppearance::Opaque`

2. WGPU opaque clear diagnostic
   - 把主 pass clear 临时改为 opaque dark。
   - 只作为诊断，不作为最终修复。

3. force render on expose/frame
   - Wayland frame callback 临时 `force_render: true`
   - X11 expose 临时 `force_render: true`

4. surface state logging
   - decorations
   - background appearance
   - alpha mode
   - opaque region
   - scale factor
   - surface size
   - dirty / force_render / require_presentation

## 建议测试矩阵

| 维度 | 组合 |
| --- | --- |
| Session | Wayland, X11 |
| GPU backend | Vulkan, OpenGL, llvmpipe |
| Decoration | Client, Server |
| Background | Opaque, Transparent |
| Present behavior | normal, force render on expose/frame |
| Clear | transparent, diagnostic opaque clear |
| Scale | 1.0, fractional scale, HiDPI |
| Scene | linux_rendering_probe, BananaTray settings, BananaTray popup |

判断标准：

- ABI probe 必须通过。
- stress grid 的所有 rounded/bordered cells 都可见。
- toggle track 和 border 必须可见。
- progress gradient 初始帧可见。
- 移动窗口后 separator/card backgrounds 不消失。
- 滚动不再“修复”此前缺失的视觉。

## 为什么不建议先做应用层 workaround

应用层尝试过：

- `flex_none`
- 预留 1px 高度
- 极端颜色
- 用 `justify_end` 替代 margin
- 手动触发 repaint

这些都没有触及 GPU storage-buffer ABI。只要 shader 还在按 160 bytes stride 读 256 bytes 的 Rust quad，应用层无法可靠规避。某些 workaround 可能偶然改变 draw order 或 batch 内容，让视觉短暂改善，但不会形成可维护的 Linux backend。

## 未解决问题

- WGPU shader ABI 修复后，透明/CSD/frame scheduling 是否仍有独立 bug，需要重新实测。
- Wayland `titlebar: None` 是否应该默认 CSD 透明，还是允许 opaque content region，需要产品语义决策。
- `CompositeAlphaMode::Inherit` 应如何与 premultiplied shader output 对齐，需要在 ABI 修复后用 compositor/GPU 组合验证。
- 是否要把 WGSL struct 生成纳入 build.rs，还是先用静态 CI probe 兜底，需要实现成本评估。
