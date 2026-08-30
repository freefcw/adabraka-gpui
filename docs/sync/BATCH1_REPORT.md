# 第一批同步完成报告

**日期**: 2026-04-30  
**分支**: `sync/zed-critical-fixes-2026-04-30`  
**状态**: ✅ 完成

## 已应用的修复

### 1. ✅ Anchored 元素尺寸计算修复
**Zed Commit**: `b38194198b`  
**文件**: `crates/gpui/src/elements/anchored.rs`

**问题**: 
- 当子元素边界在负坐标时，手动 min/max 累积会导致最大点被钳制到 (0,0)
- 这会导致计算的尺寸膨胀，影响上下文菜单和弹出窗口的定位

**修复**:
```rust
// 之前：手动 min/max 累积
let mut child_min = point(Pixels::MAX, Pixels::MAX);
let mut child_max = Point::default();
for child_layout_id in &request_layout.child_layout_ids {
    let child_bounds = window.layout_bounds(*child_layout_id);
    child_min = child_min.min(&child_bounds.origin);
    child_max = child_max.max(&child_bounds.bottom_right());
}
let size: Size<Pixels> = (child_max - child_min).into();

// 之后：使用 Bounds::union
let children_bounds = request_layout
    .child_layout_ids
    .iter()
    .map(|id| window.layout_bounds(*id))
    .reduce(|acc, bounds| acc.union(&bounds))
    .unwrap();
let size = children_bounds.size;
```

**影响**: 
- 修复上下文菜单在滚动表面中的错误位置
- 改善所有锚定元素的定位准确性

---

### 2. ✅ GIF 渲染越界 Panic 修复
**Zed Commit**: `749fcfdfd8`  
**文件**: `crates/gpui/src/elements/img.rs`

**问题**:
- 当 GIF 被替换为帧数更少的 GIF 时，缓存的 `frame_index` 可能越界
- 导致 panic 和应用崩溃

**修复**:
```rust
// 添加帧数检查和索引钳制
let frame_count = data.frame_count();
let max_frame_index = frame_count.saturating_sub(1);

if let Some(state) = &mut state {
    state.frame_index = state.frame_index.min(max_frame_index);
    // ... 其他逻辑
    frame_index = state.frame_index;
}
```

**影响**:
- 防止 Markdown 预览中 GIF 替换时的崩溃
- 提高图像查看器的稳定性

---

## 未应用的修复（需要额外工作）

### 3. ⏸️ Wayland 后台窗口冻结修复
**Zed Commit**: `10122be9cb`  
**原因**: Adabraka 代码中尚未实现帧率限制功能，缺少 `last_frame_time` 相关代码

**建议**: 在实现帧率限制功能时一并应用

---

### 4. ⏸️ 后台窗口帧率限制
**Zed Commit**: `72eb842540`  
**原因**: 需要实现完整的帧率限制机制，包括：
- 热状态检测
- 窗口激活状态跟踪
- 帧时间记录

**建议**: 作为独立功能在第二批或第三批中实现

---

## 测试结果

### 编译检查
```bash
cargo check -p fc-gpui
```
**结果**: ✅ 通过（无新增错误或警告）

### 现有问题
- 存在一些预先存在的编译警告（与本次修复无关）
- 测试编译失败是由于现有的 `NSRange` 导入问题

---

## 提交信息

```
sync: apply critical bug fixes from zed

- Fix anchored element size calculation with negative coordinates (b38194198b)
  Replaces manual min/max accumulation with Bounds::union to correctly
  compute child bounding box regardless of coordinate sign.
  
- Fix out-of-bounds panic rendering GIFs (749fcfdfd8)
  Clamp cached frame_index when image changes to prevent stale index
  from causing panic when GIF is replaced with one with fewer frames.

These fixes improve stability for context menus, popups, and image viewers.
```

---

## 下一步

### 立即行动
1. ✅ 创建 PR 合并到 main
2. ✅ 更新 CHANGELOG.md
3. ✅ 标记为 patch 版本更新 (0.6.1)

### 第二批准备（建议下周）
1. macOS 字形膨胀优化 (`a38fc8c8de`)
2. macOS 光标闪烁修复 (`d010b06a77`)
3. SharedString 优化 (`58d3a9eef4`)
4. X11 窗口图标支持 (`24a304c140`)

### 第三批评估（需要详细测试）
1. 像素对齐 (`7d42f276f2`)
2. 性能调整合集 (`a62ae579ab`)
3. 帧率限制功能（包括 Wayland 修复）

---

## 风险评估

**风险等级**: 🟢 低

**理由**:
- 两个修复都是独立的 bug 修复
- 不修改核心架构或 API
- 不影响 Adabraka 独有功能
- 编译通过，无新增警告

**建议**: 可以安全合并到 main 分支

---

**完成时间**: 2026-04-30 17:10  
**执行者**: Kiro AI Assistant  
**审核者**: 待定
