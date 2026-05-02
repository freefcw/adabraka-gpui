# GPUI Linux/WGPU Quad ABI 复核结论与修改计划

## 文档目的

本文记录对 Linux/WGPU 下非文本 UI primitive 渲染异常的复核结论，并把后续建议修改拆成可验证的步骤。

本文是 `docs/linux-wgpu-rendering-root-cause-analysis.md` 的复核补充，重点修正此前分析中不够严谨的表述：

- `Background` / `Quad` 的 ABI/stride 失配结论仍然成立。
- 简单 solid quad 可能仍然显示，因此“实际效果变化不大”不否定 ABI 问题。
- `update_transparency(!opaque)` 不是逻辑反转，它等价于 `update_transparency(state.is_transparent())`。
- premultiplied alpha 仍是可能的次级问题，但证据弱于 WGSL scene struct 失配。

## 最终复核结论

当前最确定的底层问题是：

> WGPU shader 中手写的 `Background` / `Quad` struct 已落后于 Rust scene struct。Rust renderer 直接把 `&[Quad]` 的原始内存上传到 WGPU storage buffer，而 WGSL 用更小的 struct 和更短的 array stride 读取，导致非文本 primitive 的圆角、边框、渐变、transform、blend mode 以及后续 quad instance 被错读。

这能解释 Linux/WGPU 下这些现象：

- CPU layout 大体正确，但 GPU 输出的背景、边框、圆角、separator、progress fill、nav pill、toggle track 异常。
- 文本/图标相对稳定，因为它们主要走 atlas/sprite 路径，不依赖 `Quad.background` 的完整布局。
- 简单无边框、无圆角的 solid background 有时看起来正常，因为 `Quad` 和 `Background` 的前部字段仍大体对齐。
- 越依赖 `Background` 后半部分、`Quad` 尾部字段、batch 内第二个及后续 quad 的 primitive，失败概率越高。

## 已确认事实

### 1. Rust `Background` 是 128 bytes，WGSL `Background` 当前是 72 bytes

Rust 定义位置：

- `crates/gpui/src/color.rs`

Rust 当前语义：

```rust
#[repr(C)]
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

当前 WGSL 定义位置：

- `crates/gpui/src/platform/wgpu/shaders.wgsl`

WGSL 当前语义：

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

差异：

| 项目 | Rust | WGPU WGSL 当前状态 |
| --- | --- | --- |
| color stops | 4 个 | 2 个 |
| `stop_count` | 有 | 无 |
| radial/conic `center` | 有 | 无 |
| radial `radius` | 有 | 无 |
| 估算大小 | 128 bytes | 72 bytes |

精确影响：

- 第一个 quad 的 `background.tag`、`color_space`、`solid`、`colors[0]`、`colors[1]` 通常仍在正确 offset。
- 但 `colors[2]`、`colors[3]`、`stop_count`、`center`、`radius` 在 WGPU shader 中没有 ABI 承载。
- `Quad.background` 后面的 `border_color`、`corner_radii`、`border_widths` 会从 Rust `Background` 的尾部错误位置读取。

### 2. Rust `Quad` 是 256 bytes，WGSL `Quad` 当前是 160 bytes

Rust 定义位置：

- `crates/gpui/src/scene.rs`

Rust 当前语义：

```rust
#[repr(C)]
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

当前 WGSL 定义：

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

| 项目 | Rust | WGPU WGSL 当前状态 |
| --- | --- | --- |
| `background` | 128 bytes | 72 bytes |
| `continuous_corners` | 有 | 无 |
| `_pad_before_transform` | 有 | 无 |
| `transform` | 有 | 无 |
| `blend_mode` | 有 | 无 |
| `_pad_end` | 有 | 无 |
| 估算大小 | 256 bytes | 160 bytes |

精确影响：

- 第一个 quad 的前部字段通常仍可读。
- `border_color`、`corner_radii`、`border_widths` 从错误 offset 读取。
- `b_quads[1]` 会按 WGSL 的 160-byte stride 读取，而 Rust 实际第二个 quad 从 256-byte offset 开始。
- batch 内第二个及后续 quad 的 `bounds`、`content_mask`、`background` 可能来自前一个 Rust quad 的尾部字段。

### 3. Renderer 直接上传 Rust 原始内存

WGPU renderer 当前没有对 `Quad` 做中间 repack：

```rust
fn draw_quads(...) -> bool {
    let data = unsafe { Self::instance_bytes(quads) };
    self.draw_instances(data, quads.len() as u32, ...)
}
```

```rust
unsafe fn instance_bytes<T>(instances: &[T]) -> &[u8] {
    std::slice::from_raw_parts(
        instances.as_ptr() as *const u8,
        std::mem::size_of_val(instances),
    )
}
```

因此 WGSL struct layout 必须与 Rust `#[repr(C)]` bytes 匹配，否则 GPU 一定会按错误 offset/stride 读取。

### 4. Windows/macOS shader 已包含新字段

Windows HLSL 与 macOS Metal 路径已经使用：

- `Background.colors[0..3]`
- `Background.stop_count`
- `Quad.continuous_corners`
- `Quad.transform`
- `Quad.blend_mode`
- transformed position / transformed clip

这说明能力本身已经存在；Linux/WGPU 主要是 shader ABI 和语义没有同步。

## 修正后的判断边界

### 不是所有字段都从第一个字节开始错

更准确的判断是：

- `Quad.order`、`border_style`、`bounds`、`content_mask` 前部仍大体对齐。
- `Background` 的前半部分也大体对齐，尤其 simple solid background 可能正常显示。
- 错误从 `Background` 被截断开始扩散到 `border_color`、`corner_radii`、`border_widths`。
- batch 内第二个及后续 quad 会受到 array stride 失配的系统性影响。

### “实际效果变化不大”不否定结论

此前主要新增的是文档、probe 和 demo example，并没有修改核心 WGPU shader。因此实际 app 没有明显改善是预期内的。

即便修复 frame skip commit，也只能改善 Wayland frame callback/liveness 问题，不能修复 shader 读错 `Quad` bytes 的问题。

### `update_transparency(!opaque)` 不是反逻辑

当前代码：

```rust
let opaque = !state.is_transparent();
state.renderer.update_transparency(!opaque);
```

`!opaque` 等价于 `state.is_transparent()`，而 `update_transparency` 参数语义就是 `transparent: bool`。因此这不是当前首要问题。

### Premultiplied alpha 是次级假设

WGPU 路径确实存在 surface alpha mode、shader premultiply、pipeline blend state 三者一致性问题需要继续验证。但当前代码里这三处至少使用同一个 alpha mode 派生，不像 ABI/stride 失配那样已有闭合证据。

## 修改状态与后续顺序

### 本次已实施的最小修复

已完成 P0 级修复：

- `Background.colors` 从 2 个 stop 扩展为 4 个 stop。
- `Background` 补齐 `stop_count`、`center`、`radius`。
- `Quad` 补齐 `continuous_corners`、`pad_before_transform`、`transform`、`blend_mode`、`pad_end`。
- `prepare_gradient_color` 的 `colors` 参数同步为 `array<LinearColorStop, 4>`。

P0 的目标是先让 WGPU storage buffer 中的 `Background` / `Quad` layout 和 Rust `repr(C)` scene struct 对齐，避免按错误 offset/stride 读取 instance 数据。

后续已继续完成 shader-only 修复：

- `vs_quad` 使用 `quad.transform` 生成 transformed position。
- quad clip distance 改为 transformed clip。
- linear gradient 支持最多 4 个 stops 和 `stop_count`。
- radial/conic gradient tag 与 Rust `BackgroundTag` 对齐。
- continuous corners 接入 squircle SDF。

仍未完成的是 non-normal `blend_mode`。该能力需要在 WGPU renderer 中为单个 blend quad 复制当前 framebuffer，并给 quad pipeline 额外绑定 framebuffer texture/sampler；它不是单纯补 WGSL 字段即可完成的改动。

### P0：先让 WGSL struct ABI 与 Rust 对齐

修改文件：

- `crates/gpui/src/platform/wgpu/shaders.wgsl`

建议先做最小 ABI 修复：

```wgsl
struct Background {
    tag: u32,
    color_space: u32,
    solid: Hsla,
    gradient_angle_or_pattern_height: f32,
    colors: array<LinearColorStop, 4>,
    stop_count: u32,
    center: vec2<f32>,
    radius: vec2<f32>,
}

struct Quad {
    order: u32,
    border_style: u32,
    bounds: Bounds,
    content_mask: Bounds,
    background: Background,
    border_color: Hsla,
    corner_radii: Corners,
    border_widths: Edges,
    continuous_corners: u32,
    _pad_before_transform: u32,
    transform: TransformationMatrix,
    blend_mode: u32,
    _pad_end: u32,
}
```

注意：

- `TransformationMatrix` 在当前 WGSL 中已经存在，并且 sprite 路径已经使用它。
- 这一步的首要目标是修正 storage buffer layout/stride，即使暂时不完整实现所有新语义，也应先避免 GPU 错读后续字段。

### P1：让 quad vertex path 使用 transform 和 transformed clip

当前 WGPU `vs_quad` 使用：

```wgsl
out.position = to_device_position(unit_vertex, quad.bounds);
out.clip_distances = distance_from_clip_rect(unit_vertex, quad.bounds, quad.content_mask);
```

应对齐 Windows/macOS 路径：

- position 使用 `to_device_position_transformed(unit_vertex, quad.bounds, quad.transform)`。
- clip 使用 transformed clip helper。
- fragment 中用于 SDF/gradient 的 position/framebuffer position 也要与 transform 后坐标保持一致。

### P2：补齐 gradient / fill 语义

当前 WGPU quad varying 只携带两个 gradient color：

- `background_color0`
- `background_color1`

建议对齐 Windows/macOS：

- varying 增加 `background_color2`、`background_color3`。
- `prepare_gradient_color` 接收 4 个 stops 和 `stop_count`。
- `gradient_color` 支持 4 stops，并继续处理 solid/linear/path/radial/conic 的 tag 语义。

### P3：补齐 `continuous_corners` 和 `blend_mode`

建议参考：

- `crates/gpui/src/platform/windows/shaders.hlsl`
- `crates/gpui/src/platform/mac/shaders.metal`

需要补齐：

- continuous corner SDF 分支。
- non-normal blend mode 的 framebuffer read/apply path。
- WGPU pipeline/bind group 是否需要额外 framebuffer texture 输入。

这部分可以在 ABI 修复之后分步完成，不应阻塞 P0。

### P4：再验证 Wayland alpha/frame 问题

只有在 P0/P1 修复后仍存在明显异常时，再回到这些假设：

- transparent surface / CSD / opaque region。
- `CompositeAlphaMode::PreMultiplied` 与 shader output/pipeline blend/compositor 的一致性。
- frame callback、surface commit、cached present。

否则这些实验容易被 `Quad` bytes 错读干扰，结论不稳定。

## 验证命令

### Layout probe

```bash
python3 docs/tools/wgpu_shader_layout_probe.py
```

P0 修复后预期结果：

```text
Background: MATCH
Quad: MATCH
```

### 编译验证

```bash
cargo check -p adabraka-gpui --example linux_rendering_probe
```

### 实机视觉验证

建议优先观察这些 case：

- toggle track 是否从“只剩 knob/小方块”恢复为完整 pill track。
- 1px separator 是否稳定可见。
- rounded card/nav pill 是否恢复圆角和边框。
- progress fill / multi-stop gradient 是否稳定显示。
- 滚动/交互前后是否仍存在“偶然出现/消失”。

## 风险与开放问题

- WGSL storage buffer layout 与 Rust `repr(C)` 的一致性仍靠人工维护；后续最好引入自动生成或更严格的 layout probe。
- 修复 ABI 后，仍可能暴露独立的 alpha/compositor 问题。
- `blend_mode` 在 WGPU 下可能需要额外 framebuffer read 支持，不能只补 struct 字段。
- radial/conic gradient tag 与当前 WGSL 注释/实现可能仍不一致，需要单独对齐 Windows/macOS 语义。
