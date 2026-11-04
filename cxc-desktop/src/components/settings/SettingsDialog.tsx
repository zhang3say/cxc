import { useEffect, useState } from "react";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  Cloud,
  Download,
  ExternalLink,
  Loader2,
  RefreshCw,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
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
import { errorText } from "@/lib/utils";
import { isTauri } from "@/lib/api";
import type { Lang, Messages } from "@/lib/i18n";
import type { Config, CustomDirs, Env, SyncConfig } from "@/lib/types";
import { AGENT_KINDS, AGENT_LABELS, ENV_LABELS, emptyDirs } from "@/lib/types";

const GITHUB_TOKEN_URL =
  "https://github.com/settings/tokens/new?scopes=gist&description=CXC%20Config%20Sync";

function openExternal(url: string) {
  if (isTauri) {
    void openUrl(url);
  } else {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

interface SettingsDialogProps {
  open: boolean;
  config: Config;
  envs: Env[];
  t: Messages;
  lang: Lang;
  onLang: (lang: Lang) => void;
  onClose: () => void;
  onSave: (customDirs: CustomDirs) => Promise<void>;
  onSyncSave: (sync: SyncConfig) => Promise<void>;
  onTestGist: (token: string, gistId: string) => Promise<string>;
  onSyncAction: (action: "push" | "pull" | "merge") => Promise<void>;
  version: string;
  onCheckUpdate: () => Promise<void>;
  pathEnv: string;
  onRedetect: () => Promise<void>;
}

export function SettingsDialog({
  open,
  config,
  envs,
  t,
  lang,
  onLang,
  onClose,
  onSave,
  onSyncSave,
  onTestGist,
  onSyncAction,
  version,
  onCheckUpdate,
  pathEnv,
  onRedetect,
}: SettingsDialogProps) {
  const [dirs, setDirs] = useState<CustomDirs>(config.custom_dirs ?? emptyDirs());
  const [sync, setSync] = useState<SyncConfig>(
    config.sync ?? { gist_token: "", gist_id: "" },
  );
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [syncing, setSyncing] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [syncNotice, setSyncNotice] = useState<{
    kind: "success" | "error";
    text: string;
  } | null>(null);

  useEffect(() => {
    if (!open) return;
    setDirs(config.custom_dirs ?? emptyDirs());
    setSync(config.sync ?? { gist_token: "", gist_id: "" });
    setMessage(null);
    setError(null);
    setSyncNotice(null);
    // 只在下拉打开时初始化，同步更新 config 时不要覆盖用户正在编辑的内容。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const updateSync = (patch: Partial<SyncConfig>) => {
    setSync((current) => ({ ...current, ...patch }));
    setSyncNotice(null);
  };

  const visibleEnvs = envs.length ? envs : (["wsl"] as Env[]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave(dirs);
      await onSyncSave(sync);
      setMessage(t.settingsSaved);
    } catch (saveError) {
      setError(errorText(saveError));
    } finally {
      setSaving(false);
    }
  };

  const test = async () => {
    setTesting(true);
    setSyncNotice(null);
    try {
      const login = await onTestGist(sync.gist_token, sync.gist_id);
      setSyncNotice({ kind: "success", text: `GitHub: ${login}` });
    } catch (testError) {
      setSyncNotice({ kind: "error", text: errorText(testError) });
    } finally {
      setTesting(false);
    }
  };

  const runSync = async (action: "push" | "pull" | "merge") => {
    setSyncing(action);
    setSyncNotice(null);
    try {
      await onSyncSave(sync);
      await onSyncAction(action);
      setSync((current) => ({
        ...current,
        last_synced_at: new Date().toISOString(),
      }));
      setSyncNotice({ kind: "success", text: t.syncDone });
    } catch (syncError) {
      setSyncNotice({ kind: "error", text: errorText(syncError) });
    } finally {
      setSyncing(null);
    }
  };

  const gistId = sync.gist_id.trim();
  const gistUrl = /^https?:\/\//.test(gistId)
    ? gistId
    : `https://gist.github.com/${gistId}`;

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="flex max-h-[88vh] flex-col gap-0 overflow-hidden rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-lg">
        <DialogHeader className="mb-4 shrink-0 gap-1.5 border-b border-border/30 pb-4">
          <DialogTitle className="text-base font-bold tracking-tight">
            {t.settingsTitle}
          </DialogTitle>
          <DialogDescription className="text-xs text-muted-foreground">
            {t.customDirsDesc}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 space-y-5 overflow-y-auto pr-1">
          <section className="space-y-2">
            <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              {t.customDirs}
            </Label>
            <div className="space-y-3">
              {AGENT_KINDS.map((kind) => (
                <div key={kind} className="space-y-1.5">
                  <span className="text-[10px] font-bold text-muted-foreground">
                    {AGENT_LABELS[kind]}
                  </span>
                  <div className="space-y-1">
                    {visibleEnvs.map((env) => (
                      <div key={env} className="flex items-center gap-2">
                        <span className="w-14 shrink-0 text-[9px] font-bold uppercase tracking-wider text-muted-foreground/60">
                          {ENV_LABELS[env]}
                        </span>
                        <Input
                          value={dirs[kind]?.[env] ?? ""}
                          onChange={(event) =>
                            setDirs((current) => ({
                              ...current,
                              [kind]: { ...current[kind], [env]: event.target.value },
                            }))
                          }
                          placeholder={
                            env === "windows"
                              ? kind === "opencode"
                                ? "C:\\Users\\user\\AppData\\Roaming\\opencode"
                                : `C:\\Users\\user\\.${kind}`
                              : kind === "opencode"
                                ? "~/.config/opencode"
                                : `~/.${kind}`
                          }
                          className="h-8 rounded-md border-border/30 bg-muted/10 font-mono text-[10px] focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
                        />
                      </div>
                    ))}
                  </div>
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

          <section className="space-y-2.5 border-t border-border/30 pt-4">
            <div className="flex items-center justify-between">
              <Label className="flex items-center gap-1.5 text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                <Cloud className="size-3.5" />
                {t.syncTitle}
              </Label>
              <Button
                size="xs"
                variant="outline"
                onClick={() => openExternal(GITHUB_TOKEN_URL)}
              >
                <ExternalLink className="size-3" />
                {t.createToken}
              </Button>
            </div>
            <p className="text-[10px] leading-relaxed text-muted-foreground/70">
              {t.syncHint}
            </p>

            <div className="space-y-1.5">
              <Label className="text-[10px] font-bold text-muted-foreground">
                {t.gistTokenLabel}
              </Label>
              <Input
                value={sync.gist_token}
                onChange={(event) =>
                  updateSync({ gist_token: event.target.value })
                }
                placeholder="ghp_..."
                type="password"
                className="h-8 rounded-md border-border/30 bg-muted/10 font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
              />
            </div>

            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <Label className="text-[10px] font-bold text-muted-foreground">
                  {t.gistIdLabel}
                </Label>
                {gistId ? (
                  <button
                    type="button"
                    onClick={() => openExternal(gistUrl)}
                    className="flex items-center gap-1 text-[10px] font-semibold text-primary hover:underline"
                  >
                    {t.openGist}
                    <ExternalLink className="size-2.5" />
                  </button>
                ) : null}
              </div>
              <Input
                value={sync.gist_id}
                onChange={(event) =>
                  updateSync({ gist_id: event.target.value })
                }
                placeholder="gist_id"
                className="h-8 rounded-md border-border/30 bg-muted/10 font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
              />
            </div>

            <div className="space-y-1.5">
              <Label className="text-[10px] font-bold text-muted-foreground">
                {t.secretKey}
              </Label>
              <Input
                value={sync.secret_key ?? ""}
                onChange={(event) =>
                  updateSync({ secret_key: event.target.value })
                }
                placeholder={t.secretKeyOptional}
                type="password"
                className="h-8 rounded-md border-border/30 bg-muted/10 font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
              />
            </div>

            <div className="flex flex-wrap items-center gap-1.5">
              <Button size="xs" variant="outline" onClick={() => void test()} disabled={testing}>
                {testing ? (
                  <Loader2 className="size-3 animate-spin" />
                ) : (
                  <RefreshCw className="size-3" />
                )}
                {t.testConnection}
              </Button>
              <Button
                size="xs"
                variant="outline"
                onClick={() => void runSync("push")}
                disabled={Boolean(syncing)}
              >
                {syncing === "push" ? (
                  <Loader2 className="size-3 animate-spin" />
                ) : (
                  <ArrowUpFromLine className="size-3" />
                )}
                {t.push}
              </Button>
              <Button
                size="xs"
                variant="outline"
                onClick={() => void runSync("pull")}
                disabled={Boolean(syncing)}
              >
                {syncing === "pull" ? (
                  <Loader2 className="size-3 animate-spin" />
                ) : (
                  <ArrowDownToLine className="size-3" />
                )}
                {t.pull}
              </Button>
              <Button
                size="xs"
                variant="outline"
                onClick={() => void runSync("merge")}
                disabled={Boolean(syncing)}
              >
                {syncing === "merge" ? (
                  <Loader2 className="size-3 animate-spin" />
                ) : (
                  <RefreshCw className="size-3" />
                )}
                {t.merge}
              </Button>
            </div>
            {syncNotice ? (
              <p
                role="status"
                aria-live="polite"
                className={`rounded-md border px-2.5 py-1.5 text-[10px] ${
                  syncNotice.kind === "success"
                    ? "border-emerald-200 bg-emerald-50 text-emerald-700"
                    : "border-red-200 bg-red-50 text-red-600"
                }`}
              >
                {syncNotice.text}
              </p>
            ) : null}
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
          </section>

          </div>

        <div className="shrink-0 space-y-2">
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
          <DialogFooter className="mt-0 shrink-0 gap-2 border-t border-border/30 pt-4">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={saving}>
            {t.close}
          </Button>
          <Button size="sm" onClick={() => void save()} disabled={saving}>
            {saving ? <Loader2 className="size-3 animate-spin" /> : null}
            {saving ? t.saving : t.save}
          </Button>
        </DialogFooter>
        </div>
      </DialogContent>
    </Dialog>
  );
}
