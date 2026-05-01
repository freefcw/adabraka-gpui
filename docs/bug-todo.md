# Bug TODO

经过代码级逐一验证的真实问题，按严重程度排序。

---

## BUG-1：着色器 `apply_blend_mode` 忽略目标色，混合模式语义错误

**严重程度**：高（功能性错误，所有平台）  
**文件**：
- `crates/gpui/src/platform/mac/shaders.metal:119`
- `crates/gpui/src/platform/wgpu/shaders.wgsl:584`
- `crates/gpui/src/platform/windows/shaders.hlsl:584`

**问题**：三份着色器中 `apply_blend_mode` 的实现完全相同，所有非 Normal 模式都只对 `src`（源色）自身做变换，完全忽略了 `dst`（目标/背景色）。标准混合模式定义是 `f(src, dst)`：

```metal
// 当前错误实现
case 1u: return float4(src.rgb * src.rgb, src.a);           // Multiply：应为 src * dst
case 2u: return float4(1.0 - (1.0 - src.rgb) * (1.0 - src.rgb), src.a); // Screen：应为 1-(1-src)(1-dst)
case 3u: { float3 mid = float3(0.5); ... }                  // Overlay：mid 应为 dst，不是常量 0.5
case 4u: return float4(src.rgb * src.rgb + ..., src.a);     // SoftLight：公式仅在 dst=0.5 时成立
case 5u: return float4(abs(src.rgb - 0.5), src.a);          // Difference：应为 abs(src - dst)
```

**影响**：使用 `.blend_mode(BlendMode::Multiply/Screen/Overlay/SoftLight/Difference)` 的元素渲染结果与 CSS/Photoshop 同名混合模式完全不同。

**修复方向**：需要在 fragment shader 中读取 framebuffer 当前值作为 `dst`。Metal 可通过 `[[color(0)]]` 读取，需开启 `blendingEnabled`；WGSL/HLSL 需要对应的 framebuffer 读取机制。

---

## BUG-2：Toast `push` 用数组下标标识 toast，`clear()` 后 timer 会删错 toast

**严重程度**：中（功能性错误）  
**文件**：`crates/gpui/src/elements/toast.rs:88`

**问题**：

```rust
let index = self.toasts.len() - 1;
cx.spawn_in(window, async move |this, cx| {
    Timer::after(duration).await;
    this.update(cx, |stack, cx| {
        if index < stack.toasts.len() {
            stack.toasts.remove(index);  // 用下标删除，下标可能已失效
            cx.notify();
        }
    }).ok();
}).detach();
```

复现场景：push toast A（index=0），push toast B（index=1），手动调用 `clear()` 清空，再 push toast C（index=0）。A 的 timer 到期时 `remove(0)` 会删掉 C 而不是 A（A 已不存在）。

**修复**：给每个 `ToastEntry` 分配唯一 ID，timer 到期时按 ID 查找删除：

```rust
struct ToastEntry {
    id: u64,
    toast: Toast,
}

// push 中：
let id = /* 递增计数器 */;
self.toasts.push(ToastEntry { id, toast });
cx.spawn_in(window, async move |this, cx| {
    Timer::after(duration).await;
    this.update(cx, |stack, cx| {
        stack.toasts.retain(|e| e.id != id);
        cx.notify();
    }).ok();
}).detach();
```

---

## BUG-3：take/restore 回调模式：callback 内重新注册会被覆盖

**严重程度**：中（逻辑错误，三个平台均有）  
**文件**：
- `crates/gpui/src/platform/mac/platform.rs:1963`（`handle_tray_menu_item`）
- `crates/gpui/src/platform/mac/platform.rs:1985`（`handle_tray_panel_click`）
- `crates/gpui/src/platform/mac/platform.rs:2003`（`handle_system_power_event`）
- `crates/gpui/src/platform/windows/platform.rs:1111`（`handle_global_hotkey`）
- `crates/gpui/src/platform/linux/platform.rs:86`（`dispatch_tray_menu_action`）

**问题**：所有平台的事件回调都使用 take/restore 模式：

```rust
// macOS handle_tray_menu_item
if let Some(mut callback) = lock.tray_menu_callback.take() {
    drop(lock);
    callback(ctx.id);
    platform.0.lock().tray_menu_callback = Some(callback); // ← 覆盖
}
```

如果用户在 callback 内部调用 `cx.on_tray_menu_action(new_callback)`，`restore` 会把旧 callback 写回，覆盖掉新注册的 callback。

**修复**：restore 前检查是否已有新 callback：

```rust
if let Some(mut callback) = lock.tray_menu_callback.take() {
    drop(lock);
    callback(ctx.id);
    let mut lock = platform.0.lock();
    if lock.tray_menu_callback.is_none() {  // 只在没有新 callback 时才 restore
        lock.tray_menu_callback = Some(callback);
    }
}
```

---

## BUG-4：X11 热键不处理 NumLock/CapsLock 修饰键，开启后热键失效

**严重程度**：中（功能性错误，Linux X11）  
**文件**：`crates/gpui/src/platform/linux/global_hotkey.rs:188`

**问题**：`XGrabKey` 只在修饰键**精确匹配**时触发。用户开启 NumLock（Mod2）或 CapsLock（Lock）后，注册的热键不会响应：

```rust
xcb.grab_key(
    false,
    root_window,
    modmask.into(),  // 只注册了精确的修饰键组合
    keycode,
    GrabMode::ASYNC,
    GrabMode::ASYNC,
)?
```

**修复**：对 NumLock/CapsLock 的四种组合各注册一次，`unregister` 时同样注销四次：

```rust
const MOD_NUMLOCK: u16 = 0x0010; // Mod2
const MOD_CAPSLOCK: u16 = 0x0002; // Lock

for extra in [0u16, MOD_NUMLOCK, MOD_CAPSLOCK, MOD_NUMLOCK | MOD_CAPSLOCK] {
    xcb.grab_key(false, root_window, (modmask | extra).into(), keycode,
                 GrabMode::ASYNC, GrabMode::ASYNC)?.check()?;
}
```

---

## BUG-5：Windows `create_hicon_from_bytes` 不验证 ICO 格式，传入 PNG 可能崩溃

**严重程度**：低（防御性问题，Windows）  
**文件**：`crates/gpui/src/platform/windows/tray.rs:238`

**问题**：`LookupIconIdFromDirectoryEx` 期望 ICO 格式输入（magic bytes `00 00 01 00`）。传入 PNG/WebP 等格式时，该函数可能返回非零的无意义 offset，导致 `CreateIconFromResourceEx` 读取垃圾数据，行为未定义：

```rust
fn create_hicon_from_bytes(data: &[u8]) -> Option<HICON> {
    unsafe {
        let offset = LookupIconIdFromDirectoryEx(data.as_ptr(), true, 0, 0, LR_DEFAULTCOLOR);
        if offset <= 0 { return None; }
        if (offset as usize) >= data.len() { return None; }
        // 没有验证 data 是否是合法 ICO 格式
        let icon_data = &data[offset as usize..];
        let hicon = CreateIconFromResourceEx(icon_data, true, 0x00030000, 0, 0, LR_DEFAULTCOLOR);
        hicon.ok()
    }
}
```

**修复**：在函数入口验证 ICO magic bytes：

```rust
fn create_hicon_from_bytes(data: &[u8]) -> Option<HICON> {
    // ICO format: reserved(2) + type(2) = 00 00 01 00
    if data.len() < 4 || &data[0..4] != &[0x00, 0x00, 0x01, 0x00] {
        return None;
    }
    // ... 原有逻辑
}
```

---

## BUG-6：macOS 热键注册每次暴力扫描 127 个虚拟键码

**严重程度**：低（性能问题，macOS）  
**文件**：`crates/gpui/src/platform/mac/global_hotkey.rs:22`

**问题**：`hotkey_to_native` 通过遍历所有虚拟键码（0x00~0x7e）来反查 keystroke 对应的 key code，每次调用 `semantic_hotkey_for_key_code` 都会调用底层 `UCKeyTranslate` 系统调用，最多执行 254 次（shift=false 和 shift=true 各 127 次）。键盘布局变化时 `reregister_global_hotkeys_for_current_layout` 会对每个已注册热键重复此过程。

**修复**：在 `MacKeyboardMapper` 初始化时预构建 `key_name → key_code` 的反向映射表，注册时直接查表，O(1) 完成。
