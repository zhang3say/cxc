# ADR-0013: Grok CLI 适配器设计

## 状态

已接受 (2026-07-12)

## 背景

CXC 已支持 Codex 与 Claude CLI 作为目标工具。用户在使用 Grok CLI（`~/.grok/config.toml`）时同样需要在多个 API 中转站之间快速切换。

Grok 与现有工具的差异：

| 维度 | Codex | Claude CLI | Grok |
|------|-------|------------|------|
| 配置文件 | `config.toml` + `auth.json` | `settings.json` (`env`) | `config.toml` |
| 模型入口 | 顶层 `model` + `model_providers.codex` | 多个 `ANTHROPIC_*` 环境变量 | `[models].default` + `[model.<id>]` |
| 协议字段 | `wire_api` | 无 | `api_backend` |
| 默认协议 | `responses` | Anthropic Messages | `chat_completions` |
| 密钥位置 | `auth.json` 的 `OPENAI_API_KEY` | `env.ANTHROPIC_AUTH_TOKEN` | `[model.<id>].api_key` |

## 决策

### 1. 配置写入范围

**决定：只改 API 相关字段，保留其余配置**

写入字段：

| 字段 | 位置 | 来源 |
|------|------|------|
| `default` | `[models]` | Provider.`model` |
| `base_url` | `[model."<model>"]` | Provider.`base_url` |
| `api_key` | `[model."<model>"]` | Provider.`api_key` |
| `api_backend` | `[model."<model>"]` | Provider.`wire_api`（映射后） |
| `model` | `[model."<model>"]` | Provider.`model`（与 section key 一致） |

不修改：

- `auth.json` / OAuth 会话
- marketplace、ui、skills、mcp 等无关 section
- 项目级 `.grok/config.toml`

使用 `toml_edit` 保持格式与注释；写前备份 `config.toml.bak`。

### 2. wire_api → api_backend 映射

| Provider.wire_api | Grok `api_backend` |
|-------------------|--------------------|
| 空 / `chat` / `chat_completions` | `chat_completions` |
| `responses` | `responses` |
| `messages` | `messages` |
| 其他 | 原样透传 |

新增 Grok Provider 时默认 `wire_api = chat_completions`（与 Codex 默认 `responses` 不同）。

### 3. 跨环境路径

与 Codex / Claude 一致：

- `grok_source` 纳入全局 App / WSL 同步（`set_global_source`）
- `grok_custom_dir` 可选
- 默认路径：`~/.grok`（WSL 本地）/ Windows 主机或 WSL UNC 映射

目录不存在时：**创建** `~/.grok/` 并写入最小 scaffold（更接近 Codex，而非 Claude 的“必须已安装”策略）。

### 4. CXC 配置结构

```yaml
grok_active_app: ""
grok_active_wsl: ""
grok_providers: []
grok_source: null
grok_custom_dir: ""
```

Active Provider 按 Source 独立（ADR-0012）；Provider 列表在工具内共享。

### 5. 连通性测试

Grok 中转默认 OpenAI 兼容，使用 `Tester::test_openai`（`is_claude = false`）。

### 6. 系统托盘

本轮仍仅展示 Codex Provider 列表（见 CONTEXT.md 已知限制），不扩展托盘多工具切换。

## 后果

- 桌面端 Target Tool 切换器从 2 段变为 3 段（Codex / Claude / Grok）
- `config.rs` 与 Tauri 命令层改为三路 `match`，未知 target_tool 报错
- README / CONTEXT 需同步更新
