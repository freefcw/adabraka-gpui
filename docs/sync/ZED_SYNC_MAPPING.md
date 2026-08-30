# Zed to Adabraka GPUI Sync Mapping

> **历史文档**：本文描述 crate 拆分前的目录结构，不能作为当前同步操作指南。当前基线、路径映射和验证命令见 [`CURRENT.md`](./CURRENT.md)。

本文档记录了 Adabraka GPUI 仓库与 Zed 源仓库之间的映射关系，用于将 Zed 仓库的更新同步到本仓库。

## 仓库信息

- **Zed 源仓库**: `/Users/hejun/work/my/zed`
- **Adabraka GPUI 仓库**: `/Users/hejun/work/my/adabraka-gpui`
- **提取时间**: 2024年4月（基于 GPUI v0.2.1）
- **提取方式**: 从 crates.io 导入 GPUI，然后重命名为独立的发布包命名空间（当前为 `fc-gpui*`）

## 核心提取策略

Adabraka GPUI 是从 Zed 仓库中提取的 GPUI 框架及其依赖 crates 的独立版本，专注于：
1. GPU 加速的 UI 框架核心功能
2. 系统托盘、全局热键、通知等桌面应用功能
3. 守护进程模式支持
4. 跨平台支持（macOS、Linux、Windows）

## Crate 映射关系

### 主要 Crates

| Adabraka GPUI Crate | Zed Crate | 说明 |
|---------------------|-----------|------|
| `fc-gpui` | `gpui` | 核心 UI 框架，包含窗口管理、渲染、事件系统 |
| `fc-gpui-macros` | `gpui_macros` | GPUI 宏支持（derive macros） |
| `fc-gpui-collections` | `collections` | 集合类型工具 |
| `fc-gpui-util` | `util` | 通用工具函数和类型 |
| `fc-gpui-util-macros` | `util_macros` | 工具宏 |
| `fc-gpui-refineable` | `refineable` | 可精化类型系统 |
| `fc-gpui-derive-refineable` | `derive_refineable` | Refineable derive 宏 |
| `fc-gpui-sum-tree` | `sum_tree` | Sum tree 数据结构 |
| `fc-gpui-semantic-version` | `semantic_version` | 语义化版本支持 |
| `fc-gpui-http-client` | `http_client` | HTTP 客户端 |
| `fc-gpui-media` | `media` | 媒体处理（音频/视频） |
| `fc-gpui-perf` | `perf` | 性能分析工具 |

### 目录结构映射

```
zed/crates/                          → adabraka-gpui/crates/
├── gpui/                            → gpui/
│   ├── src/                         → src/
│   │   ├── app.rs                   → app.rs
│   │   ├── window.rs                → window.rs
│   │   ├── platform.rs              → platform.rs
│   │   ├── platform/                → platform/
│   │   │   ├── mac/                 → mac/
│   │   │   ├── linux/               → linux/
│   │   │   └── windows/             → windows/
│   │   ├── elements/                → elements/
│   │   ├── text_system/             → text_system/
│   │   └── ...                      → ...
│   ├── examples/                    → examples/
│   ├── tests/                       → tests/
│   ├── Cargo.toml                   → Cargo.toml (修改 package name)
│   └── build.rs                     → build.rs
├── gpui_macros/                     → gpui-macros/
├── collections/                     → collections/
├── util/                            → util/
├── util_macros/                     → util_macros/
├── refineable/                      → refineable/
├── derive_refineable/               → derive_refineable/
├── sum_tree/                        → sum_tree/
├── semantic_version/                → semantic_version/
├── http_client/                     → http_client/
├── media/                           → media/
└── perf/                            → perf/
```

## 关键文件映射

### GPUI 核心文件

| Zed 文件路径 | Adabraka GPUI 文件路径 | 说明 |
|-------------|----------------------|------|
| `crates/gpui/src/app.rs` | `crates/gpui/src/app.rs` | 应用程序上下文和生命周期 |
| `crates/gpui/src/window.rs` | `crates/gpui/src/window.rs` | 窗口管理 |
| `crates/gpui/src/platform.rs` | `crates/gpui/src/platform.rs` | 平台抽象层 |
| `crates/gpui/src/platform/mac/` | `crates/gpui/src/platform/mac/` | macOS 平台实现 |
| `crates/gpui/src/platform/linux/` | `crates/gpui/src/platform/linux/` | Linux 平台实现 |
| `crates/gpui/src/platform/windows/` | `crates/gpui/src/platform/windows/` | Windows 平台实现 |
| `crates/gpui/src/scene.rs` | `crates/gpui/src/scene.rs` | 场景图和渲染 |
| `crates/gpui/src/element.rs` | `crates/gpui/src/element.rs` | 元素系统 |
| `crates/gpui/src/style.rs` | `crates/gpui/src/style.rs` | 样式系统 |
| `crates/gpui/src/text_system.rs` | `crates/gpui/src/text_system.rs` | 文本渲染系统 |
| `crates/gpui/src/executor.rs` | `crates/gpui/src/executor.rs` | 异步执行器 |
| `crates/gpui/src/keymap.rs` | `crates/gpui/src/keymap.rs` | 键盘映射 |
| `crates/gpui/src/action.rs` | `crates/gpui/src/action.rs` | 动作系统 |

### 平台特定功能

#### macOS
| Zed 文件 | Adabraka GPUI 文件 | 功能 |
|---------|-------------------|------|
| `crates/gpui/src/platform/mac/platform.rs` | `crates/gpui/src/platform/mac/platform.rs` | macOS 平台入口 |
| `crates/gpui/src/platform/mac/window.rs` | `crates/gpui/src/platform/mac/window.rs` | macOS 窗口实现 |
| `crates/gpui/src/platform/mac/metal_renderer.rs` | `crates/gpui/src/platform/mac/metal_renderer.rs` | Metal 渲染器 |
| N/A | `crates/gpui/src/platform/mac/tray.rs` | 系统托盘（Adabraka 新增） |
| N/A | `crates/gpui/src/platform/mac/global_hotkey.rs` | 全局热键（Adabraka 新增） |
| N/A | `crates/gpui/src/platform/mac/notification.rs` | 原生通知（Adabraka 新增） |

#### Linux
| Zed 文件 | Adabraka GPUI 文件 | 功能 |
|---------|-------------------|------|
| `crates/gpui/src/platform/linux/platform.rs` | `crates/gpui/src/platform/linux/platform.rs` | Linux 平台入口 |
| `crates/gpui/src/platform/linux/window.rs` | `crates/gpui/src/platform/linux/window.rs` | Linux 窗口实现 |
| `crates/gpui/src/platform/linux/wayland/` | `crates/gpui/src/platform/linux/wayland/` | Wayland 支持 |
| `crates/gpui/src/platform/linux/x11/` | `crates/gpui/src/platform/linux/x11/` | X11 支持 |
| N/A | `crates/gpui/src/platform/linux/tray.rs` | 系统托盘（DBus/SNI） |
| N/A | `crates/gpui/src/platform/linux/global_hotkey.rs` | 全局热键（X11） |
| N/A | `crates/gpui/src/platform/linux/notification.rs` | 原生通知 |

#### Windows
| Zed 文件 | Adabraka GPUI 文件 | 功能 |
|---------|-------------------|------|
| `crates/gpui/src/platform/windows/platform.rs` | `crates/gpui/src/platform/windows/platform.rs` | Windows 平台入口 |
| `crates/gpui/src/platform/windows/window.rs` | `crates/gpui/src/platform/windows/window.rs` | Windows 窗口实现 |
| `crates/gpui/src/platform/windows/directx_renderer.rs` | `crates/gpui/src/platform/windows/directx_renderer.rs` | DirectX 渲染器 |
| N/A | `crates/gpui/src/platform/windows/tray.rs` | 系统托盘 |
| N/A | `crates/gpui/src/platform/windows/global_hotkey.rs` | 全局热键 |
| N/A | `crates/gpui/src/platform/windows/notification.rs` | 原生通知 |

## Adabraka GPUI 独有功能

以下功能是 Adabraka GPUI 新增的，不存在于 Zed 源仓库中：

### 1. 系统托盘支持
- **文件**: `crates/gpui/src/platform/{mac,linux,windows}/tray.rs`
- **API**: `App::set_tray_icon()`, `App::set_tray_menu()`, `App::on_tray_menu_action()`
- **说明**: 跨平台系统托盘图标和菜单支持

### 2. 全局热键
- **文件**: `crates/gpui/src/platform/{mac,linux,windows}/global_hotkey.rs`
- **API**: `App::register_global_hotkey()`, `App::unregister_global_hotkey()`
- **说明**: 系统级全局热键注册（Linux 仅支持 X11）

### 3. 原生通知
- **文件**: `crates/gpui/src/platform/{mac,linux,windows}/notification.rs`
- **API**: `App::show_notification()`
- **说明**: 操作系统原生通知支持

### 4. 守护进程模式
- **修改**: `crates/gpui/src/app.rs`
- **API**: `App::set_quit_mode(QuitMode)`（原 `set_keep_alive_without_windows()`，已改为语义化 `QuitMode`）
- **说明**: 允许应用在没有可见窗口时继续运行

### 5. 窗口控制增强
- **修改**: `crates/gpui/src/window.rs`
- **API**: `Window::set_always_on_top()`, `Window::set_click_through()`
- **说明**: 覆盖窗口和点击穿透支持

### 6. 自动启动
- **文件**: `crates/gpui/src/platform/{mac,linux,windows}/autolaunch.rs`
- **API**: `App::enable_autolaunch()`, `App::disable_autolaunch()`
- **说明**: 登录时自动启动应用

### 7. 单实例锁
- **文件**: `crates/gpui/src/platform/{mac,linux,windows}/single_instance.rs`
- **API**: `App::ensure_single_instance()`
- **说明**: 防止多个应用实例运行

### 8. 焦点窗口信息
- **文件**: `crates/gpui/src/platform/{mac,windows}/focused_window.rs`
- **API**: `App::get_focused_window_info()`
- **说明**: 获取当前用户聚焦的窗口信息（macOS、Windows）

### 9. 权限查询
- **文件**: `crates/gpui/src/platform/mac/permissions.rs`
- **API**: `App::check_accessibility_permission()`, `App::check_microphone_permission()`
- **说明**: 检查系统权限状态（macOS）

### 10. 图像格式特性
- **修改**: `crates/gpui/Cargo.toml`
- **特性**: `image-format-png`, `image-format-jpeg`, `image-format-webp`, 等
- **说明**: 可选的图像解码器，减少二进制大小

## 同步工作流程

### 1. 识别 Zed 更新

监控 Zed 仓库中以下 crates 的更新：
```bash
cd /Users/hejun/work/my/zed
git log --oneline --since="2024-04-01" -- crates/gpui crates/gpui_macros crates/collections crates/util crates/refineable crates/sum_tree crates/semantic_version crates/http_client crates/media
```

### 2. 分类更新类型

#### A. 核心功能更新（需要同步）
- 渲染引擎改进
- 布局系统更新
- 文本系统优化
- 性能改进
- Bug 修复
- 平台兼容性修复

#### B. Zed 特定功能（不需要同步）
- 编辑器特定功能
- 协作功能
- AI 集成
- 项目管理
- 语言服务器集成

#### C. 冲突区域（需要仔细合并）
- `app.rs` - 应用生命周期（Adabraka 有守护进程模式扩展）
- `platform.rs` - 平台抽象（Adabraka 有托盘、热键等扩展）
- `window.rs` - 窗口管理（Adabraka 有覆盖窗口等扩展）

### 3. 同步步骤

#### 步骤 1: 创建同步分支
```bash
cd /Users/hejun/work/my/adabraka-gpui
git checkout -b sync/zed-YYYY-MM-DD
```

#### 步骤 2: 对比文件差异
```bash
# 对比单个文件
diff -u /Users/hejun/work/my/zed/crates/gpui/src/scene.rs \
        /Users/hejun/work/my/adabraka-gpui/crates/gpui/src/scene.rs

# 或使用 git diff
cd /Users/hejun/work/my/zed
git diff <old-commit> <new-commit> -- crates/gpui/src/scene.rs > /tmp/scene.patch
```

#### 步骤 3: 应用补丁
```bash
cd /Users/hejun/work/my/adabraka-gpui
# 手动审查并应用更改
# 注意保留 Adabraka 特有功能
```

#### 步骤 4: 更新依赖
检查 Zed 的 `Cargo.toml` 依赖更新，同步到 Adabraka：
```bash
# 对比依赖版本
diff /Users/hejun/work/my/zed/crates/gpui/Cargo.toml \
     /Users/hejun/work/my/adabraka-gpui/crates/gpui/Cargo.toml
```

#### 步骤 5: 测试
```bash
cd /Users/hejun/work/my/adabraka-gpui
cargo test --all
cargo build --examples
# 运行示例程序测试
cargo run --example daemon_app
```

#### 步骤 6: 提交
```bash
git add .
git commit -m "sync: merge changes from zed <commit-hash>

- Updated <file1>: <description>
- Fixed <issue>: <description>
- Preserved Adabraka features: <list>
"
```

### 4. 冲突解决策略

#### 策略 A: 保留 Adabraka 扩展
当 Zed 更新与 Adabraka 扩展冲突时：
1. 先应用 Zed 的更改
2. 然后重新应用 Adabraka 的扩展
3. 确保两者兼容

#### 策略 B: 条件编译
使用特性标志分离 Adabraka 特有功能：
```rust
#[cfg(feature = "daemon-mode")]
pub fn set_quit_mode(&mut self, mode: QuitMode) {
    // Adabraka specific
}
```

#### 策略 C: 扩展而非修改
尽量通过扩展而非修改 Zed 代码：
```rust
// 好的做法：扩展
impl App {
    // Zed 原有方法
    pub fn quit(&mut self) { ... }
    
    // Adabraka 扩展方法
    pub fn set_tray_icon(&mut self, icon: Image) { ... }
}

// 避免：直接修改 Zed 方法
```

## 重要注意事项

### 1. 命名空间
- 所有发布包名称使用 `fc-gpui*` 前缀
- 包名在 `Cargo.toml` 中为 `fc-gpui` 或 `fc-gpui-*`；`[lib] name` 保持不变
- 保持与 crates.io 发布一致

### 2. 版本管理
- 独立于 Zed 的版本号
- 遵循语义化版本
- 在 CHANGELOG.md 中记录所有更改

### 3. 许可证
- 保持 Apache-2.0 许可证
- 保留 Zed 的原始许可证声明
- 添加 Adabraka 的贡献声明

### 4. 文档
- 更新 README.md 反映 Adabraka 特性
- 保持示例代码最新
- 记录与 Zed 的差异

### 5. 测试
- 确保所有平台测试通过
- 测试 Adabraka 特有功能
- 回归测试核心功能

## 关键差异总结

| 方面 | Zed GPUI | Adabraka GPUI |
|-----|----------|---------------|
| **目标** | Zed 编辑器的 UI 框架 | 通用桌面应用框架 |
| **系统托盘** | ❌ | ✅ |
| **全局热键** | ❌ | ✅ |
| **原生通知** | ❌ | ✅ |
| **守护进程模式** | ❌ | ✅ |
| **覆盖窗口** | ❌ | ✅ |
| **自动启动** | ❌ | ✅ |
| **单实例锁** | ❌ | ✅ |
| **图像格式** | 全部内置 | 可选特性 |
| **发布到 crates.io** | ✅ | ✅ |

## 参考资源

- **Zed 仓库**: https://github.com/zed-industries/zed
- **Adabraka GPUI 仓库**: https://github.com/Augani/adabraka-gpui
- **GPUI 文档**: https://gpui.rs
- **Zed 更新日志**: https://github.com/zed-industries/zed/releases

## 维护者备注

- 定期检查 Zed 仓库更新（建议每月一次）
- 优先同步安全修复和性能改进
- 保持 Adabraka 特性的独立性和可维护性
- 考虑向 Zed 上游贡献通用改进

---

**最后更新**: 2026-04-30
**维护者**: Adabraka Team
