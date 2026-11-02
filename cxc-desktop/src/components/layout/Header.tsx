import { Bot, Moon, Plus, Search, Server, Settings2, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Messages } from "@/lib/i18n";
import { WindowControls, WindowsWindowControls } from "./WindowControls";

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
  return (
    <header
      data-tauri-drag-region
      className="sticky top-0 z-40 flex w-full items-center justify-between gap-3 border-b border-border/30 bg-background/40 px-5 py-3 backdrop-blur-xl"
    >
      <div data-tauri-drag-region className="flex min-w-0 items-center gap-3">
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

      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center justify-end gap-2">
        <div className="group relative min-w-0 max-w-56 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground/35 transition-colors group-focus-within:text-primary" />
          <Input
            value={search}
            onChange={(event) => onSearch(event.target.value)}
            placeholder={t.searchPlaceholder}
            className="h-8 w-full rounded-md border border-transparent bg-muted/20 pl-8 pr-3 text-xs shadow-none transition-all placeholder:text-muted-foreground/30 hover:border-border/40 focus:border-primary focus:bg-background focus:ring-0 focus-visible:ring-0"
          />
        </div>

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
          className="size-8 shrink-0 border-border bg-card text-foreground shadow-sm"
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
          className="size-8 shrink-0 border-border bg-card text-foreground shadow-sm"
          title={t.settingsTitle}
        >
          <Settings2 className="size-4 text-muted-foreground" />
        </Button>
        <WindowsWindowControls />
      </div>
    </header>
  );
}
