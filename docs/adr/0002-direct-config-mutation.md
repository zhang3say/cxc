# ADR-0002: Direct config file mutation for target tools

## Status

Accepted

## Context

CXC needs to switch API relay endpoints for Codex and Claude. We discovered the following config layout:

### Codex

- **`~/.codex/config.toml`** (TOML):
  - `model` (top-level) — active model name
  - `model_provider` (top-level) — which provider section to use
  - `[model_providers.<name>]` — `base_url`, `wire_api`, `name`, `requires_openai_auth`
- **`~/.codex/auth.json`** (JSON):
  - `auth_mode` — always `"apikey"`
  - `OPENAI_API_KEY` — the API key

### Claude (via Codex shell env)

Claude Code is configured through environment variables set in Codex's config.toml:
- `[shell_environment_policy.set]`:
  - `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`
  - `ANTHROPIC_DEFAULT_HAIKU_MODEL`, `ANTHROPIC_DEFAULT_OPUS_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`
  - `CLAUDE_CODE_SUBAGENT_MODEL`

### Options considered

| Option | Pros | Cons |
|--------|------|------|
| Direct file mutation | Simple, no daemon, instant | Must handle TOML/JSON parsing carefully; risk of clobbering unrelated config |
| Symlink swapping | Atomic switch | Breaks tools that write back to config; complex with two files |
| Wrapper scripts / env injection | Non-invasive | Doesn't persist across sessions; fragile |

## Decision

**Direct file mutation** — CXC reads the existing config files, modifies only the relevant fields, and writes them back. We use a TOML-preserving parser (e.g. `github.com/pelletier/go-toml/v2`) to avoid clobbering comments and unrelated sections.

Safety measures:
1. Create a `.bak` backup before every write
2. Validate the file can be re-parsed after writing
3. Only touch the specific keys CXC manages

## Consequences

- CXC must ship with knowledge of each Target Tool's config schema
- Adding a new Target Tool means adding a new config adapter
- Backup files accumulate; may need a cleanup strategy later
