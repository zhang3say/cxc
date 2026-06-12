# CXC — Glossary

## CXC (Code Cross-Connect)

The project name and CLI command name. A meta-configuration tool that manages API relay endpoint configurations for AI coding tools (Codex, Claude, etc.). Users invoke it as `cxc` in the terminal to quickly add, test, and switch API relay endpoints for their AI tools.

## Provider（中转站）

A named API relay endpoint configuration. Each Provider consists of a `name`, `base_url`, `api_key`, and `model`. Represents one API proxy or relay service that an AI coding tool (Codex, Claude) can be pointed at.

## Target Tool（目标工具）

An AI coding tool whose configuration CXC manages. Each Target Tool has a known config file path and format. MVP targets: Codex. Future: Claude. CXC writes Provider details into the Target Tool's config file when the user switches providers.

## Active Provider

The Provider currently configured for a given Target Tool. When the user switches providers, CXC updates the Target Tool's config file to point at the new Provider's endpoint.

## Connectivity Test

A lightweight, real API call (minimal chat completion request) sent to a Provider's endpoint to verify that the `base_url`, `api_key`, and `model` are valid and the service is reachable. Not a synthetic ping — it exercises the actual inference path.

## Remark（备注）

An optional user-defined description or note for a Provider. Helps users distinguish between different API relay endpoints (e.g., "backup relay", "fast relay", "dev test").

## Model Discovery（模型发现）

The act of querying a Provider's `GET /models` endpoint to retrieve the list of models it exposes. Used during Provider creation or editing so users can pick a model from the live list rather than typing it by hand. Requires a valid `base_url` and `api_key`. Distinct from a Connectivity Test — Model Discovery does not validate inference, only enumerates available model IDs.

## Desktop App（桌面端应用）

The graphical user interface (GUI) version of CXC built with Tauri. It provides a visual dashboard for managing, testing, and switching Providers, complementary to the TUI and CLI.

## System Tray（系统托盘）

A persistent quick-access menu in the operating system's notification area. It allows users to switch Active Providers instantly and view notification status without launching the full Desktop App window.

## Codex Source (Codex 来源)

The execution environment of the Target Tool (specifically Codex). It determines where CXC reads and writes the configuration. On Windows, it can be set to either "app" (native Windows Desktop App) or "wsl" (WSL CLI).

## Codex Custom Directory (Codex 自定义目录)

An optional directory path configuration in CXC. Used when the Codex config files reside in a non-standard path, such as inside a specific WSL distribution (accessed via Windows UNC paths like `\\wsl.localhost\Ubuntu\home\<user>\.codex`).

