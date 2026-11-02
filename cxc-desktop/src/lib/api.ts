import { invoke } from "@tauri-apps/api/core";
import { mockInvoke } from "./mock";
import type {
  AgentConfigInput,
  AgentKind,
  Config,
  CustomDirs,
  DetectedAgent,
  Env,
  MergeReport,
  ModelTestOutcome,
  ModelTestResult,
  ProbeReport,
  Protocol,
  SupplierInput,
  SyncConfig,
  UpdateInfo,
} from "./types";

export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) return invoke<T>(command, args);
  return mockInvoke<T>(command, args);
}

export const api = {
  getConfig: () => call<Config>("get_config"),
  getEnvs: () => call<Env[]>("get_envs"),
  detectAgents: () => call<DetectedAgent[]>("detect_agents"),
  getPathEnv: () => call<string>("get_path_env"),

  addSupplier: (input: SupplierInput) => call<Config>("add_supplier", { input }),
  editSupplier: (id: string, input: SupplierInput) =>
    call<Config>("edit_supplier", { id, input }),
  deleteSupplier: (id: string) => call<Config>("delete_supplier", { id }),
  probeSupplier: (id: string) =>
    call<{ config: Config; report: ProbeReport }>("probe_supplier", { id }),
  fetchSupplierModels: (id: string) =>
    call<{ config: Config; models: string[] }>("fetch_supplier_models", { id }),
  testSupplierModel: (
    id: string,
    model: string,
    protocol: Protocol,
    keyId?: string,
  ) =>
    call<{ config: Config; result: ModelTestResult }>("test_supplier_model", {
      id,
      model,
      protocol,
      keyId,
    }),
  testSupplierModels: (id: string, keyId?: string) =>
    call<{ config: Config; outcomes: ModelTestOutcome[] }>("test_supplier_models", {
      id,
      keyId,
    }),

  addBinding: (agent: AgentKind, env: Env, input: AgentConfigInput, activate: boolean) =>
    call<Config>("add_binding", { agent, env, input, activate }),
  editBinding: (agent: AgentKind, env: Env, id: string, input: AgentConfigInput) =>
    call<Config>("edit_binding", { agent, env, id, input }),
  deleteBinding: (agent: AgentKind, env: Env, id: string) =>
    call<Config>("delete_binding", { agent, env, id }),
  activateBinding: (agent: AgentKind, env: Env, id: string) =>
    call<Config>("activate_binding", { agent, env, id }),
  rewriteAgent: (agent: AgentKind, env: Env) =>
    call<Config>("rewrite_agent", { agent, env }),

  saveSettings: (customDirs: CustomDirs) =>
    call<Config>("save_settings", { customDirs }),

  getSyncConfig: () => call<SyncConfig>("get_sync_config"),
  saveSyncConfig: (sync: SyncConfig) => call<Config>("save_sync_config", { sync }),
  testGistConnection: (token: string, gistId?: string) =>
    call<string>("test_gist_connection", { token, gistId }),
  syncPush: () => call<{ config: Config; report: MergeReport }>("sync_push"),
  syncPull: () => call<{ config: Config; report: MergeReport }>("sync_pull"),
  syncMerge: () => call<{ config: Config; report: MergeReport }>("sync_merge"),

  getAppVersion: () => call<string>("get_app_version"),
  checkUpdate: () => call<UpdateInfo>("check_update"),
};
