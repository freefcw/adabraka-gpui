# 第二批同步报告

**日期**: 2026-04-30  
**分支**: `sync/zed-rendering-improvements-2026-04-30`  
**状态**: ⏸️ 需要更多工作

## 评估结果

经过详细评估，第二批的所有改进都需要较大的架构变更或依赖 Zed 特有的基础设施，不适合直接合并。

### 1. ❌ macOS 字形膨胀优化 (`a38fc8c8de`)

**问题**:
- 需要在 `PlatformTextSystem` trait 添加新方法 `glyph_dilation_for_color`
- 需要修改文本渲染管线以支持基于亮度的字形缓存
- 涉及 4 个文件的协调修改
- Zed 使用独立的 `gpui_macos` crate，而 Adabraka 是内联结构

**影响范围**:
- `crates/gpui/src/platform.rs` - trait 定义
- `crates/gpui/src/text_system.rs` - 核心文本系统
- `crates/gpui/src/window.rs` - 窗口渲染
- `crates/gpui/src/platform/mac/text_system.rs` - macOS 实现

**建议**: 
- 作为独立功能在未来版本实现
- 需要完整的设计和测试
- 预计工作量：2-3 天

---

### 2. ❌ macOS 光标闪烁修复 (`d010b06a77`)

**问题**:
- 依赖 `last_input_was_keyboard()` 功能，Adabraka 中不存在
- 需要实现输入模式跟踪机制
- 涉及 `HitboxId` 和光标处理的核心逻辑重构

**影响范围**:
- `crates/gpui/src/window.rs` - 多处修改
- `crates/gpui/src/platform/mac/window.rs` - macOS 特定实现
- `crates/gpui/src/platform/mac/platform.rs` - 平台层

**建议**:
- 需要先实现输入模式跟踪
- 预计工作量：1-2 天

---

### 3. ❌ SharedString 优化 (`58d3a9eef4`)

**问题**:
- Zed 使用独立的 `gpui_shared_string` crate
- Adabraka 的 SharedString 是内联在 `shared_string.rs` 中
- 需要引入 `smol_str` 依赖并重构整个 SharedString 实现

**影响范围**:
- `crates/gpui/src/shared_string.rs` - 完全重写
- 所有使用 SharedString 的代码需要测试

**建议**:
- 作为性能优化在未来版本实现
- 需要全面的性能测试
- 预计工作量：1 天

---

### 4. ❌ X11 窗口图标支持 (`24a304c140`)

**问题**:
- 需要在 `WindowParams` 添加 `icon` 字段
- 需要修改所有创建窗口的代码
- 涉及 Platform trait 的 API 变更

**影响范围**:
- `crates/gpui/src/platform.rs` - API 变更
- `crates/gpui/src/window.rs` - WindowParams 修改
- `crates/gpui/src/platform/linux/x11/window.rs` - X11 实现
- 所有调用 `open_window` 的代码

**建议**:
- 作为功能增强在未来版本实现
- 需要设计统一的窗口图标 API
- 预计工作量：1 天

---

## 总结

### 为什么第二批无法直接合并

1. **架构差异**: Zed 使用多 crate 架构（`gpui_macos`, `gpui_shared_string`），Adabraka 是单 crate
2. **功能依赖**: 多个修复依赖 Zed 特有的基础设施（输入模式跟踪等）
3. **API 变更**: 需要修改公共 API，影响范围大
4. **测试需求**: 每个改动都需要全面的跨平台测试

### 建议的实施策略

#### 短期（1-2 周）
- 跳过第二批，直接评估第三批中更简单的修复
- 专注于独立的 bug 修复，避免架构变更

#### 中期（1-2 月）
- 设计 Adabraka 的文本渲染改进方案
- 实现输入模式跟踪基础设施
- 逐步引入性能优化

#### 长期（3-6 月）
- 考虑重构为多 crate 架构（可选）
- 实现完整的字形膨胀优化
- 统一窗口图标 API

---

## 下一步行动

### 建议：评估第三批中的简单修复

从 `ZED_CHANGES_2024-2026.md` 中的其他修复中选择：

**候选修复**（独立且简单）:
1. SVG 渲染修复（多个 commits）
2. 文本渲染修复（下划线、删除线等）
3. 移除 naga 依赖
4. 鼠标光标恢复（窗口失活时）

这些修复：
- ✅ 不需要架构变更
- ✅ 不依赖 Zed 特有功能
- ✅ 可以独立测试
- ✅ 风险低

---

**评估完成**: 2026-04-30 19:15  
**建议**: 跳过第二批，评估其他简单修复
