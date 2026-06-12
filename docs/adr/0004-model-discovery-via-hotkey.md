# ADR-0004: Model Discovery via Ctrl+L Hotkey in TUI

## Status

Accepted

## Context

When adding or editing a Provider in the TUI, users must manually type a model name (e.g., `gpt-4o`). Many OpenAI-compatible relay endpoints expose a `GET /models` API that returns the list of available models. We want to let users fetch and pick from this list instead of typing blind.

Three key design questions were resolved:

1. **When to trigger the fetch** — automatically on field focus, or explicitly via a hotkey?
2. **How to display the model list** — inline dropdown within the form, or a new full-screen view mode?
3. **Where to apply** — TUI only, or also CLI?

## Decision

### Trigger: explicit hotkey `Ctrl+L`

The fetch is triggered by `Ctrl+L` ("L" for List) while in the Add or Edit form, after Base URL and API Key fields are filled. It is **not** triggered automatically.

**Rejected alternative — auto-trigger on field focus**: Network requests may be slow or fail. Silent background fetches would confuse the user with no visible cause for delays or state changes.

### UI: new `ViewMode::ModelPicker`

On a successful fetch, the TUI enters a new `ModelPicker` mode that temporarily overlays the form with a navigable model list. `Enter` selects and back-fills the Model field; `Esc` cancels and returns to the form.

**Rejected alternative — inline dropdown**: Requires clipping, z-order logic, and bounded height within an existing form layout. `ModelPicker` as a full `ViewMode` reuses the existing mode-switch pattern cleanly.

### Failure handling

- **Loading**: Model field shows `⟳ fetching…` (same style as connectivity test). Form submission is locked.
- **Empty list** or **fetch error**: Status bar shows a message (`⚠ No models returned` / `✗ <error>`). No mode switch — user falls back to manual input.

### Scope: TUI only

CLI (`cxc add`, `cxc edit`) keeps the existing `prompt_if_empty` text input. CLI usage is typically scripted and non-exploratory; fetch-and-pick adds no value there.

## Endpoint

`GET {base_url}/models` with `Authorization: Bearer {api_key}`. Fixed — all Providers in CXC are OpenAI-compatible relays that follow this standard path.

## Consequences

- A new `ViewMode::ModelPicker` is added to `tui.rs`.
- `TuiApp` gains transient state: a model list buffer and a loading flag.
- An async fetch task (parallel to the existing connectivity-test task pattern) is spawned on `Ctrl+L`.
- CLI code is unchanged.
