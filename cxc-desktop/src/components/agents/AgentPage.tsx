import { useEffect, useMemo, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import {
  GripVertical,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { Messages } from "@/lib/i18n";
import type {
  AgentConfig,
  AgentConfigInput,
  AgentKind,
  Config,
  DetectedAgent,
  Env,
  EnvInfo,
  ModelTestResult,
  Protocol,
} from "@/lib/types";
import { AGENT_KINDS, AGENT_LABELS, slotOf } from "@/lib/types";
import { AgentMark } from "./AgentMark";
import { BindingRow } from "./BindingRow";
import { BindingDialog } from "./BindingDialog";

interface AgentPageProps {
  config: Config;
  detected: DetectedAgent[];
  envInfos: EnvInfo[];
  agent: AgentKind;
  env: Env;
  search: string;
  onSelect: (agent: AgentKind, env: Env) => void;
  t: Messages;
  busy: boolean;
  onAdd: (input: AgentConfigInput, activate: boolean) => Promise<void>;
  onEdit: (id: string, input: AgentConfigInput) => Promise<void>;
  onDelete: (binding: AgentConfig) => Promise<void>;
  onActivate: (binding: AgentConfig) => Promise<void>;
  onTest: (
    supplierId: string,
    model: string,
    protocol: Protocol,
    keyId?: string,
  ) => Promise<ModelTestResult>;
  onRefreshModels: (supplierId: string) => Promise<void>;
}

/** 宿主环境不加后缀；WSL 等间接环境带上发行版，避免同名 Agent 混在一起。 */
function agentTitle(kind: AgentKind, info: EnvInfo): string {
  return info.host ? AGENT_LABELS[kind] : `${AGENT_LABELS[kind]} ${info.label}`;
}

const COLLAPSED_KEY = "cxc.agents.sidebar-collapsed";
const ORDER_KEY = "cxc.agents.order";
const ENV_SHORT = (label: string) => label.split(/[\s(]/)[0];

function readOrder(): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(ORDER_KEY) ?? "[]");
    return Array.isArray(parsed)
      ? parsed.filter((item): item is string => typeof item === "string")
      : [];
  } catch {
    return [];
  }
}

export function AgentPage({
  config,
  detected,
  envInfos,
  agent,
  env,
  search,
  onSelect,
  t,
  busy,
  onAdd,
  onEdit,
  onDelete,
  onActivate,
  onTest,
  onRefreshModels,
}: AgentPageProps) {
  const [dialog, setDialog] = useState<{ open: boolean; binding: AgentConfig | null }>({
    open: false,
    binding: null,
  });
  const [saving, setSaving] = useState(false);
  const [testingId, setTestingId] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState(
    () => localStorage.getItem(COLLAPSED_KEY) === "1",
  );
  const [order, setOrder] = useState<string[]>(readOrder);
  const [draggingKey, setDraggingKey] = useState<string | null>(null);
  const [dropTarget, setDropTarget] = useState<{
    key: string;
    after: boolean;
  } | null>(null);
  const pointerDrag = useRef<{
    key: string;
    pointerId: number;
    startX: number;
    startY: number;
    moved: boolean;
    target: { key: string; after: boolean } | null;
  } | null>(null);

  const saveOrder = (keys: string[]) => {
    setOrder(keys);
    localStorage.setItem(ORDER_KEY, JSON.stringify(keys));
  };

  const toggleCollapsed = () => {
    setCollapsed((value) => {
      localStorage.setItem(COLLAPSED_KEY, value ? "0" : "1");
      return !value;
    });
  };

  const orderedInfos = useMemo(
    () => [...envInfos].sort((a, b) => Number(b.host) - Number(a.host)),
    [envInfos],
  );

  // 列表里只放已安装的 Agent；未安装的不展示。
  const navigation = useMemo(() => {
    const items = AGENT_KINDS.flatMap((kind) =>
      orderedInfos.flatMap((info) => {
        const itemDetection = detected.find(
          (item) => item.kind === kind && item.env === info.env,
        );
        if (!itemDetection?.installed) return [];
        const slot = slotOf(config, kind, info.env);
        return [
          {
            key: `${kind}:${info.env}`,
            kind,
            info,
            count: slot?.bindings.length ?? 0,
          },
        ];
      }),
    );
    const rank = new Map(order.map((key, index) => [key, index]));
    return items.sort((a, b) => {
      const aRank = rank.get(a.key);
      const bRank = rank.get(b.key);
      if (aRank === undefined && bRank === undefined) return 0;
      if (aRank === undefined) return 1;
      if (bRank === undefined) return -1;
      return aRank - bRank;
    });
  }, [config, detected, orderedInfos, order]);

  const currentItem = navigation.find(
    (item) => item.kind === agent && item.info.env === env,
  );
  const activeItem = currentItem ?? navigation[0];
  const activeAgent = activeItem?.kind ?? agent;
  const activeEnv = activeItem?.info.env ?? env;

  useEffect(() => {
    if (currentItem || !activeItem) return;
    onSelect(activeItem.kind, activeItem.info.env);
  }, [activeItem, currentItem, onSelect]);

  useEffect(
    () => () => {
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
    },
    [],
  );

  const slot = slotOf(config, activeAgent, activeEnv);
  const bindings = slot?.bindings ?? [];
  const activeId = slot?.active ?? null;
  const detection = detected.find(
    (item) => item.kind === activeAgent && item.env === activeEnv,
  );
  const selectedInfo = activeItem?.info;
  const selectedTitle = selectedInfo
    ? agentTitle(activeAgent, selectedInfo)
    : AGENT_LABELS[activeAgent];

  const visible = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return bindings;
    const agentTerms = `${AGENT_LABELS[activeAgent]} ${selectedInfo?.label ?? ""}`;
    return bindings.filter((binding) => {
      const supplier = config.suppliers.find((item) => item.id === binding.supplier_id);
      return [binding.model, supplier?.name ?? "", agentTerms]
        .join(" ")
        .toLowerCase()
        .includes(query);
    });
  }, [bindings, search, config.suppliers, activeAgent, selectedInfo]);

  const moveItem = (fromKey: string, toKey: string, after: boolean) => {
    if (fromKey === toKey) return;
    const keys = navigation.map((item) => item.key);
    if (!keys.includes(fromKey) || !keys.includes(toKey)) return;
    const next = keys.filter((key) => key !== fromKey);
    const targetIndex = next.indexOf(toKey);
    if (targetIndex < 0) return;
    next.splice(targetIndex + (after ? 1 : 0), 0, fromKey);
    saveOrder(next);
  };

  const moveBy = (key: string, delta: -1 | 1) => {
    const keys = navigation.map((item) => item.key);
    const index = keys.indexOf(key);
    const nextIndex = index + delta;
    if (index < 0 || nextIndex < 0 || nextIndex >= keys.length) return;
    const next = [...keys];
    [next[index], next[nextIndex]] = [next[nextIndex], next[index]];
    saveOrder(next);
  };

  const resetPointerDrag = () => {
    pointerDrag.current = null;
    setDraggingKey(null);
    setDropTarget(null);
    document.body.style.userSelect = "";
    document.body.style.cursor = "";
  };

  const findDropTargetAt = (clientX: number, clientY: number) => {
    const dragged = pointerDrag.current;
    if (!dragged) return null;
    const element = document.elementFromPoint(clientX, clientY);
    const row = element?.closest<HTMLElement>("[data-agent-key]");
    const key = row?.dataset.agentKey;
    if (!row || !key || key === dragged.key) return null;
    const rect = row.getBoundingClientRect();
    return { key, after: clientY > rect.top + rect.height / 2 };
  };

  const startPointerDrag = (
    event: ReactPointerEvent<HTMLSpanElement>,
    key: string,
  ) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    pointerDrag.current = {
      key,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      moved: false,
      target: null,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    setDraggingKey(key);
    setDropTarget(null);
    document.body.style.userSelect = "none";
    document.body.style.cursor = "grabbing";
  };

  const movePointerDrag = (event: ReactPointerEvent<HTMLSpanElement>) => {
    const dragged = pointerDrag.current;
    if (!dragged || dragged.pointerId !== event.pointerId) return;
    event.preventDefault();
    if (!dragged.moved) {
      const distance = Math.hypot(
        event.clientX - dragged.startX,
        event.clientY - dragged.startY,
      );
      if (distance < 4) return;
      dragged.moved = true;
    }

    const scroller = document.getElementById("agent-nav-items")?.parentElement;
    if (scroller) {
      const rect = scroller.getBoundingClientRect();
      if (event.clientY < rect.top + 32) {
        scroller.scrollTop -= 10;
      } else if (event.clientY > rect.bottom - 32) {
        scroller.scrollTop += 10;
      }
    }

    const target = findDropTargetAt(event.clientX, event.clientY);
    dragged.target = target;
    setDropTarget((current) =>
      current?.key === target?.key && current?.after === target?.after
        ? current
        : target,
    );
  };

  const endPointerDrag = (event: ReactPointerEvent<HTMLSpanElement>) => {
    const dragged = pointerDrag.current;
    if (!dragged || dragged.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    if (dragged.moved && dragged.target) {
      moveItem(dragged.key, dragged.target.key, dragged.target.after);
    }
    resetPointerDrag();
  };

  const cancelPointerDrag = (event: ReactPointerEvent<HTMLSpanElement>) => {
    if (pointerDrag.current?.pointerId !== event.pointerId) return;
    resetPointerDrag();
  };

  const submit = async (input: AgentConfigInput, activate: boolean) => {
    setSaving(true);
    try {
      if (dialog.binding) {
        await onEdit(dialog.binding.id, input);
      } else {
        await onAdd(input, activate);
      }
      setDialog({ open: false, binding: null });
    } finally {
      setSaving(false);
    }
  };

  const test = async (binding: AgentConfig) => {
    setTestingId(binding.id);
    try {
      await onTest(
        binding.supplier_id,
        binding.model,
        binding.protocol,
        binding.key_id,
      );
    } finally {
      setTestingId(null);
    }
  };

  if (navigation.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-2 py-20 text-center">
        <p className="text-xs text-muted-foreground">{t.noInstalledAgents}</p>
        <p className="text-[10px] text-muted-foreground/50">
          {t.noInstalledAgentsHint}
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4 md:flex-row md:items-start">
      <TooltipProvider delayDuration={200}>
        <nav
          aria-label={t.sectionAgents}
          className={`shrink-0 transition-[width] duration-200 md:sticky md:top-0 ${
            collapsed ? "md:w-16" : "md:w-56"
          }`}
        >
          <div className="space-y-1 rounded-xl border border-border/25 bg-card/10 p-1.5 backdrop-blur-sm md:max-h-[calc(100vh-6.5rem)] md:overflow-y-auto md:overscroll-contain">
            <div
              className={`flex items-center px-1.5 py-1 ${
                collapsed ? "md:justify-center" : "justify-between"
              }`}
            >
              <span
                className={`text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground/70 ${
                  collapsed ? "md:hidden" : ""
                }`}
              >
                {t.sectionAgents}
              </span>
              <button
                type="button"
                onClick={toggleCollapsed}
                aria-label={collapsed ? t.expandAgents : t.collapseAgents}
                aria-expanded={!collapsed}
                aria-controls="agent-nav-items"
                title={collapsed ? t.expandAgents : t.collapseAgents}
                className="hidden shrink-0 rounded-md p-1 text-muted-foreground/60 transition-colors hover:bg-card/50 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 md:block"
              >
                {collapsed ? (
                  <PanelLeftOpen className="size-3.5" />
                ) : (
                  <PanelLeftClose className="size-3.5" />
                )}
              </button>
            </div>
            <div id="agent-nav-items" className="space-y-1">
              {navigation.map((item) => {
                const selected = item.key === activeItem?.key;
                const itemName = item.info.host
                  ? AGENT_LABELS[item.kind]
                  : `${AGENT_LABELS[item.kind]} ${item.info.label}`;
                return (
                  <Tooltip key={item.key}>
                    <TooltipTrigger asChild>
                      <button
                        type="button"
                        onClick={() => onSelect(item.kind, item.info.env)}
                        data-agent-key={item.key}
                        onKeyDown={(event) => {
                          if (
                            !event.altKey ||
                            (event.key !== "ArrowUp" &&
                              event.key !== "ArrowDown")
                          ) {
                            return;
                          }
                          event.preventDefault();
                          moveBy(item.key, event.key === "ArrowUp" ? -1 : 1);
                        }}
                        aria-label={`${itemName}, ${t.bindings} ${item.count}`}
                        aria-current={selected ? "true" : undefined}
                        aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown"
                        className={`group relative flex w-full items-center gap-2.5 rounded-lg border px-2.5 py-2 text-left transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 ${
                          collapsed
                            ? "md:flex-col md:justify-center md:gap-1 md:px-1"
                            : ""
                        } ${
                          draggingKey === item.key
                            ? "cursor-grabbing opacity-40"
                            : ""
                        } ${
                          dropTarget?.key === item.key
                            ? dropTarget.after
                              ? "after:absolute after:inset-x-2 after:-bottom-[3px] after:z-10 after:h-[2px] after:rounded-full after:bg-primary"
                              : "before:absolute before:inset-x-2 before:-top-[3px] before:z-10 before:h-[2px] before:rounded-full before:bg-primary"
                            : ""
                        } ${
                          selected
                            ? "border-primary/35 bg-primary/[0.06] shadow-[inset_0_0_0_1px_rgba(0,117,222,0.06)]"
                            : "border-transparent text-muted-foreground hover:border-border/30 hover:bg-card/40 hover:text-foreground"
                        }`}
                      >
                        <AgentMark agent={item.kind} />
                        <span
                          className={`min-w-0 flex-1 leading-tight ${
                            collapsed ? "md:hidden" : ""
                          }`}
                        >
                          <span className="block truncate text-xs font-semibold">
                            {AGENT_LABELS[item.kind]}
                          </span>
                          {item.info.host ? null : (
                            <span
                              className="mt-0.5 block truncate text-[10px] font-medium text-muted-foreground/70"
                              title={item.info.label}
                            >
                              {item.info.label}
                            </span>
                          )}
                        </span>
                        <span
                          className={`shrink-0 font-mono text-[10px] text-muted-foreground/55 ${
                            collapsed ? "md:hidden" : ""
                          }`}
                          title={`${t.bindings} ${item.count}`}
                        >
                          {item.count}
                        </span>
                        <span
                          onPointerDown={(event) =>
                            startPointerDrag(event, item.key)
                          }
                          onPointerMove={movePointerDrag}
                          onPointerUp={endPointerDrag}
                          onPointerCancel={cancelPointerDrag}
                          onLostPointerCapture={cancelPointerDrag}
                          onClick={(event) => {
                            event.preventDefault();
                            event.stopPropagation();
                          }}
                          className={`shrink-0 cursor-grab touch-none select-none items-center rounded p-1 active:cursor-grabbing ${
                            collapsed ? "hidden" : "hidden md:flex"
                          }`}
                          title={t.dragToReorder}
                        >
                          <GripVertical
                            className={`size-3 text-muted-foreground/35 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 ${
                              draggingKey === item.key
                                ? "opacity-100"
                                : "opacity-0"
                            }`}
                            aria-hidden="true"
                          />
                        </span>
                        {collapsed && !item.info.host ? (
                          <span className="hidden text-[8px] font-semibold uppercase tracking-wide text-muted-foreground/60 md:block">
                            {ENV_SHORT(item.info.label)}
                          </span>
                        ) : null}
                      </button>
                    </TooltipTrigger>
                    {collapsed ? (
                      <TooltipContent
                        side="right"
                        sideOffset={8}
                        className="hidden md:inline-flex"
                      >
                        <span className="font-semibold">{itemName}</span>
                        <span className="text-background/70">
                          {t.bindings} {item.count}
                        </span>
                      </TooltipContent>
                    ) : null}
                  </Tooltip>
                );
              })}
            </div>
          </div>
        </nav>
      </TooltipProvider>

      <div className="min-w-0 flex-1 space-y-3">
        <div className="px-1">
          <div className="flex flex-wrap items-center gap-2">
            <AgentMark agent={activeAgent} />
            <h2 className="truncate text-sm font-bold tracking-tight">
              {selectedTitle}
            </h2>
          </div>
          <p className="mt-1 truncate font-mono text-[10px] text-muted-foreground/60">
            {detection?.config_dir ?? ""}
          </p>
        </div>

        <div className="overflow-hidden rounded-xl">
          <div className="flex items-center justify-between border-b border-border/30 px-4 py-2.5">
            <span className="text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground/70">
              {t.bindings} {bindings.length}
            </span>
            <Button
              size="xs"
              onClick={() => setDialog({ open: true, binding: null })}
              disabled={busy || saving}
              className={bindings.length === 0 ? "invisible" : ""}
            >
              <Plus className="size-3" />
              {t.newBinding}
            </Button>
          </div>

        {bindings.length === 0 ? (
          <div className="flex flex-col items-center gap-2 py-12 text-center">
            <p className="text-xs text-muted-foreground">{t.noBindings}</p>
            <Button size="sm" onClick={() => setDialog({ open: true, binding: null })}>
              <Plus className="size-3.5" />
              {t.newBinding}
            </Button>
          </div>
        ) : visible.length === 0 ? (
          <div className="py-12 text-center text-xs text-muted-foreground">
            {t.emptySearch}
          </div>
        ) : (
          visible.map((binding, index) => {
            const isActive = binding.id === activeId;
            const previousActive =
              index > 0 && visible[index - 1].id === activeId;
            return (
              <div
                key={binding.id}
                className={
                  index > 0 && !isActive && !previousActive
                    ? "border-t border-border/30"
                    : ""
                }
              >
                <BindingRow
                  binding={binding}
                  supplier={config.suppliers.find(
                    (item) => item.id === binding.supplier_id,
                  )}
                  agent={activeAgent}
                  isActive={isActive}
                  busy={busy || saving}
                  testing={testingId === binding.id}
                  t={t}
                  onActivate={() => void onActivate(binding)}
                  onEdit={() => setDialog({ open: true, binding })}
                  onDelete={() => void onDelete(binding)}
                  onTest={() => void test(binding)}
                />
              </div>
            );
          })
        )}
        </div>
      </div>

      <BindingDialog
        open={dialog.open}
        agent={activeAgent}
        env={activeEnv}
        envLabel={selectedInfo?.label}
        binding={dialog.binding}
        suppliers={config.suppliers}
        isActive={dialog.binding?.id === activeId}
        t={t}
        saving={saving}
        onClose={() => setDialog({ open: false, binding: null })}
        onSubmit={submit}
        onTest={onTest}
        onRefreshModels={onRefreshModels}
      />
    </div>
  );
}
