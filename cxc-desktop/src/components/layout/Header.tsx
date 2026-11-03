import { useEffect, useRef } from "react";
import { Bot, Moon, Plus, Search, Server, Settings2, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Messages } from "@/lib/i18n";
import { WindowControls, WindowsWindowControls, isMac } from "./WindowControls";

export type Section = "suppliers" | "agents" | "settings";

interface HeaderProps {
  section: Section;
  onSection: (section: Section) => void;
  search: string;
  onSearch: (value: string) => void;
  onAdd: () => void;
  addLabel: string;
  onSettings: () => void;
  theme: "light" | "dark";
  onToggleTheme: () => void;
  t: Messages;
}

const sections: { key: Section; icon: typeof Server; label: (t: Messages) => string }[] = [
  { key: "suppliers", icon: Server, label: (t) => t.sectionSuppliers },
  { key: "agents", icon: Bot, label: (t) => t.sectionAgents },
];

export function Header({
  section,
  onSection,
  search,
  onSearch,
  onAdd,
  addLabel,
  onSettings,
  theme,
  onToggleTheme,
  t,
}: HeaderProps) {
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "f") return;
      event.preventDefault();
      searchRef.current?.focus();
      searchRef.current?.select();
    };
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  }, []);

  return (
    <header
      data-tauri-drag-region
      className="sticky top-0 z-40 grid w-full grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-3 border-b border-border/30 bg-background/40 px-5 py-3 backdrop-blur-xl"
    >
      <div data-tauri-drag-region className="flex min-w-0 items-center justify-start gap-3">
        <WindowControls />
        <div data-tauri-drag-region className="flex shrink-0 select-none items-center gap-1.5">
          <div className="relative flex size-8 items-center justify-center overflow-hidden rounded-lg border border-border bg-card shadow-sm">
            <img src="/logo.png" alt="CXC" className="size-full object-cover" />
          </div>
          <h1
            className="bg-gradient-to-r from-[#0075de] to-[#00a3ff] bg-clip-text text-sm font-black leading-none tracking-tighter text-transparent"
            style={{ fontFamily: "'Geist Mono Variable', 'Geist Mono', monospace" }}
          >
            CXC
          </h1>
        </div>

        <div className="flex h-8 items-center rounded-lg border border-border/60 bg-muted/40 p-0.5">
          {sections.map(({ key, icon: Icon, label }) => (
            <button
              key={key}
              onClick={() => onSection(key)}
              className={`flex h-full items-center gap-1.5 rounded-md px-2.5 text-[11px] font-bold transition-all ${
                section === key
                  ? "border border-border/10 bg-card text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Icon className="size-3.5" />
              {label(t)}
            </button>
          ))}
        </div>
      </div>

      <div className="group relative w-40 transition-all duration-300 focus-within:w-60 sm:w-48">
        <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground/35 transition-colors group-focus-within:text-primary" />
        <Input
          ref={searchRef}
          value={search}
          onChange={(event) => onSearch(event.target.value)}
          placeholder={t.searchShort}
          aria-label={t.searchPlaceholder}
          className="h-8 w-full rounded-md border border-transparent bg-muted/20 pl-8 pr-12 text-xs shadow-none transition-all placeholder:text-muted-foreground/30 hover:border-border/40 focus:border-primary focus:bg-background focus:ring-0 focus-visible:ring-0"
        />
        <div className="pointer-events-none absolute right-2 top-1/2 flex -translate-y-1/2 items-center transition-all duration-200 group-focus-within:scale-95 group-focus-within:opacity-0">
          <kbd className="hidden h-5 select-none items-center gap-0.5 rounded border border-border/40 bg-muted/30 px-1.5 font-mono text-[9px] font-medium text-muted-foreground/45 shadow-[inset_0_-1px_0_rgba(0,0,0,0.02)] sm:inline-flex">
            {isMac ? "⌘ F" : "Ctrl F"}
          </kbd>
        </div>
      </div>

      <div data-tauri-drag-region className="flex min-w-0 items-center justify-end gap-2">
        {section === "suppliers" && (
          <Button
            onClick={onAdd}
            className="size-8 shrink-0 rounded-md bg-primary text-primary-foreground shadow-sm transition-all hover:bg-primary/90"
            title={addLabel}
          >
            <Plus className="size-4" />
          </Button>
        )}

        <Button
          variant="outline"
          size="icon"
          onClick={onToggleTheme}
          className="size-8 shrink-0 border-border bg-card/50 text-foreground shadow-sm backdrop-blur-md hover:bg-card/70"
          title={theme === "dark" ? "Light" : "Dark"}
        >
          {theme === "dark" ? (
            <Sun className="size-4 text-muted-foreground" />
          ) : (
            <Moon className="size-4 text-muted-foreground" />
          )}
        </Button>
        <Button
          variant="outline"
          size="icon"
          onClick={onSettings}
          className="size-8 shrink-0 border-border bg-card/50 text-foreground shadow-sm backdrop-blur-md hover:bg-card/70"
          title={t.settingsTitle}
        >
          <Settings2 className="size-4 text-muted-foreground" />
        </Button>
        <WindowsWindowControls />
      </div>
    </header>
  );
}
