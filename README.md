# CXC — Code Cross-Connect

> Quickly switch API relay endpoints for AI coding tools like Codex and Claude.

[![Rust Version](https://img.shields.io/badge/rust-1.96%2B-orange)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-green)](LICENSE)

![CXC TUI Preview](docs/images/tui-preview.png)

CXC is a CLI/TUI tool for managing multiple API relay endpoint configurations for AI coding tools. Instead of manually editing TOML and JSON config files, use `cxc` to add, test, and switch between providers in seconds.

## Features

- **CLI mode** — scriptable subcommands for automation
- **TUI mode** — full-screen interactive interface (launch with `cxc`)
- **Provider management** — add, list, test, switch, remove
- **Connectivity test** — real chat completion request with latency measurement
- **Safe switching** — `.bak` backups created before any config file is modified
- **Codex integration** — automatically updates `~/.codex/config.toml` and `~/.codex/auth.json`

## Installation

### One-click Installer (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/zhang3say/cxc/main/install.sh | sh
```

### Cargo Install

```bash
cargo install --git https://github.com/zhang3say/cxc.git
```

### Build from source

```bash
git clone https://github.com/zhang3say/cxc
cd cxc
cargo build --release
```

## Usage

### TUI mode

```bash
cxc
```

Launch the interactive full-screen TUI. Keyboard shortcuts:

| Key | Action |
|-----|--------|
| `↑`/`↓` | Navigate providers |
| `a` | Add a new provider |
| `e` | Edit highlighted provider |
| `t` | Test the highlighted provider |
| `T` | Test all providers concurrently |
| `Enter`/`s` | Switch to highlighted provider |
| `d`/`Delete` | Remove highlighted provider |
| `q`/`Esc` | Quit |

### CLI mode

```bash
# Add a provider (interactive if flags omitted)
cxc provider add --name my-relay \
  --base-url https://api.example.com/v1 \
  --api-key sk-xxx \
  --model gpt-4

# List all providers
cxc provider list

# Test a provider's connectivity
cxc provider test               # tests active provider
cxc provider test my-relay      # tests named provider
cxc provider test --all         # tests all saved providers concurrently (or -a)

# Switch active provider
cxc provider switch my-relay

# Remove a provider
cxc provider remove my-relay
```

## How it works

CXC stores its own provider list at `~/.config/cxc/config.yaml` (0600 permissions).

When you switch providers, CXC updates:
- `~/.codex/config.toml` — sets `model`, `model_providers.codex.base_url`, `wire_api`
- `~/.codex/auth.json` — sets `OPENAI_API_KEY`

Both files are backed up as `.bak` before any write.

## Configuration

CXC's own config at `~/.config/cxc/config.yaml`:

```yaml
active: my-relay
providers:
  - name: my-relay
    base_url: https://api.example.com/v1
    api_key: sk-xxx
    model: gpt-4
    wire_api: responses
```

## Architecture

- **CLI**: [Clap](https://github.com/clap-rs/clap) (derive interface) — subcommand routing
- **TUI**: [Ratatui](https://github.com/ratatui-org/ratatui) (with [crossterm](https://github.com/crossterm-rs/crossterm) backend) — async event loop TUI
- **TOML**: [toml_edit](https://github.com/toml-rs/toml) — format-preserving AST mutation
- **Config**: YAML serialization via [serde_yaml](https://github.com/dtolnay/serde-yaml)

See [docs/adr/](docs/adr/) for architectural decision records.

## License

MIT
