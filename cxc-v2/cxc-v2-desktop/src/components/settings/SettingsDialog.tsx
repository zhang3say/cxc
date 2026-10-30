import { useEffect, useState } from "react";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  Check,
  Cloud,
  Download,
  Loader2,
  RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { CxcSelect } from "@/components/ui/select";
import type { Lang, Messages } from "@/lib/i18n";
import type { Config, CustomDirs, Source, SyncConfig } from "@/lib/types";
import { AGENT_KINDS, AGENT_LABELS } from "@/lib/types";

interface SettingsDialogProps {
  open: boolean;
  config: Config;
  t: Messages;
  lang: Lang;
  onLang: (lang: Lang) => void;
  onClose: () => void;
  onSave: (source: Source, customDirs: CustomDirs) => Promise<void>;
  onSyncSave: (sync: SyncConfig) => Promise<void>;
  onTestGist: (token: string, gistId: string) => Promise<string>;
  onSyncAction: (action: "push" | "pull" | "merge") => Promise<void>;
  onImport: () => Promise<void>;
  version: string;
  onCheckUpdate: () => Promise<void>;
  pathEnv: string;
  onRedetect: () => Promise<void>;
}

export function SettingsDialog({
  open,
  config,
  t,
  lang,
  onLang,
  onClose,
  onSave,
  onSyncSave,
  onTestGist,
  onSyncAction,
  onImport,
  version,
  onCheckUpdate,
  pathEnv,
  onRedetect,
}: SettingsDialogProps) {
  const [source, setSource] = useState<Source>(config.source);
  const [dirs, setDirs] = useState<CustomDirs>(config.custom_dirs);
  const [sync, setSync] = useState<SyncConfig>(
    config.sync ?? { gist_token: "", gist_id: "" },
  );
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [syncing, setSyncing] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setSource(config.source);
    setDirs(
      config.custom_dirs ?? { codex: "", claude: "", grok: "", opencode: "" },
    );
    setSync(config.sync ?? { gist_token: "", gist_id: "" });
    setMessage(null);
    setError(null);
  }, [open, config]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave(source, dirs);
      await onSyncSave(sync);
      setMessage(t.settingsSaved);
    } catch (saveError) {
      setError(String(saveError));
    } finally {
      setSaving(false);
    }
  };

  const test = async () => {
    setTesting(true);
    setError(null);
    setMessage(null);
    try {
      const login = await onTestGist(sync.gist_token, sync.gist_id);
      setMessage(`GitHub: ${login}`);
    } catch (testError) {
      setError(String(testError));
    } finally {
      setTesting(false);
    }
  };

  const runSync = async (action: "push" | "pull" | "merge") => {
    setSyncing(action);
    setError(null);
    setMessage(null);
    try {
      await onSyncSave(sync);
      await onSyncAction(action);
      setMessage(t.syncDone);
    } catch (syncError) {
      setError(String(syncError));
    } finally {
      setSyncing(null);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="flex max-h-[88vh] flex-col gap-0 overflow-hidden rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-lg">
        <DialogHeader className="mb-4 shrink-0 gap-1.5 border-b border-border/30 pb-4">
          <DialogTitle className="text-base font-bold tracking-tight">{t.settingsTitle}</DialogTitle>
          <DialogDescription className="text-xs text-muted-foreground">
            {t.sourceDesc}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 space-y-5 overflow-y-auto pr-1">
          <section className="space-y-2">
            <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              {t.sourceTitle}
            </Label>
            <div className="grid grid-cols-2 gap-2.5">
              {(["app", "wsl"] as Source[]).map((item) => (
                <button
                  key={item}
                  type="button"
                  onClick={() => setSource(item)}
                  className={`flex items-center justify-between rounded-xl border p-3.5 text-left transition-all active:scale-[0.98] ${
                    source === item
                      ? "border-primary/60 bg-primary/[0.04] text-primary shadow-sm shadow-primary/5"
                      : "border-border/30 bg-muted/10 text-foreground hover:bg-muted/20"
                  }`}
                >
                  <div>
                    <span className="block text-xs font-bold">
                      {item === "app" ? t.sourceApp : t.sourceWsl}
                    </span>
                    <span className="mt-0.5 block text-[10px] text-muted-foreground">
                      {item === "app" ? "Windows / 本机" : "WSL / Linux"}
                    </span>
                  </div>
                  {source === item ? <Check className="size-4 shrink-0 text-primary" /> : null}
                </button>
              ))}
            </div>
          </section>

          <section className="space-y-2">
            <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              {t.customDirs}
            </Label>
            <p className="text-[10px] leading-normal text-muted-foreground/70">
              {t.customDirsDesc}
            </p>
            <div className="space-y-2">
              {AGENT_KINDS.map((kind) => (
                <div key={kind} className="space-y-1">
                  <span className="text-[10px] font-bold text-muted-foreground">
                    {AGENT_LABELS[kind]}
                  </span>
                  <Input
                    value={dirs[kind]}
                    onChange={(event) =>
                      setDirs((current) => ({ ...current, [kind]: event.target.value }))
                    }
                    placeholder={
                      kind === "codex"
                        ? "\\\\wsl.localhost\\Ubuntu\\home\\user\\.codex"
                        : kind === "opencode"
                          ? "~/.config/opencode"
                          : `~/.${kind}`
                    }
                    className="h-8 rounded-md border-border/30 bg-muted/10 font-mono text-[10px] focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
                  />
                </div>
              ))}
            </div>
            <div className="flex items-center justify-between pt-1">
              <span className="text-[10px] font-bold text-muted-foreground">{t.pathEnv}</span>
              <Button size="xs" variant="outline" onClick={() => void onRedetect()}>
                <RefreshCw className="size-3" />
                {t.redetect}
              </Button>
            </div>
            <pre className="max-h-16 overflow-auto rounded-md border border-border/30 bg-muted/10 p-2 font-mono text-[9px] leading-relaxed text-muted-foreground/80">
              {pathEnv || "—"}
            </pre>
          </section>

          <section className="space-y-2 border-t border-border/30 pt-4">
            <Label className="flex items-center gap-1.5 text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              <Cloud className="size-3.5" />
              {t.syncTitle}
            </Label>
            <div className="grid grid-cols-2 gap-2">
              <Input
                value={sync.gist_token}
                onChange={(event) =>
                  setSync((current) => ({ ...current, gist_token: event.target.value }))
                }
                placeholder={t.gistToken}
                type="password"
                className="h-8 rounded-md border-border/30 bg-muted/10 font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
              />
              <Input
                value={sync.gist_id}
                onChange={(event) =>
                  setSync((current) => ({ ...current, gist_id: event.target.value }))
                }
                placeholder={t.gistId}
                className="h-8 rounded-md border-border/30 bg-muted/10 font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
              />
            </div>
            <Input
              value={sync.secret_key ?? ""}
              onChange={(event) =>
                setSync((current) => ({ ...current, secret_key: event.target.value }))
              }
              placeholder={t.secretKey}
              type="password"
              className="h-8 rounded-md border-border/30 bg-muted/10 font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
            />
            <div className="flex flex-wrap items-center gap-1.5">
              <Button size="xs" variant="outline" onClick={() => void test()} disabled={testing}>
                {testing ? <Loader2 className="size-3 animate-spin" /> : <RefreshCw className="size-3" />}
                {t.testConnection}
              </Button>
              <Button size="xs" variant="outline" onClick={() => void runSync("push")} disabled={Boolean(syncing)}>
                {syncing === "push" ? <Loader2 className="size-3 animate-spin" /> : <ArrowUpFromLine className="size-3" />}
                {t.push}
              </Button>
              <Button size="xs" variant="outline" onClick={() => void runSync("pull")} disabled={Boolean(syncing)}>
                {syncing === "pull" ? <Loader2 className="size-3 animate-spin" /> : <ArrowDownToLine className="size-3" />}
                {t.pull}
              </Button>
              <Button size="xs" variant="outline" onClick={() => void runSync("merge")} disabled={Boolean(syncing)}>
                {syncing === "merge" ? <Loader2 className="size-3 animate-spin" /> : <RefreshCw className="size-3" />}
                {t.merge}
              </Button>
            </div>
            {sync.last_synced_at ? (
              <p className="text-[10px] text-muted-foreground/60">
                {t.lastSynced}: {new Date(sync.last_synced_at).toLocaleString()}
              </p>
            ) : null}
          </section>

          <section className="space-y-2 border-t border-border/30 pt-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-[11px] font-bold text-foreground/80">{t.about}</p>
                <p className="text-[10px] text-muted-foreground">
                  {t.version} {version}
                </p>
              </div>
              <Button size="xs" variant="outline" onClick={() => void onCheckUpdate()}>
                <Download className="size-3" />
                {t.update}
              </Button>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[11px] font-bold text-foreground/80">{t.language}</span>
              <div className="w-36">
                <CxcSelect
                  value={lang}
                  onValueChange={(value) => onLang(value as Lang)}
                  options={[
                    { value: "zh", label: "简体中文" },
                    { value: "en", label: "English" },
                  ]}
                  size="xs"
                />
              </div>
            </div>
            <Button size="xs" variant="outline" onClick={() => void onImport()}>
              <RefreshCw className="size-3" />
              {t.importLegacy}
            </Button>
          </section>

          {message ? (
            <p className="rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-[11px] text-emerald-700">
              {message}
            </p>
          ) : null}
          {error ? (
            <p className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-[11px] text-red-600">
              {error}
            </p>
          ) : null}
        </div>

        <DialogFooter className="mt-4 shrink-0 gap-2 border-t border-border/30 pt-4">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={saving}>
            {t.close}
          </Button>
          <Button size="sm" onClick={() => void save()} disabled={saving}>
            {saving ? <Loader2 className="size-3 animate-spin" /> : null}
            {saving ? t.saving : t.save}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
