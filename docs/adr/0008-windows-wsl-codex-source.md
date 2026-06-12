# ADR-0008: Windows and WSL Codex Source Selection and Path Customization

## Status

Accepted

## Context

When running CXC on Windows, users may want to configure their AI relay providers to sync with either the Windows "Codex Desktop App" or the "Codex CLI" running inside the WSL (Windows Subsystem for Linux) environment. 

These environments store their `.codex` configuration directories in different locations. Crucially, a Windows-native desktop process (like the Tauri backend) must access WSL files using UNC network share paths (e.g., `\\wsl.localhost\Ubuntu\home\<username>\.codex`). We need a unified configuration mechanism to dynamically resolve the target config path based on user choice, without bloating the frontend or client modules with path resolution code.

## Decision

We will implement a unified Codex source resolution mechanism across the core and desktop layers:

### 1. Unified Configuration Schema Extension
In `cxc-core/src/config.rs`, we extend the global `Config` struct with two new fields:
* `codex_source`: An optional string specifying `"app"` (default) or `"wsl"`.
* `codex_custom_dir`: An optional string representing a custom absolute directory path (essential for WSL UNC paths on Windows).

### 2. Core Adapter Resolution Logic
In `cxc-core/src/target/codex.rs`, `CodexAdapter::new()` will automatically query these global settings. If `codex_source` is set to `"wsl"` and a `codex_custom_dir` is defined, the adapter will use the custom UNC directory path as the source of truth. Otherwise, it defaults to the standard user home directory (`~/.codex`).

This centralization guarantees that the CLI, TUI, and Tauri desktop backends instantly inherit the correct file resolution behavior.

### 3. Tauri RPC Settings Command
In `cxc-desktop/src-tauri/src/lib.rs`, we introduce and register a new Tauri command:
* `save_settings(source: String, custom_dir: String)`: Allows the React frontend to update global configuration values and write them to disk.

### 4. Notion-Style Settings UI
In `cxc-desktop/src/App.tsx`, we introduce:
* A Settings gear button in the header bar.
* A clean, Notion-style Settings dialog allowing users to toggle between **Desktop App** and **WSL CLI** sources.
* A custom directory input field that dynamically updates placeholders and displays helper notes tailored to the selected option (such as illustrating UNC paths for WSL).

## Consequences

* Windows users can now seamlessly manage their Codex configuration whether it is based in the native Windows filesystem or inside a WSL Linux container.
* The system keeps the path-resolution logic strictly inside the Rust Core layer, leaving the frontend UI decoupled from filesystem path mechanics.
