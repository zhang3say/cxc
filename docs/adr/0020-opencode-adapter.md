# ADR-0020: OpenCode 适配器

## 背景

OpenCode 通过 `opencode.json` 的 `provider` 和顶层 `model` 选择模型。它使用 AI SDK 的 provider 包，和 Codex 的 TOML、Claude 的 `settings.json` 都不同。

## 决策

新增 OpenCode 适配器，配置文件默认位于：

- Linux / WSL：`~/.config/opencode/opencode.json`
- Windows：`%APPDATA%\opencode\opencode.json`

CXC 管理一个 `provider.cxc-<supplier-id>` 块：

- `chat_completions` → `npm: "@ai-sdk/openai-compatible"`
- `messages` → `npm: "@ai-sdk/anthropic"`
- `responses` → `npm: "@ai-sdk/openai"`，模型 `options.useResponses = true`，标记为实验能力

顶层 `model` 写成 `cxc-<supplier-id>/<model>`。模型参数映射：

- 推理档位 → `models.<model>.options.reasoningEffort`
- 上下文 → `models.<model>.limit.context`
- 压缩比例 → `compaction.auto = true`，`compaction.reserved = context × (1 − ratio)`

写入只覆盖 `provider.cxc-*`、顶层 `model`、`compaction` 三个受管区域，其余键原样保留；写入前生成 `.bak`。

## 后果

- OpenCode 可以和其它 Agent 共用同一批中转站。
- Responses 组合依赖 OpenCode 对 `useResponses` 的支持；验证失败时在能力矩阵中禁用，保留 Chat / Messages。
- OpenCode 的配置文件是 JSON，注释不会保留。

## 状态

已采纳。实现见 `cxc-core/src/target/opencode.rs`。
