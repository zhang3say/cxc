import { ExternalLink, Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { Messages } from "@/lib/i18n";
import type { UpdateInfo } from "@/lib/types";

interface UpdateDialogProps {
  open: boolean;
  info: UpdateInfo | null;
  t: Messages;
  onClose: () => void;
  onOpenUrl: (url: string) => void;
}

function formatPublished(value: string): string {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

/** 把 Release 正文按行渲染成纯文本：只识别标题与短横线列表，不解析 HTML。 */
function ReleaseNotes({ notes }: { notes: string }) {
  const lines = notes.split("\n");
  return (
    <div className="max-h-64 space-y-1 overflow-y-auto rounded-md border border-border/40 bg-muted/10 p-3 text-[11px] leading-relaxed text-foreground/80">
      {lines.map((line, index) => {
        const key = `${index}-${line}`;
        if (line.startsWith("### ")) {
          return (
            <p key={key} className="pt-1 text-[10px] font-bold uppercase tracking-wide text-muted-foreground">
              {line.slice(4)}
            </p>
          );
        }
        if (line.startsWith("## ")) {
          return (
            <p key={key} className="text-xs font-bold text-foreground">
              {line.slice(3)}
            </p>
          );
        }
        if (line.startsWith("# ")) {
          return (
            <p key={key} className="text-xs font-bold text-foreground">
              {line.slice(2)}
            </p>
          );
        }
        if (line.startsWith("- ")) {
          return (
            <p key={key} className="flex gap-1.5 pl-1">
              <span className="text-muted-foreground/60">•</span>
              <span>{line.slice(2)}</span>
            </p>
          );
        }
        if (!line.trim()) return <div key={key} className="h-1" />;
        return <p key={key}>{line}</p>;
      })}
    </div>
  );
}

export function UpdateDialog({ open, info, t, onClose, onOpenUrl }: UpdateDialogProps) {
  const target = info?.download_url || info?.release_url;
  const published = formatPublished(info?.published_at ?? "");

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="flex max-h-[88vh] flex-col gap-0 overflow-hidden rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-lg">
        <DialogHeader className="mb-4 shrink-0 gap-1.5 border-b border-border/30 pb-4">
          <DialogTitle className="text-base font-bold tracking-tight">
            {t.newVersion(info?.latest_version ?? "")}
          </DialogTitle>
          <DialogDescription className="text-xs text-muted-foreground">
            {t.version} {info?.current_version}
            {published ? ` · ${t.publishedAt} ${published}` : ""}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 flex-1 overflow-y-auto -mr-6 pr-6">
          <p className="mb-1.5 text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
            {t.releaseNotes}
          </p>
          {info?.release_notes ? (
            <ReleaseNotes notes={info.release_notes} />
          ) : (
            <p className="rounded-md border border-border/40 bg-muted/10 px-3 py-2 text-[11px] text-muted-foreground/70">
              {t.noReleaseNotes}
            </p>
          )}
        </div>

        <DialogFooter className="mt-4 shrink-0 gap-2 border-t border-border/30 pt-4">
          {info?.release_url ? (
            <Button
              variant="ghost"
              size="sm"
              className="mr-auto"
              onClick={() => onOpenUrl(info.release_url)}
            >
              <ExternalLink className="size-3" />
              {t.viewOnGithub}
            </Button>
          ) : null}
          <Button variant="ghost" size="sm" onClick={onClose}>
            {t.later}
          </Button>
          <Button
            size="sm"
            disabled={!target}
            onClick={() => target && onOpenUrl(target)}
          >
            <Download className="size-3" />
            {info?.download_url ? t.downloadInstaller : t.openReleasePage}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
