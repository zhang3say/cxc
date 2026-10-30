# CXC 2

CXC 第二代桌面端。与旧版并行开发，沿用旧版的视觉语言，数据模型改为「全局中转站 + 每个 Agent 的绑定目录」。

## 功能

- **Agent 自动识别**：按 PATH 探测 Codex、Claude Code、Grok CLI、OpenCode；未检测到的 Agent 可在「显示全部」中手动配置。
- **全局中转站**：名称、Base URL、API Key、备注、协议集合、模型目录只保存一份。
- **三协议探测**：Anthropic Messages、OpenAI Chat Completions、OpenAI Responses，逐个端点探测并写回。
- **每 Agent 独立绑定**：一个 Agent 可以保存多条绑定，同一来源同时只启用一条；绑定携带模型、推理档位、上下文长度、压缩比例，以及 Claude 多模型映射 / Grok web_search 策略。
- **按模型测连通性**：单模型测试、按中转站批量测试，结果持久化。
- **旧版功能对齐**：App / WSL 来源、自定义目录、`.bak` 备份、Gist 同步、系统托盘、更新检查、中英文与明暗主题。

## 目录

```text
cxc-v2/
  cxc-v2-core/          # 数据模型、协议、探测、适配器、同步、迁移
  cxc-v2-desktop/       # Tauri 2 + React 19 桌面端
```

## 数据

配置文件：Linux / WSL `~/.config/cxc-v2/config.yaml`，Windows `%USERPROFILE%\.config\cxc-v2\config.yaml`。

```yaml
version: 2
source: wsl
suppliers:
  - id: "..."
    name: "openrouter"
    base_url: "https://openrouter.ai/api/v1"
    api_key: "sk-..."
    protocols: [chat_completions, messages]
    models:
      - model: "anthropic/claude-sonnet-4.5"
        protocols: [chat_completions, messages]
agents:
  codex:
    active_wsl: "binding-id"
    bindings:
      - id: "binding-id"
        supplier_id: "..."
        protocol: responses
        model: "openai/gpt-5"
        reasoning_effort: high
        context_length: 200k
        compact_ratio: 80
```

旧版 `~/.config/cxc/config.yaml` 在首次启动时一次性导入；旧文件只读，不会被修改。

## 开发

```bash
cd cxc-v2
cargo test -p cxc-v2-core
cargo check -p cxc-v2-desktop

cd cxc-v2-desktop
npm install
npm run dev          # 浏览器预览（内置 mock IPC）
npm run tauri dev    # 桌面端
npm run build
```

浏览器预览不依赖 Tauri，`src/lib/mock.ts` 会模拟配置、探测、测试与写入，便于用 Playwright 验证界面流程。

## Agent 能力矩阵

| 能力 | Codex | Claude | Grok | OpenCode |
|---|---|---|---|---|
| 协议 | responses / chat_completions | messages | chat_completions / responses | chat_completions / messages；responses 实验 |
| 推理档位 | 普通 + 计划模式 | 全局 `CLAUDE_CODE_EFFORT_LEVEL` | 模型块 | `options.reasoningEffort` |
| 上下文 | `model_context_window` | 仅 `1m` | `context_window` | `limit.context` |
| 压缩比例 | `model_auto_compact_token_limit` | `CLAUDE_CODE_AUTO_COMPACT_WINDOW` | `auto_compact_threshold_percent` | `compaction.reserved` 换算 |
| 多模型 | — | opus / sonnet / haiku / subagent | web_search | — |

## 设计文档

- `docs/adr/0018-global-suppliers-and-agent-bindings.md`
- `docs/adr/0019-three-protocol-probing.md`
- `docs/adr/0020-opencode-adapter.md`
- `docs/adr/0021-cxc-v2-config-and-migration.md`
