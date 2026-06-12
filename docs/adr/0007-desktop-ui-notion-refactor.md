# ADR-0007: Desktop UI Refactoring to Notion Design Language

## Status

Accepted

## Context

The user reported that the current `@cxc-desktop` UI/UX is unappealing and requested a complete refactoring of its interface based on the specifications in DESIGN.md. The existing desktop interface uses standard dark-neutral aesthetics with Tailwind CSS, Geist Sans typography, and generic Card surfaces.

We need to redefine the visual identity of the desktop application using Notion's calm, document-like aesthetic, while keeping both light and dark modes fully usable and aesthetically aligned.

## Decision

We will refactor the desktop application UI/UX to implement the Notion design system:

### 1. Unified Theme Colors via Tailwind v4 CSS Variables
We will map the Tailwind theme variables inside index.css to match the Notion system:
* **Light Theme (Notion daylight)**:
  * Page Background (`--background` / `canvas-soft`): Warm paper off-white `#f6f5f4`
  * Card/Panel Surface (`--card` / `--popover` / `surface`): Pure white `#ffffff`
  * Text Primary (`--foreground` / `ink`): Soft true-black `#000000` (rendered at ~95% alpha)
  * Text Secondary (`--muted-foreground` / `warm charcoal`): `#31302e`
  * Borders (`--border` / `hairline`): Thin 1px solid `#e6e6e6`
  * Active/Primary Color (`--primary` / `Notion blue`): `#0075de`
* **Dark Theme (Notion night)**:
  * Page Background (`--background`): Deep charcoal `#191919`
  * Card/Panel Surface (`--card` / `--popover`): Soft charcoal `#222222`
  * Text Primary (`--foreground`): Clean white `#e3e3e3`
  * Text Secondary (`--muted-foreground`): Warm gray `#b3b3b3`
  * Borders (`--border`): `#2c2c2c`
  * Active/Primary Color (`--primary`): Bright Notion blue `#2eaadc` for better dark-mode contrast

### 2. Typography: Geist Sans to Inter
We will replace the `@fontsource-variable/geist` package with `@fontsource-variable/inter` to align with the `NotionInter` specification. We will apply tight letter tracking (negative letter-spacing) to large headings (`tracking-[-2px]`) and spacious tracking for eyebrow tags.

### 3. Layout and UI Components Refactoring
* **Header**: Simplify the sticky header into a clean, document-like navigation header with a white/transparent surface and a thin border.
* **Segmented View Toggle**: Added list/card segmented toggles in the toolbar. Selection defaults to the list view and persists selection across reloads via localStorage.
* **List View**: Designed a clean Notion database-like list view where providers are rendered as structured rows inside a clean panel, collapsing gracefully on mobile, and presenting clear alignment of models, endpoints, active indicators, and actions.
* **Buttons**:
  * **Primary CTAs**: Set primary buttons (`button-primary`) to be pill-shaped (`rounded-full`) painted in Notion Blue.
  * **Secondary / Utility Actions**: Individual card/row actions (Test, Edit, Delete) will use tight, 8px rounded (`rounded-md`) utility buttons (`button-utility`) with thin borders.
* **Cards & Grids**:
  * Simplify the provider cards to use flat white surfaces with hairline borders (`colors.hairline`) rather than heavy elevation.
  * Elevate active provider cards using a subtle Notion Blue border or side-accent indicator.
* **Pill Badges & Stickers**:
  * Use pill badges (`rounded-full`) and utilize the colorful decorative sticker palette (green for fast latency, orange/red for high latency or offline status) for connection testing feedback.

## Consequences

* The desktop application will adopt a clean, document-calm visual layout, improving the reading experience and clarity.
* Theme switching will preserve the aesthetic by transitioning between warm-paper daylight and a aligned dark mode.
* Typography changes from Geist Sans to Inter will align the branding across the desktop app.
