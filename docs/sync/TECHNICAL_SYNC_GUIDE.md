# Adabraka GPUI 技术同步指南

> **历史文档**：本文代码映射基于 crate 拆分前结构。当前同步边界和验证命令见 [`CURRENT.md`](./CURRENT.md)。

本文档提供详细的技术映射和代码级别的同步指南。

## 代码结构对比

### GPUI 核心模块结构

```
crates/gpui/src/
├── app.rs                    # 应用上下文 [MODIFIED]
├── window.rs                 # 窗口管理 [MODIFIED]
├── platform.rs               # 平台抽象 [MODIFIED]
├── platform/
│   ├── mac/
│   │   ├── platform.rs       # macOS 平台实现 [MODIFIED]
│   │   ├── window.rs         # macOS 窗口 [MODIFIED]
│   │   ├── metal_renderer.rs # Metal 渲染器 [SYNC]
│   │   ├── text_system.rs    # 文本系统 [SYNC]
│   │   ├── tray.rs           # [ADABRAKA ONLY]
│   │   ├── global_hotkey.rs  # [ADABRAKA ONLY]
│   │   ├── notification.rs   # [ADABRAKA ONLY]
│   │   ├── autolaunch.rs     # [ADABRAKA ONLY]
│   │   ├── single_instance.rs # [ADABRAKA ONLY]
│   │   ├── focused_window.rs # [ADABRAKA ONLY]
│   │   └── permissions.rs    # [ADABRAKA ONLY]
│   ├── linux/
│   │   ├── platform.rs       # Linux 平台实现 [MODIFIED]
│   │   ├── window.rs         # Linux 窗口 [MODIFIED]
│   │   ├── wayland/          # Wayland 支持 [SYNC]
│   │   ├── x11/              # X11 支持 [SYNC]
│   │   ├── tray.rs           # [ADABRAKA ONLY]
│   │   ├── global_hotkey.rs  # [ADABRAKA ONLY]
│   │   ├── notification.rs   # [ADABRAKA ONLY]
│   │   ├── autolaunch.rs     # [ADABRAKA ONLY]
│   │   └── single_instance.rs # [ADABRAKA ONLY]
│   └── windows/
│       ├── platform.rs       # Windows 平台实现 [MODIFIED]
│       ├── window.rs         # Windows 窗口 [MODIFIED]
│       ├── directx_renderer.rs # DirectX 渲染器 [SYNC]
│       ├── tray.rs           # [ADABRAKA ONLY]
│       ├── global_hotkey.rs  # [ADABRAKA ONLY]
│       ├── notification.rs   # [ADABRAKA ONLY]
│       ├── autolaunch.rs     # [ADABRAKA ONLY]
│       └── single_instance.rs # [ADABRAKA ONLY]
├── scene.rs                  # 场景图 [SYNC]
├── element.rs                # 元素系统 [SYNC]
├── style.rs                  # 样式系统 [SYNC]
├── text_system.rs            # 文本渲染 [SYNC]
├── executor.rs               # 异步执行器 [SYNC]
├── keymap.rs                 # 键盘映射 [SYNC]
├── action.rs                 # 动作系统 [SYNC]
├── geometry.rs               # 几何类型 [SYNC]
├── color.rs                  # 颜色系统 [MODIFIED]
└── ...

[SYNC] = 直接同步 Zed 更新
[MODIFIED] = 需要合并 Adabraka 扩展
[ADABRAKA ONLY] = Adabraka 独有文件
```

## API 映射表

### App Context API

#### Zed GPUI
```rust
impl App {
    pub fn new() -> Self;
    pub fn run<F>(self, on_finish_launching: F);
    pub fn quit(&mut self);
    pub fn on_quit<F>(&mut self, callback: F);
    pub fn open_window<V>(&mut self, options: WindowOptions, build_view: F);
    pub fn update<R>(&mut self, update: impl FnOnce(&mut Self) -> R) -> R;
}
```

#### Adabraka GPUI (扩展)
```rust
impl App {
    // Zed 原有 API (保持兼容)
    pub fn new() -> Self;
    pub fn run<F>(self, on_finish_launching: F);
    pub fn quit(&mut self);
    pub fn on_quit<F>(&mut self, callback: F);
    pub fn open_window<V>(&mut self, options: WindowOptions, build_view: F);
    pub fn update<R>(&mut self, update: impl FnOnce(&mut Self) -> R) -> R;
    
    // Adabraka 扩展 API
    pub fn set_quit_mode(&mut self, mode: QuitMode);
    pub fn set_tray_icon(&mut self, icon: Image);
    pub fn set_tray_tooltip(&mut self, tooltip: impl Into<String>);
    pub fn set_tray_menu(&mut self, menu: Vec<TrayMenuItem>);
    pub fn on_tray_menu_action<F>(&mut self, callback: F);
    pub fn register_global_hotkey(&mut self, hotkey: Hotkey) -> Result<HotkeyId>;
    pub fn unregister_global_hotkey(&mut self, id: HotkeyId);
    pub fn show_notification(&mut self, notification: Notification);
    pub fn enable_autolaunch(&mut self) -> Result<()>;
    pub fn disable_autolaunch(&mut self) -> Result<()>;
    pub fn ensure_single_instance(&mut self, app_id: &str) -> Result<()>;
    
    // macOS 特定
    #[cfg(target_os = "macos")]
    pub fn get_focused_window_info(&self) -> Option<FocusedWindowInfo>;
    #[cfg(target_os = "macos")]
    pub fn check_accessibility_permission(&self) -> PermissionStatus;
    #[cfg(target_os = "macos")]
    pub fn check_microphone_permission(&self) -> PermissionStatus;
}
```

### Window API

#### Zed GPUI
```rust
impl Window {
    pub fn show(&mut self);
    pub fn hide(&mut self);
    pub fn minimize(&mut self);
    pub fn maximize(&mut self);
    pub fn set_title(&mut self, title: impl Into<String>);
    pub fn set_size(&mut self, size: Size<Pixels>);
    pub fn set_position(&mut self, position: Point<Pixels>);
}
```

#### Adabraka GPUI (扩展)
```rust
impl Window {
    // Zed 原有 API
    pub fn show(&mut self);
    pub fn hide(&mut self);
    pub fn minimize(&mut self);
    pub fn maximize(&mut self);
    pub fn set_title(&mut self, title: impl Into<String>);
    pub fn set_size(&mut self, size: Size<Pixels>);
    pub fn set_position(&mut self, position: Point<Pixels>);
    
    // Adabraka 扩展 API
    pub fn set_always_on_top(&mut self, always_on_top: bool);
    pub fn set_click_through(&mut self, click_through: bool);
    pub fn set_transparent(&mut self, transparent: bool);
    pub fn set_decorations(&mut self, decorations: bool);
}
```

### Platform Trait

#### Zed GPUI
```rust
pub trait Platform: Send + Sync {
    fn run(&self, on_finish_launching: Box<dyn FnOnce()>);
    fn quit(&self);
    fn open_window(&self, options: WindowOptions) -> Box<dyn PlatformWindow>;
    fn display_link(&self) -> Box<dyn DisplayLink>;
    // ... 其他方法
}
```

#### Adabraka GPUI (扩展)
```rust
pub trait Platform: Send + Sync {
    // Zed 原有方法
    fn run(&self, on_finish_launching: Box<dyn FnOnce()>);
    fn quit(&self);
    fn open_window(&self, options: WindowOptions) -> Box<dyn PlatformWindow>;
    fn display_link(&self) -> Box<dyn DisplayLink>;
    
    // Adabraka 扩展方法
    fn set_quit_mode(&self, mode: QuitMode);
    fn set_tray_icon(&self, icon: Image);
    fn set_tray_menu(&self, menu: Vec<TrayMenuItem>);
    fn on_tray_menu_action(&self, callback: Box<dyn Fn(String)>);
    fn register_global_hotkey(&self, hotkey: Hotkey) -> Result<HotkeyId>;
    fn unregister_global_hotkey(&self, id: HotkeyId);
    fn show_notification(&self, notification: Notification);
    // ... 其他扩展方法
}
```

## 同步检查清单

### 每次同步前检查

- [ ] 备份当前 Adabraka 代码
- [ ] 创建新的同步分支
- [ ] 记录 Zed 源 commit hash
- [ ] 识别要同步的文件列表

### 核心文件同步检查

#### 渲染系统
- [ ] `scene.rs` - 场景图更新
- [ ] `element.rs` - 元素系统更新
- [ ] `platform/*/metal_renderer.rs` - Metal 渲染器
- [ ] `platform/*/directx_renderer.rs` - DirectX 渲染器
- [ ] `platform/linux/wayland/renderer.rs` - Wayland 渲染器
- [ ] `platform/linux/x11/renderer.rs` - X11 渲染器

#### 文本系统
- [ ] `text_system.rs` - 文本渲染核心
- [ ] `text_system/` - 文本系统模块
- [ ] `platform/*/text_system.rs` - 平台文本实现

#### 布局系统
- [ ] `style.rs` - 样式定义
- [ ] `styled.rs` - 样式应用
- [ ] `taffy.rs` - Taffy 布局集成
- [ ] `geometry.rs` - 几何类型

#### 事件系统
- [ ] `input.rs` - 输入事件
- [ ] `keymap.rs` - 键盘映射
- [ ] `action.rs` - 动作系统
- [ ] `key_dispatch.rs` - 键盘分发

#### 平台抽象
- [ ] `platform.rs` - 平台 trait 定义
- [ ] `platform/mac/platform.rs` - macOS 实现
- [ ] `platform/linux/platform.rs` - Linux 实现
- [ ] `platform/windows/platform.rs` - Windows 实现

### Adabraka 扩展保护检查

#### App Context 扩展
- [ ] `set_quit_mode` 方法保留
- [ ] 托盘相关方法保留
- [ ] 全局热键方法保留
- [ ] 通知方法保留
- [ ] 自动启动方法保留
- [ ] 单实例锁方法保留

#### Window 扩展
- [ ] `set_always_on_top` 方法保留
- [ ] `set_click_through` 方法保留
- [ ] 透明窗口支持保留

#### Platform 扩展
- [ ] 托盘实现文件保留
- [ ] 全局热键实现文件保留
- [ ] 通知实现文件保留
- [ ] 其他 Adabraka 特有文件保留

### 测试检查

#### 单元测试
- [ ] `cargo test --lib` 通过
- [ ] 所有平台测试通过
- [ ] Adabraka 特性测试通过

#### 集成测试
- [ ] 示例程序编译通过
- [ ] `daemon_app` 示例运行正常
- [ ] 托盘功能正常
- [ ] 全局热键功能正常
- [ ] 通知功能正常

#### 平台测试
- [ ] macOS 测试通过
- [ ] Linux (X11) 测试通过
- [ ] Linux (Wayland) 测试通过
- [ ] Windows 测试通过

## 常见同步场景

### 场景 1: 纯渲染系统更新

**示例**: Zed 更新了 Metal 渲染器性能

**步骤**:
1. 直接复制 `platform/mac/metal_renderer.rs`
2. 检查依赖更新
3. 运行测试
4. 提交

**风险**: 低 - 不涉及 Adabraka 扩展

### 场景 2: App Context 更新

**示例**: Zed 添加了新的生命周期回调

**步骤**:
1. 对比 `app.rs` 差异
2. 应用 Zed 更改
3. 确保 Adabraka 扩展方法仍然存在
4. 更新 Adabraka 扩展以兼容新的生命周期
5. 运行测试
6. 提交

**风险**: 中 - 需要合并 Adabraka 扩展

### 场景 3: Platform Trait 更新

**示例**: Zed 修改了 Platform trait 签名

**步骤**:
1. 对比 `platform.rs` 差异
2. 应用 Zed 更改到 trait 定义
3. 更新所有平台实现（mac/linux/windows）
4. 确保 Adabraka 扩展方法兼容
5. 更新 Adabraka 特有的平台实现
6. 运行所有平台测试
7. 提交

**风险**: 高 - 影响所有平台实现

### 场景 4: 依赖版本更新

**示例**: Zed 升级了 `wgpu` 版本

**步骤**:
1. 对比 `Cargo.toml` 差异
2. 更新依赖版本
3. 检查 API 兼容性
4. 更新受影响的代码
5. 运行完整测试套件
6. 提交

**风险**: 中到高 - 取决于 API 变化

## 代码合并模式

### 模式 1: 直接替换

适用于不涉及 Adabraka 扩展的文件。

```bash
# 直接复制文件
cp /Users/hejun/work/my/zed/crates/gpui/src/scene.rs \
   /Users/hejun/work/my/adabraka-gpui/crates/gpui/src/scene.rs
```

### 模式 2: 三方合并

适用于有 Adabraka 扩展的文件。

```bash
# 使用 git merge-file
git merge-file \
  /Users/hejun/work/my/adabraka-gpui/crates/gpui/src/app.rs \
  <base-version> \
  /Users/hejun/work/my/zed/crates/gpui/src/app.rs
```

### 模式 3: 手动合并

适用于复杂的冲突情况。

```rust
// 1. 保存 Adabraka 扩展到临时文件
// 2. 应用 Zed 更新
// 3. 重新应用 Adabraka 扩展
// 4. 测试兼容性

// 示例：app.rs 合并
impl App {
    // === Zed 原有方法（从 Zed 同步） ===
    pub fn new() -> Self {
        // Zed 的实现
    }
    
    pub fn run<F>(self, on_finish_launching: F) {
        // Zed 的实现
    }
    
    // === Adabraka 扩展方法（保留） ===
    pub fn set_quit_mode(&mut self, mode: QuitMode) {
        // Adabraka 的实现
        self.platform.set_quit_mode(mode);
    }
    
    pub fn set_tray_icon(&mut self, icon: Image) {
        // Adabraka 的实现
        self.platform.set_tray_icon(icon);
    }
}
```

## 版本兼容性矩阵

| Adabraka GPUI | 基于 Zed GPUI | Zed Commit | 同步日期 |
|---------------|---------------|------------|----------|
| 0.6.0 | 0.2.2 | TBD | 2024-04 |
| 0.5.1 | 0.2.1 | TBD | 2024-04 |
| 0.5.0 | 0.2.1 | TBD | 2024-04 |

## 自动化工具

### 差异检测脚本

```bash
#!/bin/bash
# scripts/check-zed-diff.sh

ZED_PATH="/Users/hejun/work/my/zed"
ADABRAKA_PATH="/Users/hejun/work/my/adabraka-gpui"

# 核心文件列表
CORE_FILES=(
    "crates/gpui/src/scene.rs"
    "crates/gpui/src/element.rs"
    "crates/gpui/src/style.rs"
    "crates/gpui/src/text_system.rs"
    "crates/gpui/src/geometry.rs"
)

for file in "${CORE_FILES[@]}"; do
    echo "Checking $file..."
    diff -u "$ZED_PATH/$file" "$ADABRAKA_PATH/$file" || true
done
```

### 同步辅助脚本

```bash
#!/bin/bash
# scripts/sync-from-zed.sh

set -e

ZED_PATH="/Users/hejun/work/my/zed"
ADABRAKA_PATH="/Users/hejun/work/my/adabraka-gpui"
SYNC_BRANCH="sync/zed-$(date +%Y-%m-%d)"

cd "$ADABRAKA_PATH"

# 创建同步分支
git checkout -b "$SYNC_BRANCH"

# 同步核心文件（不涉及 Adabraka 扩展）
echo "Syncing core files..."
cp "$ZED_PATH/crates/gpui/src/scene.rs" \
   "$ADABRAKA_PATH/crates/gpui/src/scene.rs"

# 运行测试
echo "Running tests..."
cargo test --lib

echo "Sync complete. Review changes and commit."
```

## 故障排除

### 问题 1: 编译错误 - 缺少 Adabraka 方法

**症状**: 编译时提示找不到 `set_tray_icon` 等方法

**原因**: Zed 更新覆盖了 Adabraka 扩展

**解决**:
1. 检查 `app.rs` 是否包含 Adabraka 扩展方法
2. 从备份恢复 Adabraka 扩展
3. 重新合并

### 问题 2: 运行时错误 - 平台方法未实现

**症状**: 运行时 panic: "method not implemented"

**原因**: Platform trait 更新但平台实现未同步

**解决**:
1. 检查所有平台实现（mac/linux/windows）
2. 确保实现了所有 trait 方法
3. 更新 Adabraka 扩展的平台实现

### 问题 3: 测试失败 - Adabraka 特性不工作

**症状**: 托盘、热键等功能测试失败

**原因**: Zed 更新影响了 Adabraka 扩展的实现

**解决**:
1. 检查 Adabraka 特有文件是否被修改
2. 审查 Platform trait 变化
3. 更新 Adabraka 实现以适配新的 Platform API

## 最佳实践

### 1. 增量同步
- 不要一次同步太多更改
- 每次同步后立即测试
- 保持小的、可管理的提交

### 2. 保持文档更新
- 记录每次同步的 Zed commit hash
- 更新版本兼容性矩阵
- 记录遇到的问题和解决方案

### 3. 自动化测试
- 在 CI 中运行所有平台测试
- 测试 Adabraka 特有功能
- 回归测试核心功能

### 4. 代码审查
- 所有同步都需要代码审查
- 特别关注 Adabraka 扩展的兼容性
- 确保没有意外删除 Adabraka 功能

### 5. 版本管理
- 使用语义化版本
- 主版本号变化表示不兼容更新
- 次版本号变化表示新功能
- 补丁版本号变化表示 bug 修复

---

**维护者**: Adabraka Team
**最后更新**: 2026-04-30
