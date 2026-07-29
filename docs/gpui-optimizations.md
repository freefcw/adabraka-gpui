# Adabraka GPUI 优化与变更清单

从初始导入 GPUI v0.2.1 至今的所有优化和功能增强。

## 渲染增强

### 元素变换 (v0.3.0)
- **Commit**: e92732d
- 为 Quad 添加 TransformationMatrix 字段
- 支持 2D 仿射变换：`.rotate()`, `.scale()`, `.scale_xy()`, `.transform_origin()`
- 更新所有着色器后端（Metal, WGSL, HLSL）

### 渐变支持 (v0.3.0)
- **Commit**: c178fa7
- Background 从 2 色扩展到 4 色停止点，支持分段插值
- 新增 RadialGradient 和 ConicGradient
- 所有着色器后端实现 `interpolate_multi_stop`、径向距离和 atan2 锥形渐变

### 混合模式 (v0.3.0)
- **Commit**: a4ff20f
- 为 Quad 添加 BlendMode 枚举
- 支持：Normal, Multiply, Screen, Overlay, SoftLight, Difference
- 片段着色器在标准 alpha 合成前应用混合模式

### 元素调整事件 (v0.3.0)
- **Commit**: 672ecb3
- 新增 ResizeEvent（size, bounds）
- StatefulInteractiveElement 添加 `on_resize()` 方法
- 在 paint 时检测边界变化并触发监听器

## 跨平台桌面功能

### 守护进程模式 (v0.4.0)
- **Commit**: 3be75a6, bea2896, 72c044d, 7746069
- 无窗口保活：`set_keep_alive_without_windows(true)`
- macOS: `applicationShouldTerminateAfterLastWindowClosed` 委托
- Linux/Windows: 事件循环标志控制

### 系统托盘 (v0.4.0)
- **Commit**: 3be75a6, bea2896, 72c044d, 7746069
- macOS: NSStatusBar/NSStatusItem
- Linux: ksni (DBus StatusNotifierItem)
- Windows: Shell_NotifyIconW
- 支持图标、工具提示、嵌套菜单、动作回调
- **v0.4.0**: 修复 macOS 菜单冻结（dispatch_async）
- **v0.4.1**: 面板模式、菜单图标、内存泄漏修复
- **v0.5.3**: 多显示器坐标修复
- **v0.6.0**: 图标渲染模式控制（Adaptive/Original）
- **v0.6.0**: GNOME AppIndicator 显示修复（设置 SNI id）
- **v0.6.0**: 升级 ksni 0.2→0.3.4，修复 GNOME 点击行为
- Linux 托盘定位使用 StatusNotifierItem `activate(x, y)` 的屏幕坐标 hint 近似生成 `TrayAnchor`；该 hint 不是托盘图标真实 bounds。
- HiDPI 下会按显示器 scale 将 Linux 原始坐标转换为 GPUI 逻辑 `Pixels`；Wayland fractional scaling 因缺少关联 surface，只能使用 `wl_output.scale` 近似转换。

### 全局热键 (v0.4.0)
- **Commit**: 3be75a6, bea2896, 72c044d, 7746069
- macOS: Carbon RegisterEventHotKey（v0.6.0 从 NSEvent 迁移）
- Linux X11: XGrabKey
- Windows: RegisterHotKey
- **v0.4.1**: 键名规范化（特殊键匹配）
- **v0.6.0**: 语义按键到 Carbon 虚拟键码转换，支持箭头/导航/功能键

### 覆盖窗口 (v0.4.0)
- **Commit**: 3be75a6, bea2896, 72c044d, 7746069
- WindowKind::Overlay 变体
- macOS: level 25, stationary, all spaces
- Linux X11: shape extension
- Linux Wayland: wl_region
- Windows: HWND_TOPMOST
- 鼠标穿透：`set_mouse_passthrough()`

### 原生通知 (v0.4.0)
- **Commit**: 3be75a6, bea2896, 72c044d, 7746069
- macOS: UNUserNotificationCenter（防护无 bundle 崩溃）
- Linux: notify-rust
- Windows: Shell balloon
- 应用内 Toast 组件（可堆叠、自动消失）

### 窗口控制 (v0.4.0)
- **Commit**: 3be75a6, bea2896, 72c044d, 7746069
- `show()`, `hide()`, `is_visible()`
- 所有平台实现

### 自动启动 (v0.4.0)
- **Commit**: 3be75a6, bea2896, 72c044d, 7746069
- macOS: SMAppService (macOS 13+)
- Linux: XDG autostart desktop 文件
- Windows: Registry Run 键

### 单实例锁 (v0.4.0)
- **Commit**: 111057f
- macOS/Linux: Unix domain socket
- Windows: Named mutex
- 激活信号传递

### 焦点窗口信息 (v0.4.0)
- **Commit**: 3be75a6, bea2896, 72c044d, 7746069
- macOS: NSWorkspace + Accessibility API (AXUIElement)
- Linux X11: EWMH
- Windows: Win32

### 权限查询 (v0.4.0)
- **Commit**: 3be75a6, bea2896
- macOS: Accessibility (AXIsProcessTrusted), 麦克风
- 其他平台：存根

## 扩展桌面功能 (v0.5.0)

### 电源事件 (v0.5.0)
- **Commit**: 122f36c, 46a0b55, 5ccaff0, c9ff2e1
- macOS: NSWorkspace 通知观察者
- Windows: WM_POWERBROADCAST, WM_WTSSESSION_CHANGE
- Linux: 存根

### 网络状态 (v0.5.0)
- **Commit**: 122f36c, 46a0b55, 5ccaff0, c9ff2e1
- macOS: NWPathMonitor (Network framework)
- Windows: INetworkListManager COM
- Linux: 读取 /sys/class/net/*/operstate

### 媒体键 (v0.5.0)
- **Commit**: 122f36c, 46a0b55, 5ccaff0, c9ff2e1
- macOS: NSEvent global monitor (NSSystemDefinedMask)
- Windows: WM_APPCOMMAND
- Linux: XF86 keysym 拦截

### 原生对话框 (v0.5.0)
- **Commit**: 122f36c, 6b14e74, 5ccaff0, c9ff2e1
- macOS: NSAlert
- Windows: MessageBoxW
- Linux: zenity + kdialog 回退

### 进度条 (v0.5.0)
- **Commit**: 122f36c, b4528c7, 46a0b55, 5ccaff0
- macOS: NSDockTile + NSProgressIndicator
- Windows: ITaskbarList3
- Window API: `set_progress_bar()`

### 用户注意力请求 (v0.5.0)
- **Commit**: 122f36c, 5ccaff0, c9ff2e1
- Windows: FlashWindowEx
- Linux X11: EWMH _NET_WM_STATE_DEMANDS_ATTENTION

### 系统空闲时间 (v0.5.0)
- **Commit**: 122f36c, 5ccaff0, c9ff2e1
- Windows: GetLastInputInfo
- Linux X11: screensaver extension

### 省电阻止器 (v0.5.0)
- **Commit**: 122f36c, 6b14e74, 5ccaff0, c9ff2e1
- macOS: PreventUserIdleDisplaySleep
- Windows: SetThreadExecutionState
- Linux: dbus-send screensaver inhibit, systemd-inhibit

### 上下文菜单 (v0.5.0)
- **Commit**: 122f36c, 46a0b55, 5ccaff0
- macOS: NSMenu + handleContextMenuItem
- Windows: TrackPopupMenu
- Linux: 存根

### 系统信息 (v0.5.0)
- **Commit**: 122f36c, 6b14e74, 5ccaff0, c9ff2e1
- macOS: 系统 API
- Windows: RtlGetVersion
- Linux: 解析 /etc/os-release, /etc/hostname, locale 环境变量

### 生物识别认证 (v0.5.0)
- **Commit**: 122f36c, 6b14e74, 5ccaff0
- macOS: LocalAuthentication framework
- 其他平台：存根

### 窗口定位器 (v0.5.0)
- **Commit**: 67cef3e
- WindowPosition 枚举：Center, CenterOnScreen, TrayCenter, Custom
- **v0.5.3**: 新增 TrayAnchored(TrayAnchor)，弃用 TrayCenter

### 窗口状态 (v0.5.0)
- **Commit**: b4528c7, 46a0b55
- 全屏切换恢复

## 性能优化

### 场景图排序优化 (v0.5.1)
- **Commit**: ca2b967
- 使用 `is_sorted_by_key` 检查跳过已排序的 O(n log n) 排序
- GPU 实例缓冲区使用 1.5x 增长而非 next_power_of_two

### DirectX 优化 (v0.5.1)
- **Commit**: ca2b967, 76293ea
- DirectWrite 文本格式缓存（字体布局重用）
- 管线状态缓存（跳过冗余 set_pipeline_state 调用）
- 跨窗口文本布局缓存（全局 TextSystem 缓存）
- 修复 HLSL 着色器标识符冲突

### SharedString 优化 (v0.6.2)
- **Commit**: dd5cd1c
- 从 ArcCow 迁移到 smol_str
- 结构体大小从 32 字节减少到 24 字节
- 小字符串优化（< 23 字节内联存储）
- 保持 API 兼容性

### macOS 文本渲染优化 (v0.6.2)
- **Commit**: d8dd10d
- 基于颜色的字形膨胀（2 级：暗/亮）
- 使用 Rec. 709 标准计算亮度
- 改善文本渲染质量

## 平台迁移与现代化

### macOS objc2 迁移 (v0.6.0)
- **Commit**: 7146771, 53aa801, 76d19f8
- 从弃用的 cocoa crate 迁移到 objc2-app-kit/objc2-foundation
- 覆盖：dialog, display, dock, events, platform, tray, window, text_system, metal_renderer
- 移除直接 cocoa 依赖
- 消除下游弃用警告

### 依赖升级 (v0.6.0)
- **Commit**: 62eb2be
- 升级实用工具依赖
- 移除每个 crate 的 lockfile
- 固定 core-text=21.0.0（防止 core-graphics 冲突）

### 图像格式可选功能 (v0.6.0)
- **Commit**: e6dcdda
- 禁用 image crate 默认功能
- 暴露 `image-format-*` 和 `image-rayon` 功能
- 允许下游减少二进制体积

## Bug 修复

### macOS 修复
- **v0.4.0**: 通知无 bundle 崩溃防护
- **v0.4.0**: 托盘菜单 UI 冻结（dispatch_async）
- **v0.4.1**: NSImage 内存泄漏
- **v0.5.1**: HLSL 着色器标识符冲突
- **v0.6.0**: 窗口关闭后过时帧回调
- **v0.6.0**: 多显示器托盘位置计算
- **v0.6.0**: 多显示器上下文菜单坐标
- **v0.6.2**: 窗口失焦时恢复鼠标光标
- **v0.6.2**: 输入模式跟踪（键盘/鼠标）

### Linux 修复
- **v0.4.0**: ksni 后台线程清理
- **v0.4.0**: XDG autostart desktop 文件引用和清理
- **v0.4.0**: Wayland 显示时帧回调
- **v0.5.1**: XCBConnection::setup() 缺失 Connection trait 导入
- **v0.6.0**: GNOME AppIndicator 托盘图标显示（设置 SNI id）
- **v0.6.0**: GNOME 托盘点击行为（ksni 0.3.4 升级）
- **v0.6.0**: calloop 通道事件分发

### Windows 修复
- **v0.4.0**: WM_COMMAND 托盘菜单分发
- **v0.4.0**: 菜单项 ID 冲突（全局计数器）
- **v0.4.0**: TrackPopupMenu RefCell panic
- **v0.4.0**: HICON 资源泄漏
- **v0.4.0**: 鼠标穿透样式更改（WS_EX_LAYERED, SetWindowPos）
- **v0.5.1**: 平台编译问题

### 跨平台修复
- **v0.5.1**: 禁用 doctest 防止 SIGBUS（递归限制）
- **v0.6.1**: 锚定元素负坐标大小计算（Bounds::union）
- **v0.6.1**: GIF 渲染越界 panic（帧索引钳制）
- **v0.6.2**: 移除 naga 构建依赖（减少编译时间）

## 代码质量

### 格式化与 Lint (v0.4.1)
- **Commit**: 304bbdb
- 解决所有 clippy 警告
- 使用 `copy_from_slice` 替代手动切片复制
- 为枚举派生 Default
- 移除不必要的指针转换和解引用

### 文档
- **v0.4.0**: 设计文档（守护进程、托盘、热键等）
- **v0.4.0**: 实现计划（20 个任务）
- **v0.4.0**: README 重写（Adabraka 品牌、平台矩阵）
- **v0.5.0**: 桌面平台功能设计（15 个新功能）
- **v0.6.1**: 同步文档和批次报告

## Zed 上游同步

### 批次 1 (v0.6.1)
- **Commit**: 6276daa
- 锚定元素大小计算修复（b38194198b）
- GIF 渲染越界 panic 修复（749fcfdfd8）

### 批次 2 (v0.6.2)
- **Commit**: eddd037
- 移除 naga 构建依赖（e712f3c6df）
- 窗口失焦时恢复鼠标光标（c01671eac1）

### 批次 3 (v0.6.2)
- **Commit**: dd5cd1c, 99bced5, d8dd10d, c04b417
- SharedString smol_str 优化（58d3a9eef4）
- X11 窗口图标支持（24a304c140）
- macOS 字形膨胀（a38fc8c8de）
- 输入模式跟踪（d010b06a77）

## 版本历史

- **v0.2.1**: 初始导入（来自 crates.io）
- **v0.3.0**: 命名空间重命名 + 渲染增强
- **v0.4.0**: 守护进程模式 + 系统托盘 + 全局热键
- **v0.4.1**: 托盘面板模式 + 菜单图标
- **v0.5.0**: 15 个扩展桌面功能
- **v0.5.1**: 性能优化 + DirectX 改进
- **v0.5.3**: 多显示器修复
- **v0.6.0**: objc2 迁移 + 图像功能 + ksni 升级
- **v0.6.1**: Zed 同步批次 1
- **v0.6.2**: Zed 同步批次 2-3 + 性能优化
- **v0.7.0**: Layer-Shell 显式 API + AppResourceProfile (Minimal/Utility/Desktop) + 水印淘汰算法
- **v0.8.0**: Workspace 8 Crate 解耦拆分 + 权限/捕获状态 + 避免边框 Quad 过绘 + Win32 线程池调度 + Taffy 0.12.2
- **v0.8.1**: DirectX 离屏 Readback 结构 Padding 物理对齐 + 发布语义自动化校验

## 统计

- **总提交数**: 140+
- **主要功能版本**: 9
- **平台支持**: macOS, Linux (X11/Wayland), Windows
- **新增 API**: 60+ 平台方法
- **性能改进**: 8+ 关键优化
- **Bug 修复**: 45+ 修复
