# Zed 同步文档索引

本目录包含 Adabraka GPUI 与 Zed 源仓库同步的完整文档。

## 文档列表

### 1. [ZED_SYNC_MAPPING.md](./ZED_SYNC_MAPPING.md)
**主要映射文档** - 完整的仓库结构和 crate 映射关系

**内容**:
- 仓库信息和提取策略
- Crate 映射关系表
- 目录结构映射
- 关键文件映射
- Adabraka 独有功能列表
- 同步工作流程
- 冲突解决策略
- 重要注意事项

**适用场景**: 
- 首次了解两个仓库的关系
- 规划同步策略
- 理解 Adabraka 扩展

### 2. [TECHNICAL_SYNC_GUIDE.md](./TECHNICAL_SYNC_GUIDE.md)
**技术同步指南** - 详细的代码级别同步指南

**内容**:
- 代码结构对比
- API 映射表
- 同步检查清单
- 常见同步场景
- 代码合并模式
- 版本兼容性矩阵
- 自动化工具脚本
- 故障排除

**适用场景**:
- 执行实际同步操作
- 解决合并冲突
- 编写同步脚本
- 调试同步问题

### 3. [QUICK_SYNC_REFERENCE.md](./QUICK_SYNC_REFERENCE.md)
**快速参考表** - 常用操作的速查手册

**内容**:
- 快速映射表
- 文件同步优先级（🟢🟡🔴）
- 平台实现同步指南
- 常用命令
- 同步工作流
- 依赖更新指南
- 版本历史

**适用场景**:
- 快速查找文件映射
- 执行常规同步操作
- 检查文件是否可以安全同步

## 使用指南

### 新手入门

1. **首次阅读**: 从 [ZED_SYNC_MAPPING.md](./ZED_SYNC_MAPPING.md) 开始
2. **理解结构**: 了解两个仓库的关系和 Adabraka 的扩展
3. **学习流程**: 阅读同步工作流程部分

### 执行同步

1. **查看快速参考**: [QUICK_SYNC_REFERENCE.md](./QUICK_SYNC_REFERENCE.md)
2. **确定文件优先级**: 使用颜色标记（🟢🟡🔴）
3. **执行同步**: 按照工作流程操作
4. **遇到问题**: 查看 [TECHNICAL_SYNC_GUIDE.md](./TECHNICAL_SYNC_GUIDE.md) 的故障排除部分

### 解决冲突

1. **识别冲突类型**: 参考 [TECHNICAL_SYNC_GUIDE.md](./TECHNICAL_SYNC_GUIDE.md) 的同步场景
2. **选择合并模式**: 直接替换、三方合并或手动合并
3. **保护 Adabraka 扩展**: 使用检查清单确保不丢失功能
4. **测试验证**: 运行完整测试套件

## 文件优先级说明

### 🟢 安全同步
- 可以直接从 Zed 复制
- 不包含 Adabraka 扩展
- 低风险操作

**示例**: `scene.rs`, `element.rs`, `geometry.rs`

### 🟡 谨慎同步
- 需要手动合并
- 包含 Adabraka 扩展
- 中等风险操作

**示例**: `app.rs`, `window.rs`, `platform.rs`

### 🔴 不要同步
- Adabraka 独有文件
- 不存在于 Zed
- 高风险操作

**示例**: `tray.rs`, `global_hotkey.rs`, `notification.rs`

## 同步频率建议

- **安全文件**: 每月检查一次
- **谨慎文件**: 每季度检查一次
- **依赖更新**: 每季度检查一次
- **安全修复**: 立即同步

## 工具和命令模板

### 差异检测

当前仓库没有提供可直接执行的 `scripts/check-zed-diff.sh`。可以参考
[TECHNICAL_SYNC_GUIDE.md](./TECHNICAL_SYNC_GUIDE.md) 中的脚本模板，或直接运行：

```bash
diff -u \
  /path/to/zed/crates/gpui/src/scene.rs \
  crates/gpui/src/scene.rs
```

### 自动同步

当前仓库没有提供可直接执行的 `scripts/sync-from-zed.sh`。可以参考
[TECHNICAL_SYNC_GUIDE.md](./TECHNICAL_SYNC_GUIDE.md) 中的脚本模板，或按需复制单个安全文件：

```bash
cp /path/to/zed/crates/gpui/src/scene.rs \
   crates/gpui/src/scene.rs
```

### 测试验证
```bash
# 运行完整测试
cargo test --all

# 运行平台特定测试
cargo test --features "x11"
cargo test --features "wayland"
```

## 版本对应关系

| Adabraka GPUI | 基于 Zed GPUI | 状态 |
|---------------|---------------|------|
| 0.6.0 | 0.2.2 | 当前 |
| 0.5.1 | 0.2.1 | 稳定 |
| 0.5.0 | 0.2.1 | 稳定 |

## 贡献指南

### 更新文档

当发现新的同步模式或问题时：

1. 更新相应的文档
2. 添加到版本历史
3. 提交 PR

### 添加工具

当创建新的同步工具时：

1. 添加到 `scripts/` 目录
2. 在文档中记录使用方法
3. 添加示例

## 相关资源

- **Zed 仓库**: https://github.com/zed-industries/zed
- **Adabraka GPUI 仓库**: https://github.com/Augani/adabraka-gpui
- **GPUI 文档**: https://gpui.rs
- **问题跟踪**: https://github.com/Augani/adabraka-gpui/issues

## 联系方式

- **问题报告**: [GitHub Issues](https://github.com/Augani/adabraka-gpui/issues)
- **功能讨论**: [GitHub Discussions](https://github.com/Augani/adabraka-gpui/discussions)

---

**维护者**: Adabraka Team  
**最后更新**: 2026-04-30
