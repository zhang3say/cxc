# CXC — Glossary

## CXC (Code Cross-Connect)

The project name. A meta-configuration tool that manages API relay endpoint configurations for AI coding tools (Codex, Claude, etc.). It provides a desktop application for users to quickly add, test, and switch API relay endpoints for their AI tools.

## Provider（中转站）

A named API relay endpoint configuration. Each Provider consists of a `name`, `base_url`, `api_key`, and `model`. Represents one API proxy or relay service that an AI coding tool (Codex, Claude) can be pointed at.

## Target Tool（目标工具）

An AI coding tool whose configuration CXC manages. Each Target Tool has a known config file path and format. Currently supported targets: Codex and Claude CLI. CXC writes Provider details into the Target Tool's config file when the user switches providers.

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

The globally selected execution environment for configuration writes. CXC mirrors this App / WSL choice across Codex and Claude CLI, so both adapters resolve their target config files using the same source. On Windows, it can be set to either "app" (native Windows Desktop App) or "wsl" (WSL CLI).

## Codex Custom Directory (Codex 自定义目录)

An optional directory path configuration in CXC. Used when the Codex config files reside in a non-standard path, such as inside a specific WSL distribution (accessed via Windows UNC paths like `\\wsl.localhost\Ubuntu\home\<user>\.codex`).

## Claude CLI Adapter (Claude CLI 适配器)

A Target Tool adapter that manages Claude CLI configuration. Unlike Codex which uses separate TOML and JSON files, Claude CLI uses a single `settings.json` file with an `env` object containing environment variables (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, and model configurations).

## Claude Source (Claude 来源)

Mirrors the same global App / WSL source choice used by Codex. It determines which Claude CLI configuration to modify. On Windows, it can be "app" (native Windows config at `%USERPROFILE%\.claude\`) or "wsl" (WSL config at `~/.claude/`). On Linux/WSL, "wsl" means local config, "app" means Windows host config accessed via `/mnt/c/`.

## Claude Models Configuration (Claude 模型配置)

An optional multi-model configuration for Claude CLI providers. Allows mapping different model tiers (Opus, Sonnet, Haiku, Fable) to specific model IDs. When not configured, all tiers fall back to the Provider's primary `model` field. This enables providers like DeepSeek to map high-performance models (v4-pro) to Opus/Sonnet and fast models (v4-flash) to Haiku, matching the official provider recommendations.
