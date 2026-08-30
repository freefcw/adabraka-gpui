# Zed → Adabraka GPUI 快速同步参考

> **历史文档**：本文命令和平台路径基于 crate 拆分前结构。当前同步只使用 [`CURRENT.md`](./CURRENT.md)。

## 快速映射表

### Crate 名称映射

| Zed | Adabraka | 包名 |
|-----|----------|------|
| `gpui` | `gpui` | `fc-gpui` |
| `gpui_macros` | `gpui-macros` | `fc-gpui-macros` |
| `collections` | `collections` | `fc-gpui-collections` |
| `util` | `util` | `fc-gpui-util` |
| `util_macros` | `util_macros` | `fc-gpui-util-macros` |
| `refineable` | `refineable` | `fc-gpui-refineable` |
| `derive_refineable` | `derive_refineable` | `fc-gpui-derive-refineable` |
| `sum_tree` | `sum_tree` | `fc-gpui-sum-tree` |
| `semantic_version` | `semantic_version` | `fc-gpui-semantic-version` |
| `http_client` | `http_client` | `fc-gpui-http-client` |
| `media` | `media` | `fc-gpui-media` |
| `perf` | `perf` | `fc-gpui-perf` |

### 文件同步优先级

#### 🟢 安全同步（直接复制）
这些文件不包含 Adabraka 扩展，可以直接从 Zed 复制：

```
crates/gpui/src/
├── scene.rs
├── element.rs
├── geometry.rs
├── colors.rs
├── path_builder.rs
├── shared_string.rs
├── shared_uri.rs
├── arena.rs
├── bounds_tree.rs
├── asset_cache.rs
├── assets.rs
├── subscription.rs
├── global.rs
├── tab_stop.rs
├── svg_renderer.rs
├── test.rs
├── inspector.rs
├── taffy.rs
├── view.rs
├── prelude.rs
├── gpui.rs
├── elements/
│   ├── canvas.rs
│   ├── div.rs
│   ├── img.rs
│   ├── svg.rs
│   ├── text.rs
│   ├── uniform_list.rs
│   └── ...
├── text_system/
│   └── ...
├── keymap/
│   └── ...
└── window/
    └── ...
```

#### 🟡 谨慎同步（需要合并）
这些文件包含 Adabraka 扩展，需要手动合并：

```
crates/gpui/src/
├── app.rs                    # 包含守护进程模式、托盘、热键等扩展
├── window.rs                 # 包含覆盖窗口、点击穿透等扩展
├── platform.rs               # 包含 Adabraka 扩展的 trait 方法
├── style.rs                  # 可能包含样式扩展
├── styled.rs                 # 可能包含样式扩展
├── color.rs                  # 可能包含颜色扩展
├── interactive.rs            # 可能包含交互扩展
├── input.rs                  # 可能包含输入扩展
├── keymap.rs                 # 可能包含键盘映射扩展
├── action.rs                 # 可能包含动作扩展
├── key_dispatch.rs           # 可能包含键盘分发扩展
├── executor.rs               # 可能包含执行器扩展
└── text_system.rs            # 可能包含文本系统扩展
```

#### 🔴 不要同步（Adabraka 独有）
这些文件是 Adabraka 独有的，不存在于 Zed：

```
crates/gpui/src/platform/
├── mac/
│   ├── tray.rs
│   ├── global_hotkey.rs
│   ├── notification.rs
│   ├── autolaunch.rs
│   ├── single_instance.rs
│   ├── focused_window.rs
│   └── permissions.rs
├── linux/
│   ├── tray.rs
│   ├── global_hotkey.rs
│   ├── notification.rs
│   ├── autolaunch.rs
│   └── single_instance.rs
└── windows/
    ├── tray.rs
    ├── global_hotkey.rs
    ├── notification.rs
    ├── autolaunch.rs
    └── single_instance.rs
```

### 平台实现同步

#### macOS
```
crates/gpui/src/platform/mac/
├── platform.rs               # 🟡 谨慎同步
├── window.rs                 # 🟡 谨慎同步
├── metal_renderer.rs         # 🟢 安全同步
├── text_system.rs            # 🟢 安全同步
├── display.rs                # 🟢 安全同步
├── events.rs                 # 🟢 安全同步
├── tray.rs                   # 🔴 不要同步
├── global_hotkey.rs          # 🔴 不要同步
├── notification.rs           # 🔴 不要同步
├── autolaunch.rs             # 🔴 不要同步
├── single_instance.rs        # 🔴 不要同步
├── focused_window.rs         # 🔴 不要同步
└── permissions.rs            # 🔴 不要同步
```

#### Linux
```
crates/gpui/src/platform/linux/
├── platform.rs               # 🟡 谨慎同步
├── window.rs                 # 🟡 谨慎同步
├── wayland/                  # 🟢 安全同步
├── x11/                      # 🟢 安全同步
├── tray.rs                   # 🔴 不要同步
├── global_hotkey.rs          # 🔴 不要同步
├── notification.rs           # 🔴 不要同步
├── autolaunch.rs             # 🔴 不要同步
└── single_instance.rs        # 🔴 不要同步
```

#### Windows
```
crates/gpui/src/platform/windows/
├── platform.rs               # 🟡 谨慎同步
├── window.rs                 # 🟡 谨慎同步
├── directx_renderer.rs       # 🟢 安全同步
├── text_system.rs            # 🟢 安全同步
├── tray.rs                   # 🔴 不要同步
├── global_hotkey.rs          # 🔴 不要同步
├── notification.rs           # 🔴 不要同步
├── autolaunch.rs             # 🔴 不要同步
└── single_instance.rs        # 🔴 不要同步
```

## 常用命令

### 检查 Zed 更新
```bash
cd /Users/hejun/work/my/zed
git fetch origin
git log --oneline origin/main --since="2024-04-01" -- crates/gpui
```

### 对比单个文件
```bash
diff -u \
  /Users/hejun/work/my/zed/crates/gpui/src/scene.rs \
  /Users/hejun/work/my/adabraka-gpui/crates/gpui/src/scene.rs
```

### 对比整个目录
```bash
diff -ur \
  /Users/hejun/work/my/zed/crates/gpui/src/elements \
  /Users/hejun/work/my/adabraka-gpui/crates/gpui/src/elements
```

### 创建同步分支
```bash
cd /Users/hejun/work/my/adabraka-gpui
git checkout -b sync/zed-$(date +%Y-%m-%d)
```

### 复制安全文件
```bash
# 单个文件
cp /Users/hejun/work/my/zed/crates/gpui/src/scene.rs \
   /Users/hejun/work/my/adabraka-gpui/crates/gpui/src/scene.rs

# 整个目录
cp -r /Users/hejun/work/my/zed/crates/gpui/src/elements \
      /Users/hejun/work/my/adabraka-gpui/crates/gpui/src/
```

### 运行测试
```bash
cd /Users/hejun/work/my/adabraka-gpui

# 所有测试
cargo test --all

# 特定 crate
cargo test -p fc-gpui

# 特定平台
cargo test --features "x11"
cargo test --features "wayland"
```

### 构建示例
```bash
cd /Users/hejun/work/my/adabraka-gpui

# 所有示例
cargo build --examples

# 特定示例
cargo run --example daemon_app
cargo run --example hello_world
```

## 同步工作流

### 标准同步流程

```bash
# 1. 准备
cd /Users/hejun/work/my/adabraka-gpui
git checkout main
git pull
git checkout -b sync/zed-$(date +%Y-%m-%d)

# 2. 检查 Zed 更新
cd /Users/hejun/work/my/zed
git pull
git log --oneline --since="<last-sync-date>" -- crates/gpui

# 3. 同步安全文件（示例）
cd /Users/hejun/work/my/adabraka-gpui
cp /Users/hejun/work/my/zed/crates/gpui/src/scene.rs \
   crates/gpui/src/scene.rs

# 4. 测试
cargo test -p fc-gpui

# 5. 提交
git add .
git commit -m "sync: update scene.rs from zed <commit-hash>"

# 6. 推送并创建 PR
git push origin sync/zed-$(date +%Y-%m-%d)
```

### 合并冲突文件流程

```bash
# 1. 备份 Adabraka 版本
cd /Users/hejun/work/my/adabraka-gpui
cp crates/gpui/src/app.rs /tmp/app.rs.adabraka

# 2. 复制 Zed 版本
cp /Users/hejun/work/my/zed/crates/gpui/src/app.rs \
   crates/gpui/src/app.rs

# 3. 手动合并 Adabraka 扩展
# 编辑 crates/gpui/src/app.rs
# 添加 Adabraka 扩展方法

# 4. 对比确认
diff /tmp/app.rs.adabraka crates/gpui/src/app.rs

# 5. 测试
cargo test -p fc-gpui

# 6. 提交
git add crates/gpui/src/app.rs
git commit -m "sync: merge app.rs from zed <commit-hash>, preserve adabraka extensions"
```

## 依赖更新

### 检查依赖差异
```bash
diff \
  /Users/hejun/work/my/zed/crates/gpui/Cargo.toml \
  /Users/hejun/work/my/adabraka-gpui/crates/gpui/Cargo.toml
```

### 常见依赖

| 依赖 | 用途 | 同步策略 |
|-----|------|---------|
| `wgpu` | GPU 渲染 | 跟随 Zed |
| `winit` | 窗口管理 | 跟随 Zed |
| `taffy` | 布局引擎 | 跟随 Zed |
| `cosmic-text` | 文本渲染 | 跟随 Zed |
| `image` | 图像解码 | 跟随 Zed，但保持特性配置 |
| `resvg` | SVG 渲染 | 跟随 Zed |
| `ksni` | Linux 托盘 | Adabraka 独有 |
| `tray-icon` | 跨平台托盘 | Adabraka 独有 |
| `global-hotkey` | 全局热键 | Adabraka 独有 |
| `notify-rust` | Linux 通知 | Adabraka 独有 |

## 版本历史

| 日期 | Adabraka 版本 | Zed Commit | 同步内容 |
|------|--------------|------------|---------|
| 2024-04-30 | 0.6.0 | TBD | 初始映射文档 |
| 2024-04-21 | 0.6.0 | TBD | 图像格式特性 |
| 2024-04-20 | 0.5.1 | TBD | 性能优化 |
| 2024-04-11 | 0.5.0 | TBD | 初始发布 |

## 联系方式

- **问题报告**: https://github.com/Augani/adabraka-gpui/issues
- **讨论**: https://github.com/Augani/adabraka-gpui/discussions
- **Zed 上游**: https://github.com/zed-industries/zed

---

**提示**: 
- 🟢 = 安全操作
- 🟡 = 需要注意
- 🔴 = 危险操作

**最后更新**: 2026-04-30
