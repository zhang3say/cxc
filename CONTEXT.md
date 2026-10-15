# CXC — Glossary

## CXC (Code Cross-Connect)

The project name. A meta-configuration tool that manages API relay endpoint configurations for AI coding tools (Codex, Claude, Grok, etc.). It provides a desktop application for users to quickly add, test, and switch API relay endpoints for their AI tools.

## Provider（中转站）

A named API relay endpoint configuration. Each Provider consists of a `name`, `base_url`, `api_key`, and `model`. Represents one API proxy or relay service that an AI coding tool (Codex, Claude, Grok) can be pointed at.

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

An optional multi-model configuration for Claude CLI providers. Allows mapping different model tiers (Opus, Sonnet, Haiku, Fable) to specific model IDs. When not configured, all tiers fall back to the Provider's primary `model` field. This enables providers like DeepSeek to map high-performance models (v4-pro) to Opus/Sonnet and fast models (v4-flash) to Haiku, matching the official provider recommendations.

## Grok CLI Adapter (Grok CLI 适配器)

A Target Tool adapter that manages Grok CLI configuration. Grok uses a single `~/.grok/config.toml` file: CXC sets `[models].default` and writes the endpoint as a relay block `[model_providers."<中转站名称>"]` carrying `base_url` / `api_key` / `api_backend`, while each managed model block keeps only `model` and a `model_provider` reference (see ADR-0015). Unrelated sections (ui, marketplace, skills, etc.) are preserved via `toml_edit`. OAuth `auth.json` is not modified; relay scenarios use the per-model `api_key` field.

## Grok Relay Model List (Grok 中转站模型列表)

The optional `grok_models` list on a Grok Provider: extra models written alongside the default model, all referencing the same relay block. Removing a model from the list never deletes its block from `config.toml`.

## Grok web_search Policy (Grok web_search 策略)

The `grok_web_search` setting on a Grok Provider deciding how CXC treats `[models].web_search`, which is a separate pin and does **not** follow `[models].default`: keep the file as is, follow the relay's default model, name a specific model, or unset it (searches then use the official endpoint).

## Grok Relay Reorganize (Grok 中转站整理)

A one-time migration action that rewrites `[model.*]` blocks into the relay-centric structure: models sharing an endpoint are grouped into one `[model_providers.<id>]` block, duplicated `base_url` / `api_key` / `api_backend` are removed, and the discovered models are written back into CXC's `grok_models` list. The block key comes from the matching CXC Provider name, or falls back to an existing block key and finally to the `base_url` host.

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

