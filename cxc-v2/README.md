# CXC 2

CXC 第二代桌面端。与旧版并行开发，沿用旧版的视觉语言，数据模型改为「全局中转站 + 每个 Agent 的绑定目录」。

## 功能

- **Agent = 工具 + 环境**：Codex / Claude Code / Grok CLI / OpenCode 各自在 WSL 与 Windows 下是独立 Agent，配置互不影响。
- **Agent 自动识别**：按 PATH 探测每个 `(工具, 环境)`；不可达的环境不显示，未安装的工具仍可手动配置。
- **全局中转站**：名称、Base URL、API Key、备注、协议集合、模型目录只保存一份。
- **三协议探测**：Anthropic Messages、OpenAI Chat Completions、OpenAI Responses，逐个端点探测并写回。
- **绑定列表**：每个 Agent 可以保存多条绑定（中转站 + 模型 + 参数），同一时刻只有一条使用中；列表里点「启用」即切换并写入目标文件，表单在对话框里编辑。
- **按模型测连通性**：单模型测试、按中转站批量测试，结果持久化。
- **其余能力**：自定义目录（按工具 × 环境）、`.bak` 备份、Gist 同步、系统托盘、更新检查、中英文与明暗主题。

## 目录

```text
cxc-v2/
  cxc-v2-core/          # 数据模型、协议、探测、适配器、同步、迁移
  cxc-v2-desktop/       # Tauri 2 + React 19 桌面端
```

## 数据

配置文件：Linux / WSL `~/.config/cxc-v2/config.yaml`，Windows `%USERPROFILE%\.config\cxc-v2\config.yaml`。

```yaml
version: 4
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
    wsl:
      active: "b1"
      bindings:
        - id: "b1"
          supplier_id: "..."
          protocol: responses
          model: "openai/gpt-5"
          reasoning_effort: high
          context_length: 200k
          compact_ratio: 80
        - id: "b2"
          supplier_id: "..."
          protocol: responses
          model: "gpt-5-mini"
    windows:
      active: "b3"
      bindings:
        - id: "b3"
          supplier_id: "..."
          protocol: responses
          model: "gpt-5-mini"
  claude: {}
  grok: {}
  opencode: {}
custom_dirs:
  codex: { wsl: "", windows: "" }
  claude: { wsl: "", windows: "" }
  grok: { wsl: "", windows: "" }
  opencode: { wsl: "", windows: "" }
```

旧版 `~/.config/cxc/config.yaml` 在首次启动时一次性导入；版本 2 / 3 的 `cxc-v2` 配置会自动收敛成绑定列表（旧配置成为唯一一条使用中的绑定）。旧文件只读，不会被修改。

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

每个工具在 WSL 与 Windows 下各有一份绑定列表，使用中的那条写入各自环境的配置文件。

## 设计文档

- `docs/adr/0018-global-suppliers-and-agent-bindings.md`
- `docs/adr/0019-three-protocol-probing.md`
- `docs/adr/0020-opencode-adapter.md`
- `docs/adr/0021-cxc-v2-config-and-migration.md`
- `docs/adr/0022-agent-per-environment-single-config.md`
- `docs/adr/0023-agent-binding-list.md`
