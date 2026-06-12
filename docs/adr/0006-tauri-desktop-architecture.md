# ADR-0006: Tauri Desktop Architecture

## Status

Accepted

## Context

We want to build a desktop graphical user interface (GUI) version of CXC to complement the existing CLI/TUI interfaces. The desktop app needs to:
1. Re-use existing Rust-based business logic (modifying configurations, checking connectivity, etc.).
2. Deliver a visually stunning, responsive user interface with native look-and-feel.
3. Feature a quick switcher via the system notification area (System Tray).
4. Run cleanly on Windows (specifically Win11/WSL2) and other desktop platforms with minimal memory footprint.

We need to establish the architecture, layout, UI stack, and release pipeline for this new component.

## Decision

### Project Layout: Cargo Workspace

We will refactor the repository from a single crate into a Cargo Workspace:
- `cxc-core` (library): Extracted core business logic (Provider database, TargetAdapter, connectivity checks, Model Discovery).
- `cxc-cli` (binary): Original CLI parser and Ratatui TUI, depending on `cxc-core`.
- `cxc-desktop` (binary/Tauri app): Tauri app backend, depending on `cxc-core` and exposing its functions via Tauri command RPCs.

### Technology Stack: Tauri v2 + React + TailwindCSS + shadcn/ui

- **Framework**: Tauri v2 (latest stable), providing native OS windowing, file system access, and system tray management.
- **Frontend**: Vite + React + TypeScript.
- **Styling & UI**: TailwindCSS and shadcn/ui components to produce a premium, modern themeable dashboard.
- **IPC**: React frontend invokes Tauri commands (`cxc-core` functions) using `@tauri-apps/api/core`.

### Interface Modes: Main Window + System Tray

- **Main Window**: Used for advanced administration (adding/editing Providers, testing connections, performing Model Discovery, general settings).
- **System Tray (常驻托盘)**: Resident in the OS taskbar/menu bar. Right-clicking the tray icon presents a quick-access menu displaying all saved Providers; clicking one switches the active provider instantly and displays a system-level desktop notification.

### Packaging & CI/CD: tauri-action

To automate multi-platform builds and code signing for Windows, macOS, and Linux, we will introduce a separate GitHub Actions workflow using the official `tauri-apps/tauri-action`. This separates the GUI packaging from the CLI/TUI cargo-dist workflow.

## Consequences

- The project structure changes to workspace-style.
- Development requires Node.js/npm dependencies in the `cxc-desktop` directory.
- Build files (like `tauri.conf.json`) are introduced under `cxc-desktop/src-tauri`.
