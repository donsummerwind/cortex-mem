# Cortex Mem 工具集

这是一个用于管理 `cortex-mem` Rust workspace 项目的工具集，包含版本更新和 crates.io 发布功能。

## 功能

### 1. 版本更新工具 (`update-versions.js`)

- 从 workspace `Cargo.toml` 读取版本号（`[workspace.package].version`）
- 扫描项目中所有的 `Cargo.toml` 文件
- 更新每个 crate 的版本号（支持 `version.workspace = true` 和硬编码版本）
- 自动更新内部依赖引用的版本号
- 排除 `target`、`node_modules`、`.git` 和 `cortex-mem-insights` 目录

### 2. Crates.io 发布工具 (`publish-crates.js`)

- 按依赖顺序自动发布多个 crate 到 crates.io（排除 `cortex-mem-insights` web 项目）
- 自动处理本地路径依赖（path dependencies）
- 支持预发布检查（dry-run）
- 自动等待 crate 在 crates.io 上可用
- 发布后自动恢复原始 Cargo.toml

## 使用方法

### 安装依赖

```bash
cd scripts
npm install
```

### 更新版本号

要更新所有 crate 的版本号：

```bash
npm run update-versions
```

或直接运行：

```bash
node update-versions.js
```

**自定义版本**：编辑根目录 `Cargo.toml` 中的 workspace 版本：

```toml
[workspace.package]
version = "2.5.0"  # 更改为你想要的版本号
```

脚本会自动从这个位置读取版本号。对于使用 `version.workspace = true` 的 crate，无需手动修改。

### 发布到 crates.io

#### 1. 预发布检查（推荐先运行）

```bash
npm run publish-dry-run
```

或：

```bash
node publish-crates.js --dry-run
```

这会检查所有 crate 是否可以发布，但不会实际执行发布操作。

#### 2. 实际发布

```bash
npm run publish-crates
```

或：

```bash
node publish-crates.js
```

#### 3. 跳过等待时间（高级用户）

```bash
node publish-crates.js --skip-wait
```

此选项会跳过等待 crate 在 crates.io 上可用的步骤，适用于你知道 crate 已经可用的情况。

## 发布流程

发布工具会按以下顺序发布 crate（基于依赖关系排序）：

1. **cortex-mem-config** - 基础配置库（无内部依赖）
2. **cortex-mem-core** - 核心引擎（依赖 config）
3. **cortex-mem-tools** - 高层工具（依赖 core）
4. **cortex-mem-rig** - Rig 框架集成（依赖 core, tools）
5. **cortex-mem-service** - HTTP 服务（依赖 core, config）
6. **cortex-mem-cli** - 命令行工具（依赖 core, tools, config）
7. **cortex-mem-mcp** - MCP 服务器（依赖 core, tools, config）
8. **cortex-mem-tars** - TUI 应用（依赖 config, core, tools, rig）

> **注意**：`cortex-mem-insights` 是 Svelte web 项目，不发布到 crates.io。

### 发布步骤

对每个 crate，工具会执行以下操作：

1. 检测是否有本地路径依赖
2. 将路径依赖转换为版本依赖（临时修改 Cargo.toml）
3. 运行 `cargo publish --dry-run` 进行预检查
4. 如果预检查通过，运行 `cargo publish` 发布
5. 等待 crate 在 crates.io 上可用（最多 120 秒）
6. 恢复原始 Cargo.toml

## 发布前准备清单

在发布之前，请确保：

- [ ] 已登录 crates.io：`cargo login`
- [ ] 所有 crate 都有 `description` 和 `license` 字段
- [ ] 所有 crate 都有 `README.md`
- [ ] 版本号符合语义化版本规范（Semantic Versioning）
- [ ] 运行 `cargo test` 确保所有测试通过
- [ ] 运行 `cargo clippy` 检查代码质量
- [ ] 更新 CHANGELOG.md（如果有）

## 示例输出

```
============================================================
Cortex Mem Crates Publishing Tool
============================================================

📦 Crates to publish (in dependency order):
  1. cortex-mem-config v2.5.0
  2. cortex-mem-core v2.5.0
  3. cortex-mem-tools v2.5.0
  4. cortex-mem-rig v2.0.0
  5. cortex-mem-service v2.0.0
  6. cortex-mem-cli v2.5.0
  7. cortex-mem-mcp v2.5.0
  8. cortex-mem-tars v2.0.0

============================================================

⚠️  This will publish the above crates to crates.io
Press Ctrl+C to cancel, or press Enter to continue...

📦 [1/8] Publishing cortex-mem-config v2.5.0
    ⚠️  Found path dependencies, converting for publishing...
    ✓ Dependencies converted
    🔍 Running dry-run check...
    ✓ Dry run passed
    🚀 Publishing to crates.io...
    ✓ cortex-mem-config v2.5.0 published successfully!
    Restored original Cargo.toml

...

============================================================
Publish Summary:
  ✓ 8 crates published successfully
============================================================

🎉 All crates published successfully!
You can now install them with: cargo add cortex-mem-core
```

## 常见问题

### Q: 发布失败怎么办？

A: 检查错误信息，常见原因包括：
- crates.io 上已有相同版本（需要增加版本号）
- 依赖的 crate 还未发布到 crates.io
- Cargo.toml 格式错误

### Q: 如何回滚已发布的版本？

A: crates.io 不支持删除已发布的版本。你需要：
1. 发布一个新版本修复问题
2. 在新版本中标记旧版本为已废弃（使用 `cargo yank`）

### Q: 可以只发布部分 crate 吗？

A: 可以。编辑 `publish-crates.js` 中的 `CRATES_TO_PUBLISH` 数组，只保留需要发布的 crate。

## 注意事项

1. **备份**：脚本会自动备份原始 Cargo.toml 文件，但建议在运行前手动提交到 git
2. **网络**：发布过程需要稳定的网络连接
3. **API Token**：确保已使用 `cargo login` 配置 crates.io API token
4. **等待时间**：每个 crate 发布后需要等待约 1-2 分钟才能在 crates.io 上可用

## 许可证

MIT License - 与 cortex-mem 项目一致
