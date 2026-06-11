# ADR-0003: Refactor CXC to Rust using Clap and Ratatui

## Status

Accepted

## Context

We are refactoring the Go-based CXC implementation to Rust. The application must retain its dual-mode execution model:
1. **CLI mode**: Subcommand routing for automated scripts.
2. **TUI mode**: An interactive full-screen interface launched when no subcommands are provided.

We need to choose appropriate libraries in the Rust ecosystem to replace Go's Cobra, Bubble Tea, go-toml/v2, and Go's concurrency primitives, while preserving all existing behaviors (e.g., AST-preserving TOML configuration mutation).

### Options Considered

* **CLI Framework**: [Clap](https://crates.io/crates/clap) (with derive features) is selected as the default equivalent to Cobra. It is mature, supports structured flags, subcommands, and auto-generated help.
* **TUI Framework**:
  * **Option A: Cursive**: A view-based, event-driven framework. Highly productive for form inputs but difficult to style into high-fidelity, custom-designed dark interfaces. It also introduces callback-based lifetime complexities.
  * **Option B: Ratatui**: An immediate-mode rendering framework. It provides complete style customization (Nord theme, colored remarks, dynamic latency display) but requires manual event handling and custom input components.
  * *Decision*: **Ratatui (with crossterm)** to maintain visual parity and custom styling flexibility.
* **Async Runtime & HTTP Client**:
  * **Option A: Tokio + Reqwest**: A full async ecosystem. Allows building a non-blocking `tokio::select!` main loop in the TUI that processes Crossterm inputs and asynchronous HTTP test outcomes concurrently.
  * **Option B: Standard threads + ureq**: A sync-threaded model. Lower binary footprint but more boilerplate for cross-thread messaging to the TUI.
  * *Decision*: **Tokio + Reqwest** for robust non-blocking HTTP connectivity tests and idiomatic async TUI integration.
* **Configuration Mutation**:
  * **YAML/JSON**: `serde` + `serde_yaml` (for CXC config) and `serde_json` (for Codex `auth.json`).
  * **TOML**: [toml_edit](https://crates.io/crates/toml_edit) is selected for `config.toml` mutation. It parses TOML into an AST, allowing key modification while preserving user comments, blank lines, and unrelated structure.
* **CI/CD & Releases**: [cargo-dist](https://github.com/axodotdev/cargo-dist) is chosen to replace GoReleaser. It generates automated release configurations for GitHub Actions, compiles multi-platform binaries, and auto-produces installer scripts.
* **Testing Seams**:
  * `tempfile` is used for disk I/O tests (creating temporary workspace structures).
  * `wiremock` is used to spin up local HTTP mocks for non-live network connectivity tests.

## Decision

1. Rewrite CXC in Rust using `clap` for the CLI and `ratatui` (with `crossterm` backend) for the TUI.
2. Build TUI control flow around an async event loop using `tokio::select!` and channel-based communication (`tokio::sync::mpsc`).
3. Leverage `toml_edit` to ensure formatting-preserving updates for Codex TOML configs.
4. Implement target tool adapter pattern via `TargetAdapter` trait and `TargetTool` enum.
5. Deploy `cargo-dist` for release management.

## Consequences

- Full code parity with safety guarantees of Rust (no nil-pointer risks or data races).
- TUI input management (cursor index, deletion) must be explicitly managed or delegated to light helper crates (`tui-input`).
- Rust binary size will be slightly larger due to Tokio/Reqwest dependencies, but offset by `cargo-dist` optimization flags.
- Logging is configured to redirect to `~/.config/cxc/cxc.log` when running in TUI mode to prevent stdout/stderr corruption.
