# 当前 Zed GPUI 同步状态

> 这是当前唯一可执行的同步说明。其他 `docs/sync` 文档是带日期的历史报告或拆分前指南，只用于追溯当时的判断。

## 当前基线

- 当前开发分支：`develop/0.9`
- 下一版本：GPUI 八个发布包为 `0.9.0`；`adabraka_util` 和 `adabraka_util_macros` 为 `0.6.0`
- 兼容基线：GPUI `0.8.1`、utility crates `0.5.1`；breaking API/feature 变化必须在 `CHANGELOG.md` 提供迁移映射
- 永久兼容入口：`Application::new/headless`；不在任何版本安排弃用或删除
- Registry 发布顺序：`adabraka_util_macros 0.6.0` → `adabraka_util 0.6.0` → `adabraka-gpui-macros 0.9.0` → core/renderer/backends → platform → public facade
- 当前仓库审计基线：`5aa5083184d6692167b905b51a00999d28ad20ab`（2026-07-26）
- Zed 上游审计基线：`ec3d887507f272119d9fe146c685f0a941d0e798`（2026-07-22）
- 上游仓库默认位置：`../zed`
- 公共包：`adabraka-gpui`，lib 名仍为 `gpui`
- 核心实现包：`adabraka-gpui-core`，lib 名为 `gpui_core`

同步新改动前，先更新 `../zed`，再把上游基线改成实际审完的 commit。不要只按日期判断是否已同步。

## 当前结构

```text
adabraka-gpui (public facade) -> adabraka-gpui-core
adabraka-gpui (public facade) -> adabraka-gpui-platform
adabraka-gpui-platform -> adabraka-gpui-core
adabraka-gpui-platform -> adabraka-gpui-macos -> adabraka-gpui-core
adabraka-gpui-platform -> adabraka-gpui-windows -> adabraka-gpui-core
adabraka-gpui-platform -> adabraka-gpui-linux -> adabraka-gpui-core
adabraka-gpui-linux -> adabraka-gpui-wgpu -> adabraka-gpui-core
```

`adabraka-gpui` 负责兼容现有下游入口和 `Application::new/headless`。核心状态、元素、布局、窗口协议和单元测试属于 `adabraka-gpui-core`。平台选择属于 `adabraka-gpui-platform`，具体操作系统实现不再位于 core。

## 上游路径映射

| Zed 路径 | 当前路径 | 说明 |
|---|---|---|
| `crates/gpui` | `crates/gpui` | 核心实现；本地 package 是 `adabraka-gpui-core` |
| 无直接对应 | `crates/gpui-compat` | 公共兼容入口、示例和集成测试 |
| `crates/gpui_platform` | `crates/gpui-platform` | 当前平台选择和应用组装 |
| `crates/gpui_wgpu` | `crates/gpui-wgpu` | Linux WGPU renderer |
| `crates/gpui_linux` | `crates/gpui-linux` | Linux/FreeBSD；主要实现位于 `src/linux` |
| `crates/gpui_macos` | `crates/gpui-macos` | 上游平铺文件对应本地 `src/mac` |
| `crates/gpui_windows` | `crates/gpui-windows` | 上游平铺文件对应本地 `src/windows` |
| `crates/gpui_macros` | `crates/gpui-macros` | proc macros；同时支持正常和重命名依赖 |
| `crates/collections` | `crates/collections` | `adabraka_collections` |
| `crates/util` | `crates/util` | `adabraka_util` |
| `crates/util_macros` | `crates/util_macros` | `adabraka_util_macros` |
| `crates/refineable` | `crates/refineable` | `adabraka_refineable` |
| `crates/sum_tree` | `crates/sum_tree` | `adabraka_sum_tree` |
| `crates/http_client` | `crates/http_client` | `adabraka_http_client` |
| `crates/media` | `crates/media` | `adabraka_media` |

不要直接覆盖 `app.rs`、`window.rs`、`platform.rs` 或平台 backend 文件。这些位置包含 daemon、tray、hotkey、overlay、notification、resource profile 和兼容层所需的本地行为。

## 当前明确分歧

| 主题 | 当前决定 | 重新评估触发条件 |
|---|---|---|
| Scheduler / queue | 保留本地 test-only priority prototype，不迁完整 scheduler | profiling 证明后台任务影响首帧、托盘弹窗或长期运行 |
| Structured system notifications | 延期；保留现有 title/body API | 应用需要 tag 替换、dismiss、action button 或点击回调 |
| Parent-anchored native popup | 延期 | Wayland/native popup 出现可复现的定位、焦点或关闭问题 |
| Web / mobile | 不在当前产品边界 | 项目明确增加非桌面平台目标 |
| View API / container query | 独立评估，不随 bugfix 批次迁移 | 下游组件需要对应能力或上游修复强依赖它们 |
| Adabraka desktop extensions | 保留 | 只有兼容迁移方案和下游弃用计划同时存在时才可删除 |

这些是有意分歧，不应在增量审计中自动定为遗漏。

## 同步流程

1. 更新 `../zed` 并记录准备审计的上游 HEAD。
2. 只列出上一个基线之后、影响映射 crate 的提交。
3. 对每个提交标记：`backport`、`equivalent`、`deferred` 或 `not-applicable`。
4. 一个上游主题对应一个本地提交；backport 提交正文写完整 `Zed-Origin: <hash>`。
5. 先运行改动区域测试，再运行完整迁移验证。
6. 审完所有提交后再更新本文的上游基线。

推荐的增量查询：

```sh
git -C ../zed log --oneline \
  ec3d887507f272119d9fe146c685f0a941d0e798..HEAD -- \
  crates/gpui crates/gpui_platform crates/gpui_wgpu \
  crates/gpui_linux crates/gpui_macos crates/gpui_windows crates/gpui_macros
```

## 有效验证命令

核心 focused test 必须指定 `adabraka-gpui-core`：

```sh
cargo test --locked -p adabraka-gpui-core --lib --features test-support -- profiler
cargo test --locked -p adabraka-gpui-core --lib --features test-support -- elements::list
```

公共兼容入口单独验证：

```sh
cargo test --locked -p adabraka-gpui --tests --features test-support
cargo check --locked -p adabraka-gpui-downstream-compat
cargo check --locked -p adabraka-gpui-renamed-dependency-compat
```

完整本地闭环：

```sh
scripts/verify-migration.sh
```

发布前安装 `cargo-semver-checks` 后执行：

```sh
scripts/verify-migration.sh --semver
```

Linux/Windows 的真实窗口、输入、托盘和 GPU 行为仍由对应平台 CI 或人工 smoke test 判定；交叉编译不能替代运行时证据。

## 历史材料

- `ZED_GPUI_INCREMENTAL_AUDIT_2026-07-01.md`：带日期的增量判断和 backport 记录。
- `ZED_GPUI_P1_BACKPORT_2026-07-20.md`：P1 批次及当时的平台验证证据。
- `ZED_SYNC_MAPPING.md`、`QUICK_SYNC_REFERENCE.md`、`TECHNICAL_SYNC_GUIDE.md`：拆分前指南，仅用于历史追溯。

更新历史材料中的旧命令不会改变当时的事实，因此不要批量重写；当前执行以本文为准。
