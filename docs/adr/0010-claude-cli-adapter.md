# ADR-0010: Claude CLI 适配器设计

## 状态

已接受 (2026-06-15)

## 背景

CXC 当前仅支持 Codex 作为目标工具。用户需要支持 Claude CLI 的配置切换，以便：

1. 在同一个工具中管理多个 AI 编码工具的 API 中转配置
2. 支持 Claude CLI 在 WSL 和 Windows 之间的跨环境配置
3. 支持第三方模型（如 DeepSeek、GLM）通过 Claude CLI 使用

Claude CLI 与 Codex 的主要差异：
- **配置文件**：Claude 使用单一 `settings.json`（JSON 格式），Codex 使用 `config.toml` + `auth.json`
- **配置方式**：Claude 通过 `env` 对象设置环境变量，Codex 直接修改配置字段
- **模型配置**：Claude 有四个独立的模型字段（Opus/Sonnet/Haiku/Fable），Codex 只有一个 `model` 字段
- **认证方式**：Claude 支持 `ANTHROPIC_API_KEY` 和 `ANTHROPIC_AUTH_TOKEN` 两种认证字段

## 决策

### 1. 配置方式

**决定：只支持 `settings.json` 的 `env` 对象方式**

- 修改 `~/.claude/settings.json` 中的 `env` 字段
- 不支持 `apiKeyHelper` 脚本或系统环境变量文件
- 理由：这是官方推荐方式，优先级明确，实现简单

### 2. 多模型配置

**决定：扩展 Provider 结构，支持 `claude_models` 嵌套配置**

```yaml
providers:
  - name: my-deepseek
    model: deepseek-v4-pro  # 主模型（用于 Codex，必填）
    claude_models:           # Claude 专用配置（可选）
      opus: deepseek-v4-pro[1m]
      sonnet: deepseek-v4-pro[1m]
      haiku: deepseek-v4-flash
      fable: deepseek-v4-pro[1m]
```

写入逻辑：
- 如果配置了 `claude_models`，使用其中的值
- 否则回退到 `model` 字段（所有四个模型字段使用同一值）
- 这样支持 DeepSeek 官方推荐的配置方式（pro 对应高级模型，flash 对应快速模型）

理由：
- 支持第三方模型的精细配置（如 DeepSeek v4-pro/flash 映射）
- 向后兼容：不填 `claude_models` 时，全部使用 `model` 字段
- 结构清晰：明确标识这是 Claude 特定配置

### 3. 跨环境配置支持

**决定：支持 `claude_source` 和 `claude_custom_dir` 配置**

类似 Codex 的设计：
- `claude_source: "wsl"` - 使用 WSL 本地配置（Linux 默认）
- `claude_source: "app"` - 使用 Windows 主机配置（或从 Windows 访问 WSL 配置）
- `claude_custom_dir` - 自定义配置路径（可选）

路径映射：
- WSL + `source=wsl` → `~/.claude/`
- WSL + `source=app` → `/mnt/c/Users/<user>/.claude/`
- Windows + `source=app` → `%USERPROFILE%\.claude\`
- Windows + `source=wsl` → `\\wsl.localhost\<distro>\home\<user>\.claude\`

理由：
- 用户在 WSL 和 Windows 都使用 Claude CLI，需要同步配置
- VS Code Extension 在 Windows 上运行，需要从 WSL 修改 Windows 配置
- 与 Codex 适配器保持一致的设计

### 4. 字段保留策略

**决定：只修改 API 相关字段，保留所有其他配置**

修改的字段：
- `env.ANTHROPIC_BASE_URL`
- `env.ANTHROPIC_AUTH_TOKEN` 或 `env.ANTHROPIC_API_KEY`（二选一）
- `env.ANTHROPIC_DEFAULT_OPUS_MODEL`
- `env.ANTHROPIC_DEFAULT_SONNET_MODEL`
- `env.ANTHROPIC_DEFAULT_HAIKU_MODEL`
- `env.ANTHROPIC_DEFAULT_FABLE_MODEL`

保留的字段：
- `env` 中的其他字段（如 `CLAUDE_CODE_EFFORT_LEVEL`, `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`）
- 所有非 `env` 的顶层字段（如 `permissions`, `statusLine`, `enabledPlugins`）

认证字段处理：
- 保持用户已有的认证字段名（`API_KEY` 或 `AUTH_TOKEN`）
- 如果都不存在，默认使用 `ANTHROPIC_AUTH_TOKEN`
- 清理冲突：只保留一个认证字段，删除另一个

理由：
- 尊重用户的其他配置（如 effort level、插件、权限等）
- 避免破坏用户手动配置的高级功能
- 与 Codex 适配器保持一致（保留无关字段）

### 5. `wire_api` 字段处理

**决定：Claude 适配器忽略 `wire_api` 字段**

- Provider 结构保留 `wire_api` 字段（向后兼容）
- Claude 适配器的 `write()` 方法不使用此字段
- 支持同一 Provider 用于 Codex 和 Claude（如果网关兼容两种格式）

理由：
- `wire_api` 是 Codex 特有配置（`responses` vs `chunks`）
- Claude CLI 没有类似概念
- 保持数据结构简单，避免破坏向后兼容性

### 6. 备份策略

**决定：创建 `.bak` 备份文件，与 Codex 保持一致**

- 写入前创建 `settings.json.bak`
- 每次覆盖同一个备份文件
- 备份位置：与原文件同目录（跨环境时也在原文件目录）

理由：
- 与 Codex 适配器保持一致
- 简单可靠，用户知道在哪里找备份
- 对配置切换场景已足够（用户可以立即发现问题并恢复）

### 7. 配置文件初始化

**决定：检查目录存在性，不存在则报错**

- 检查 `~/.claude/` 目录是否存在
- 目录存在 → 创建/更新 `settings.json`，自动添加 `env` 对象（如果不存在）
- 目录不存在 → 报错（Claude CLI 未安装）

理由：
- 避免在未安装 Claude CLI 的环境创建配置
- 配置应该由官方工具初始化
- 安全优先，不污染用户系统

### 8. 错误处理

**决定：提供详细的错误信息和建议**

错误信息包含：
- 具体的错误原因
- 可能的问题原因列表
- 可操作的建议

示例：
```
Error: Claude CLI 配置目录不存在：/mnt/c/Users/leezi/.claude

可能原因：
- Windows 主机上未安装 Claude CLI
- claude_source 配置错误

建议：
- 在 Windows 上安装 Claude CLI
- 或修改 CXC 配置：claude_source = "wsl"
```

理由：
- 友好的错误信息减少用户困惑
- 跨环境配置场景复杂，需要清晰的指引
- 提高用户体验

### 9. 配置读取

**决定：返回主模型，内部处理多模型**

`read()` 方法：
- 返回一个代表性的模型（优先级：OPUS → SONNET → FABLE → HAIKU）
- 认证字段优先返回 `ANTHROPIC_API_KEY`，不存在则返回 `ANTHROPIC_AUTH_TOKEN`
- `wire_api` 返回空字符串

理由：
- 保持 `TargetConfig` 简单，不需要修改核心结构
- `read()` 主要用于 UI 显示，显示一个代表性模型即可
- 完整的多模型配置存储在 Provider 的 `claude_models` 字段中

## 影响

### 代码变更

1. **数据结构扩展**（`cxc-core/src/config.rs`）
   - 新增 `ClaudeModels` 结构
   - Provider 添加 `claude_models: Option<ClaudeModels>` 字段
   - Config 添加 `claude_source` 和 `claude_custom_dir` 字段

2. **新增适配器**（`cxc-core/src/target/claude.rs`）
   - 实现 `ClaudeAdapter` 结构
   - 实现 `TargetAdapter` trait
   - 实现路径检测和转换逻辑
   - 实现 JSON 配置读写逻辑

3. **错误类型扩展**（`cxc-core/src/target/mod.rs`）
   - 新增 `ClaudeConfigDirNotFound` 错误
   - 新增 `ClaudeConfigInvalid` 错误

4. **UI 扩展**（`cxc-desktop/src/App.tsx`）
   - 添加 Claude 高级配置（四个模型字段）
   - 添加 Claude 设置区域（source, custom_dir）
   - 根据目标工具动态显示/隐藏字段

### 向后兼容性

- ✅ 现有 Provider 配置无需修改（`claude_models` 是可选字段）
- ✅ 核心数据结构保持兼容（只增加可选字段）
- ✅ Codex 适配器不受影响
- ✅ `wire_api` 字段保留，Codex 继续使用

### 用户体验

- ✅ 支持 Claude CLI 配置切换
- ✅ 支持 WSL/Windows 跨环境配置
- ✅ 支持第三方模型的精细配置
- ✅ 友好的错误提示
- ✅ 自动备份，安全可靠

## 参考资料

- [Claude Code Environment Variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code Settings](https://code.claude.com/docs/en/settings)
- [Claude Code LLM Gateway Configuration](https://code.claude.com/docs/en/llm-gateway)
- [DeepSeek Claude Code 接入文档](https://platform.deepseek.com/api-docs/zh-cn/claude-code/)
- ADR-0008: Windows/WSL Codex Source（本项目）

## 备注

- 本 ADR 基于 2026-06-15 的设计讨论
- MVP 阶段只支持标准的 Anthropic Messages API 格式
- 未来可能扩展支持 Bedrock/Vertex 格式（通过新的配置字段）
