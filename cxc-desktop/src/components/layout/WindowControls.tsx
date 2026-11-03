import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";
import { isTauri } from "@/lib/api";

export const isMac =
  typeof navigator !== "undefined" && navigator.userAgent.toLowerCase().includes("mac");

function run(action: "close" | "minimize" | "maximize") {
  if (!isTauri) return;
  const window = getCurrentWindow();
  if (action === "close") void window.close();
  if (action === "minimize") void window.minimize();
  if (action === "maximize") void window.toggleMaximize();
}

export function WindowControls() {
  if (!isMac) return null;

  return (
    <div className="mr-1 flex items-center gap-1.5">
      <button
        onClick={() => run("close")}
        className="size-3 rounded-full border border-black/10 bg-[#ff5f57] transition-transform hover:scale-110"
        title="Close"
      />
      <button
        onClick={() => run("minimize")}
        className="size-3 rounded-full border border-black/10 bg-[#febc2e] transition-transform hover:scale-110"
        title="Minimize"
      />
      <button
        onClick={() => run("maximize")}
        className="size-3 rounded-full border border-black/10 bg-[#28c840] transition-transform hover:scale-110"
        title="Maximize"
      />
    </div>
  );
}

export function WindowsWindowControls() {
  if (isMac) return null;

  return (
    <div className="ml-1 flex h-8 shrink-0 items-center gap-0.5 border-l border-border/40 pl-1">
      <button
        onClick={() => run("minimize")}
        className="flex size-7 items-center justify-center rounded-md text-foreground/70 transition-colors hover:bg-muted/80 hover:text-foreground"
        title="Minimize"
      >
        <Minus className="size-3.5" />
      </button>
      <button
        onClick={() => run("maximize")}
        className="flex size-7 items-center justify-center rounded-md text-foreground/70 transition-colors hover:bg-muted/80 hover:text-foreground"
        title="Maximize"
      >
        <Square className="size-3" />
      </button>
      <button
        onClick={() => run("close")}
        className="flex size-7 items-center justify-center rounded-md text-foreground/70 transition-colors hover:bg-red-500 hover:text-white"
        title="Close"
      >
        <X className="size-3.5" />
      </button>
    </div>
  );
}
