# ADR-0001: Use Cobra + Bubble Tea for CLI/TUI dual-mode architecture

## Status

Superseded by [ADR-0003](0003-refactor-to-rust-clap-ratatui.md)

## Context

CXC needs to support two interaction modes from a single binary:

1. **CLI mode** — traditional subcommands (`cxc provider add`, `cxc provider test`, etc.)
2. **TUI mode** — full-screen interactive terminal UI launched by running `cxc` with no arguments

We need Go libraries that handle both cleanly without fighting each other.

### Options considered

| Option | CLI | TUI | Trade-off |
|--------|-----|-----|-----------|
| Cobra + Bubble Tea | Cobra handles routing/flags/help; Bubble Tea handles TUI | Best-in-class for both; large ecosystem | Two dependencies, but both from mature orgs |
| urfave/cli + tview | Simpler CLI API; tview is widget-oriented | tview uses immediate-mode rendering, harder to compose | Easier to start, harder to evolve |
| Kong + tcell | Kong uses struct tags for CLI; tcell is low-level | Very flexible but lots of boilerplate | Over-engineered for this scope |

## Decision

Use **Cobra** (`spf13/cobra`) for CLI command routing and **Bubble Tea** (`charmbracelet/bubbletea`) for the TUI.

The entry point logic:
- If `os.Args` contains a subcommand → Cobra handles it (CLI mode)
- If no subcommand → launch Bubble Tea full-screen app (TUI mode)

## Consequences

- Cobra's root command `RunE` becomes the TUI entry point
- All subcommands (`provider add`, `provider list`, etc.) are registered as Cobra children
- Bubble Tea models can reuse the same domain logic as CLI handlers
- Both charmbracelet and spf13 are well-maintained; dependency risk is low
