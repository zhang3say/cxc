export type TargetTool = "codex" | "claude" | "grok";

type NamedModel = { name: string; model: string };

/** 快速换模型只改卡片上的模型，激活指针保持不动。 */
export interface QuickModelLists {
  codex_providers: NamedModel[];
  claude_providers: NamedModel[];
  grok_providers?: NamedModel[];
  codex_active_app?: string;
  codex_active_wsl?: string;
  claude_active_app?: string;
  claude_active_wsl?: string;
  grok_active_app?: string;
  grok_active_wsl?: string;
}

export function applyQuickModelChange<T extends QuickModelLists>(
  config: T,
  tool: TargetTool,
  name: string,
  model: string,
): T {
  const next = { ...config };
  const patch = <P extends NamedModel>(list: P[] | undefined): P[] =>
    (list ?? []).map((provider) =>
      provider.name === name ? { ...provider, model } : provider,
    );

  if (tool === "codex") {
    next.codex_providers = patch(config.codex_providers);
  } else if (tool === "claude") {
    next.claude_providers = patch(config.claude_providers);
  } else {
    next.grok_providers = patch(config.grok_providers);
  }

  return next;
}
