import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, Loader2, RefreshCw } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import type { Messages } from "@/lib/i18n";
import type { SupplierModel } from "@/lib/types";

const MAX_RENDERED = 2000;

interface ModelPickerProps {
  value: string;
  models: SupplierModel[];
  placeholder: string;
  disabled?: boolean;
  refreshing: boolean;
  refreshError: string | null;
  onChange: (value: string) => void;
  onRefresh: () => Promise<void>;
  /** 下拉开合状态，外层对话框用它决定 Esc 先关下拉还是先关对话框。 */
  onOpenChange?: (open: boolean) => void;
  t: Messages;
}

export function ModelPicker({
  value,
  models,
  placeholder,
  disabled,
  refreshing,
  refreshError,
  onChange,
  onRefresh,
  onOpenChange,
  t,
}: ModelPickerProps) {
  const [text, setText] = useState(value);
  const [open, setOpen] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [highlight, setHighlight] = useState(0);
  const listRef = useRef<HTMLDivElement | null>(null);
  const anchorRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const previousRefreshing = useRef(refreshing);
  const [rect, setRect] = useState<{
    top: number;
    left: number;
    width: number;
    flip: boolean;
    listMax: number;
  } | null>(null);

  const updateRect = useCallback(() => {
    // 面板宽度对齐模型输入框本身，不含右侧刷新按钮。
    const node = inputRef.current;
    if (!node) return;
    const box = node.getBoundingClientRect();
    const spaceBelow = window.innerHeight - box.bottom;
    const flip = spaceBelow < 280 && box.top > spaceBelow;
    const available = (flip ? box.top : spaceBelow) - 12;
    const listMax = Math.min(240, Math.max(96, available - 44));
    setRect({
      top: flip ? box.top : box.bottom,
      left: box.left,
      width: box.width,
      flip,
      listMax,
    });
  }, []);

  useEffect(() => {
    onOpenChange?.(open);
  }, [open, onOpenChange]);

  useEffect(() => {
    if (!open) {
      setRect(null);
      return;
    }
    updateRect();
    const onScroll = () => updateRect();
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onScroll);
    return () => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onScroll);
    };
  }, [open, updateRect]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!target) return;
      if (anchorRef.current?.contains(target)) return;
      if (panelRef.current?.contains(target)) return;
      setOpen(false);
      setDirty(false);
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    return () => document.removeEventListener("pointerdown", onPointerDown, true);
  }, [open]);

  useEffect(() => {
    setText(value);
    setDirty(false);
  }, [value]);

  const filtered = useMemo(() => {
    const query = dirty ? text.trim().toLowerCase() : "";
    if (!query) return models;
    return models.filter((entry) => entry.model.toLowerCase().includes(query));
  }, [models, text, dirty]);

  const truncated = filtered.length > MAX_RENDERED;
  const rendered = truncated ? filtered.slice(0, MAX_RENDERED) : filtered;
  const customValue = text.trim();
  // 只在没有任何匹配时才提供自定义入口，避免有结果时多出一行干扰。
  const customOption =
    dirty &&
    customValue.length > 0 &&
    filtered.length === 0 &&
    !models.some((entry) => entry.model === customValue)
      ? customValue
      : null;
  const optionCount = rendered.length + (customOption ? 1 : 0);

  const missingCurrent =
    value.trim().length > 0 &&
    models.length > 0 &&
    !models.some((entry) => entry.model === value.trim());

  useEffect(() => {
    setHighlight(0);
  }, [text, open]);

  useEffect(() => {
    if (!open) return;
    const node = listRef.current?.querySelector<HTMLElement>(
      `[data-index="${highlight}"]`,
    );
    node?.scrollIntoView({ block: "nearest" });
  }, [highlight, open]);

  // 刷新成功后，输入为空时自动填入第一项。
  useEffect(() => {
    const wasRefreshing = previousRefreshing.current;
    previousRefreshing.current = refreshing;
    if (!wasRefreshing || refreshing) return;
    if (text.trim() || models.length === 0) return;
    const first = models[0].model;
    setText(first);
    setDirty(false);
    onChange(first);
  }, [refreshing, models, text, onChange]);

  const commit = (next: string) => {
    setText(next);
    setDirty(false);
    setOpen(false);
    onChange(next);
  };

  const choose = (index: number) => {
    if (index < rendered.length) {
      commit(rendered[index].model);
    } else if (customOption) {
      commit(customOption);
    }
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (!open) {
        setOpen(true);
        return;
      }
      setHighlight((current) => Math.min(current + 1, Math.max(optionCount - 1, 0)));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setHighlight((current) => Math.max(current - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      if (!open) return;
      event.preventDefault();
      if (optionCount === 0) {
        setOpen(false);
        return;
      }
      choose(highlight);
      return;
    }
    if (event.key === "Escape") {
      if (!open) return;
      event.preventDefault();
      setText(value);
      setDirty(false);
      setOpen(false);
    }
  };

  // 失焦只提交输入值；关闭由 Radix 的 outside 交互负责，
  // 否则内容挂载时抢焦点会让下拉刚打开就收起。
  const handleBlur = () => {
    setDirty(false);
    const next = text.trim();
    if (next !== value) {
      onChange(next);
    }
  };

  const panel = open && rect ? (
    <div
      ref={panelRef}
      data-cxc-model-picker-panel
      style={{
        position: "fixed",
        left: rect.left,
        width: rect.width,
        top: rect.flip ? undefined : rect.top + 4,
        bottom: rect.flip ? window.innerHeight - rect.top + 4 : undefined,
        zIndex: 80,
        // Radix Dialog 会把 body 设为 pointer-events: none，面板挂在 body 上需要显式恢复。
        pointerEvents: "auto",
      }}
      className="overflow-hidden rounded-lg border border-border/60 bg-card shadow-xl"
    >
      <div className="flex items-center justify-between border-b border-border/40 bg-muted/20 px-2.5 py-1.5">
        <span className="text-[9px] font-bold uppercase tracking-wider text-muted-foreground/70">
          {dirty && text.trim() ? `${t.matchingModels} ${filtered.length}` : t.allModels}
        </span>
        <span className="text-[9px] font-bold uppercase tracking-wider text-muted-foreground/60">
          {t.totalModels} {models.length}
        </span>
      </div>

      {refreshError ? (
        <p className="border-b border-red-200 bg-red-50 px-2.5 py-1.5 text-[10px] text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400">
          {refreshError}
        </p>
      ) : null}

      <div
        ref={listRef}
        className="overflow-y-auto py-1"
        style={{ maxHeight: rect.listMax }}
        onMouseDown={(event) => event.preventDefault()}
      >
        {optionCount === 0 ? (
          <p className="px-2.5 py-3 text-center text-[10px] text-muted-foreground">
            {models.length === 0 ? t.noModelsYet : t.emptySearch}
          </p>
        ) : (
          <>
            {rendered.map((entry, index) => (
              <button
                key={entry.model}
                type="button"
                data-index={index}
                onMouseEnter={() => setHighlight(index)}
                onClick={() => choose(index)}
                className={`flex w-full items-center justify-between gap-2 px-2.5 py-1.5 text-left transition-colors ${
                  highlight === index ? "bg-primary/[0.07]" : "hover:bg-muted/30"
                }`}
              >
                <span className="flex min-w-0 items-center gap-1.5">
                  {entry.model === value ? (
                    <Check className="size-2.5 shrink-0 text-primary" />
                  ) : (
                    <span className="size-2.5 shrink-0" />
                  )}
                  <span
                    className="truncate font-mono text-[10.5px] text-foreground/85"
                    title={entry.model}
                  >
                    {entry.model}
                  </span>
                </span>
                {entry.last_ok != null ? (
                  <span
                    className={`shrink-0 font-mono text-[9px] font-bold ${
                      entry.last_ok ? "text-sticker-green" : "text-red-500"
                    }`}
                    title={entry.last_error}
                  >
                    {entry.last_ok ? t.success : t.failed} · {entry.latency_ms ?? 0} ms
                  </span>
                ) : null}
              </button>
            ))}

            {customOption ? (
              <button
                type="button"
                data-index={rendered.length}
                onMouseEnter={() => setHighlight(rendered.length)}
                onClick={() => choose(rendered.length)}
                className={`flex w-full items-center gap-1.5 border-t border-border/40 px-2.5 py-1.5 text-left transition-colors ${
                  highlight === rendered.length ? "bg-primary/[0.07]" : "hover:bg-muted/30"
                }`}
              >
                <span className="size-2.5 shrink-0" />
                <span className="truncate font-mono text-[10.5px] text-primary">
                  {t.useCustomModel} “{customOption}”
                </span>
              </button>
            ) : null}
          </>
        )}
      </div>

      {truncated ? (
        <p className="border-t border-border/40 bg-muted/20 px-2.5 py-1.5 text-[9px] text-muted-foreground/70">
          {t.truncatedModels}
        </p>
      ) : null}
    </div>
  ) : null;

  return (
    <div className="relative">
      <div ref={anchorRef} className="flex items-center gap-1.5">
        <div className="relative min-w-0 flex-1">
          <Input
            ref={inputRef}
            value={text}
            disabled={disabled}
            placeholder={placeholder}
            onChange={(event) => {
              setText(event.target.value);
              setDirty(true);
              setOpen(true);
            }}
            onFocus={() => {
              setOpen(true);
              setDirty(false);
            }}
            onClick={() => {
              // Esc 收起后输入框仍有焦点，再点一次要能重新展开。
              setOpen(true);
              setDirty(false);
            }}
            onBlur={handleBlur}
            onKeyDown={handleKeyDown}
            className="h-9 rounded-md border-border/30 bg-muted/10 pr-8 font-mono text-xs focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
          />
          <ChevronDown
            aria-hidden
            className={`pointer-events-none absolute right-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground/50 transition-transform duration-150 ${
              open ? "rotate-180" : ""
            }`}
          />
        </div>
        <Button
          type="button"
          variant="outline"
          size="icon"
          disabled={disabled || refreshing}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => void onRefresh()}
          className="size-9 shrink-0 border-border/40"
          title={t.refreshModels}
        >
          {refreshing ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <RefreshCw className="size-3.5" />
          )}
        </Button>
      </div>

      {missingCurrent ? (
        <p className="mt-1 text-[10px] text-amber-600 dark:text-amber-400">
          {t.modelNotInList}
        </p>
      ) : null}

      {panel ? createPortal(panel, document.body) : null}
    </div>
  );
}
