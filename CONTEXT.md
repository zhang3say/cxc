# CXC — Glossary

## CXC (Code Cross-Connect)

The project name. A meta-configuration tool that manages API relay endpoint configurations for AI coding tools (Codex, Claude, Grok, etc.). It provides a desktop application for users to quickly add, test, and switch API relay endpoints for their AI tools.

## Provider（中转站）

A named API relay endpoint configuration. Each Provider consists of a `name`, `base_url`, `api_key`, and `model`. Represents one API proxy or relay service that an AI coding tool (Codex, Claude, Grok) can be pointed at.

## Reasoning Effort（推理档位）

The selected reasoning level on a Provider. Every model on that card uses it. Codex also keeps a separate plan-mode level on the same card. When a level is absent, activating the card removes the corresponding key in the target tool. `none` is not a level.
_Avoid_: 推理强度, reasoning menu, effort cap

## Context Length（上下文长度）

Optional size stored on a Provider. Claude's form only offers unset and `1m`. Codex and Grok also accept `200k`, `500k`, and custom sizes. `1m` is 1,000,000 tokens. Blank or `none` means unset. Activating a Claude card appends `[1m]` to the model names when the length is `1m`. Codex writes `model_context_window`. Grok writes `context_window` on the model blocks this activation manages. A connectivity test never sends the `[1m]` suffix.
_Avoid_: 上下文窗口菜单, 1MiB, 1048576

## Compact Ratio（压缩比例）

Optional integer percent from 70 to 100 on a Provider. Claude and Codex convert it with the context length into a token threshold (`CLAUDE_CODE_AUTO_COMPACT_WINDOW`, `model_auto_compact_token_limit`), clamped to 100,000–1,000,000. Grok writes the percent itself to `[session].auto_compact_threshold_percent`. Outside 70–100 means unset, and activating the card removes the key CXC owns.
_Avoid_: 压缩率小数, autocompact percent override

## Target Tool（目标工具）

An AI coding tool whose configuration CXC manages. Each Target Tool has a known config file path and format. Currently supported targets: Codex, Claude CLI, and Grok. CXC writes Provider details into the Target Tool's config file when the user switches providers.

## Active Provider

The Provider currently configured for a specific Source (execution environment) within a Target Tool. Because environments like App and WSL are distinct, each Source independently tracks its own Active Provider, even though they share the same list of available Providers. When the user switches providers, CXC updates the specific config file associated with the currently selected Source.

## Connectivity Test

A lightweight, real API call (minimal chat completion request) sent to a Provider's endpoint to verify that the `base_url`, `api_key`, and `model` are valid and the service is reachable. Not a synthetic ping — it exercises the actual inference path.

## Remark（备注）

An optional user-defined description or note for a Provider. Helps users distinguish between different API relay endpoints (e.g., "backup relay", "fast relay", "dev test").

## Model Discovery（模型发现）

The act of querying a Provider's `GET /models` endpoint to retrieve the list of models it exposes. Used during Provider creation or editing so users can pick a model from the live list rather than typing it by hand. Requires a valid `base_url` and `api_key`. Distinct from a Connectivity Test — Model Discovery does not validate inference, only enumerates available model IDs.

## Desktop App（桌面端应用）

The graphical user interface (GUI) of CXC built with Tauri. It features a frameless, integrated visual canvas that adapts to the host operating system's native window controls and vibrancy physics. It enforces desktop application behavior by preventing browser-specific interactions and intelligently distributing notifications between lightweight in-app toasts and system-level OS notifications.

## System Tray（系统托盘）

A persistent quick-access menu in the operating system's notification area. It allows users to switch Active Providers instantly and view notification status without launching the full Desktop App window. Note: As a current design limitation, the system tray menu only displays and manages the provider list for the Codex target tool.

## Codex Source (Codex 来源)

The globally selected execution environment for configuration writes. CXC mirrors this App / WSL choice across Codex, Claude CLI, and Grok, so all adapters resolve their target config files using the same source. On Windows, it can be set to either "app" (native Windows Desktop App) or "wsl" (WSL CLI).

## Codex Custom Directory (Codex 自定义目录)

An optional directory path configuration in CXC. Used when the Codex config files reside in a non-standard path, such as inside a specific WSL distribution (accessed via Windows UNC paths like `\\wsl.localhost\Ubuntu\home\<user>\.codex`).

## Claude CLI Adapter (Claude CLI 适配器)

A Target Tool adapter that manages Claude CLI configuration. Unlike Codex which uses separate TOML and JSON files, Claude CLI uses a single `settings.json` file with an `env` object containing environment variables (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, and model configurations).

## Claude Source (Claude 来源)

Mirrors the same global App / WSL source choice used by Codex. It determines which Claude CLI configuration to modify. On Windows, it can be "app" (native Windows config at `%USERPROFILE%\.claude\`) or "wsl" (WSL config at `~/.claude/`). On Linux/WSL, "wsl" means local config, "app" means Windows host config accessed via `/mnt/c/`.

## Claude Models Configuration (Claude 模型配置)

An optional multi-model configuration for Claude CLI providers. Maps Opus, Sonnet, Haiku, and Subagent to specific model IDs. An empty tier is written as the Provider's primary `model`. Fable is not a separate field; `ANTHROPIC_DEFAULT_FABLE_MODEL` always follows that primary model. See ADR-0017.

## Grok CLI Adapter (Grok CLI 适配器)

A Target Tool adapter that manages Grok CLI configuration. Grok uses a single `~/.grok/config.toml` file: CXC sets `[models].default` and writes the endpoint as a relay block `[model_providers."<中转站名称>"]` carrying `base_url` / `api_key` / `api_backend`, while each managed model block keeps only `model` and a `model_provider` reference (see ADR-0015). Unrelated sections (ui, marketplace, skills, etc.) are preserved via `toml_edit`. OAuth `auth.json` is not modified; relay scenarios use the per-model `api_key` field.

## Grok web_search Policy (Grok web_search 策略)

The `grok_web_search` setting on a Grok Provider deciding how CXC treats `[models].web_search`. The default is to follow the relay's default model. The other states keep the file as is, name a specific model, or unset the field so searches use the official endpoint.

## Grok Relay Reorganize (Grok 中转站整理)

A one-time migration action that rewrites `[model.*]` blocks into the relay-centric structure: models sharing an endpoint are grouped into one `[model_providers.<id>]` block, and duplicated `base_url` / `api_key` / `api_backend` are removed. The block key comes from the matching CXC Provider name, or falls back to an existing block key and finally to the `base_url` host. Existing model blocks are not deleted.

## Grok Source (Grok 来源)

Mirrors the same global App / WSL source choice used by Codex and Claude. It determines which Grok configuration directory to modify (`~/.grok`, Windows host path, or WSL UNC path).

## Grok Custom Directory (Grok 自定义目录)

An optional directory path configuration in CXC. Used when the Grok config files reside in a non-standard path, such as inside a specific WSL distribution (accessed via Windows UNC paths like `\\wsl.localhost\Ubuntu\home\<user>\.grok`).

## GitHub Gist Sync (配置同步)

基于 GitHub Secret Gist 的跨设备配置同步机制。允许用户仅凭借具备 Gist 权限的 GitHub Personal Access Token 即可在多台开发工作站之间同步服务商 Provider 列表，免除集中式后端服务器的搭建与运维。

## Sync Payload (同步载荷)

跨设备流转的标准化数据集。包含 Codex、Claude CLI 和 Grok 的全量中转服务商配置。显式排除当前机器特异性的执行环境来源（`app` / `wsl`）、本地自定义路径（WSL UNC 路径）以及各端各自的激活状态，杜绝跨操作系统与异构环境之间的配置污染。

## End-to-End Encryption (端到端加密)

面向同步数据的客户端主动安全加固机制。当用户设置了同步主密码时，同步载荷在上传到 GitHub Gist 前会在本地通过 AES-256-GCM 算法全量加密，确保 API Key 与机密中转凭证在云端永不以明文留存。

## CXC 2 (第二代桌面端)

`cxc-v2/` 下的并行重写版本。沿用旧版视觉语言，使用独立配置 `~/.config/cxc-v2/config.yaml`，旧版配置只作为一次性导入来源。数据模型是「全局中转站 + 每个 Agent 一份配置」，Agent 的身份是「工具 + 环境」。

## Environment (环境)

Agent 所在的运行环境，取值为 `wsl` 或 `windows`。环境是 Agent 身份的一部分：`Grok (WSL)` 与 `Grok (Windows)` 是两个独立 Agent，各自一份配置、各自写入。不可达的环境不显示（Linux / WSL 下 Windows 侧依赖 `/mnt/c`，Windows 下 WSL 侧依赖 `wsl.exe`）。

## Supplier (全局中转站)

CXC 2 中只保存一份的中转站凭证：`id`、名称、Base URL、API Key、协议集合、备注和模型目录。所有 Agent 共用同一批 Supplier；Agent 配置通过 `supplier_id` 引用它。删除被引用的 Supplier 会被拒绝。

## Agent Config (Agent 配置)

某个 `(工具, 环境)` 唯一的一份配置：引用一个 Supplier，并保存协议、模型、推理档位、上下文长度、压缩比例，以及 Claude 多模型映射或 Grok web_search 策略。Agent 卡片内直接编辑，点「保存并写入」立即写目标配置文件；写入失败时配置保持不变。

## Protocol Set (协议集合)

Supplier 上确认支持的协议集合，取值为 `chat_completions`、`responses`、`messages`。可以手选，也可以对端点探测后写回；探测只增不删。Agent 配置的协议必须同时被工具和 Supplier 支持。

## Agent Detection (Agent 自动识别)

按 PATH 探测每个 `(工具, 环境)` 是否已安装：本机环境用 `which`，跨环境用 `cmd.exe /c where` 或 `wsl.exe command -v`。配置目录只用于标记「已配置 / 未配置」和定位写入目标；四个工具固定显示，用状态点标出安装状态。

## Model Connectivity (模型连通性)

对 Supplier 模型目录中的单个模型发送最小推理请求，记录延迟、成功状态和错误。支持单模型测试和按中转站批量测试；结果持久化在模型条目上。


