import assert from "node:assert/strict";
import { test } from "node:test";
import { applyQuickModelChange } from "./quick-model.ts";

function sample() {
  return {
    codex_active_app: "primary",
    codex_active_wsl: "primary",
    claude_active_app: "primary",
    claude_active_wsl: "",
    grok_active_app: "grok-relay",
    grok_active_wsl: "",
    codex_providers: [
      { name: "primary", model: "gpt-4o", base_url: "https://a.example" },
      { name: "other", model: "claude-3-5-sonnet", base_url: "https://b.example" },
    ],
    claude_providers: [{ name: "primary", model: "claude-3-5-sonnet" }],
    grok_providers: [{ name: "grok-relay", model: "grok-4.5" }],
  };
}

test("换另一个中转的模型时不改启用项", () => {
  const next = applyQuickModelChange(sample(), "codex", "other", "gpt-4-turbo");

  assert.equal(next.codex_providers.find((p) => p.name === "other")?.model, "gpt-4-turbo");
  assert.equal(next.codex_providers.find((p) => p.name === "primary")?.model, "gpt-4o");
  assert.equal(next.codex_providers.find((p) => p.name === "other")?.base_url, "https://b.example");
  assert.equal(next.codex_active_app, "primary");
  assert.equal(next.codex_active_wsl, "primary");
  assert.equal(next.claude_active_app, "primary");
  assert.equal(next.grok_active_app, "grok-relay");
});

test("换当前启用中转的模型时仍保持启用", () => {
  const next = applyQuickModelChange(sample(), "codex", "primary", "gpt-4.1");

  assert.equal(next.codex_providers.find((p) => p.name === "primary")?.model, "gpt-4.1");
  assert.equal(next.codex_active_app, "primary");
  assert.equal(next.codex_active_wsl, "primary");
});

test("只改当前工具的列表", () => {
  const next = applyQuickModelChange(sample(), "grok", "grok-relay", "grok-4");

  assert.equal(next.grok_providers?.find((p) => p.name === "grok-relay")?.model, "grok-4");
  assert.equal(next.codex_providers.find((p) => p.name === "primary")?.model, "gpt-4o");
  assert.equal(next.grok_active_app, "grok-relay");
});
