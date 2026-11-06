// Typed bridge to the Rust commands in src-tauri/src/commands.rs. Command names
// and argument names are written here by hand; the data types are generated.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type * as Rust from "./bindings";

export const inTauri = "__TAURI_INTERNALS__" in window;

// Types the Rust side sends, generated from the serde structs (see bindings.ts).
// The `_Serialize` shapes are what Rust emits; they're also valid to send back.
export type Phase = Rust.Phase;
export type Status = Rust.Status_Serialize;
export type DeliveryMode = Rust.DeliveryMode;
export type AiProvider = Rust.AiProvider;
export type AiSettings = Rust.AiSettings_Serialize;
export type StyleId = Rust.Style;
export type StyleOverride = Rust.StyleOverride_Serialize;
export type AppRule = Rust.AppRule;
export type Settings = Rust.Settings_Serialize;
export type SettingsView = Rust.SettingsView_Serialize;
export type HistoryItem = Rust.HistoryItem;
export type Stats = Rust.Stats;
export type Note = Rust.Note;
export type DictionaryEntry = Rust.DictionaryEntry_Serialize;
export type Snippet = Rust.Snippet;
export type Suggestion = Rust.Suggestion;
export type ModelStatus = Rust.ModelStatus;
export type ModelProgress = Rust.ModelProgress;
export type StyleInfo = Rust.StyleInfo;
export type Permissions = Rust.Permissions;
export type RestyleResult = Rust.RestyleResult;
export type UpdateInfo = Rust.UpdateInfo;
export type UpdateProgress = Rust.UpdateProgress_Serialize;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) {
    // `vite dev` in a plain browser: sample data, so screens can be designed and checked.
    if (import.meta.env.DEV) return (await import("./mock")).mockCall(command, args) as T;
    throw new Error("Hams' native core isn't running (browser preview).");
  }
  return invoke<T>(command, args);
}

export const api = {
  getSettings: () => call<SettingsView>("get_settings"),
  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),
  setApiKey: (provider: AiProvider, key: string) => call<void>("set_api_key", { provider, key }),
  getStatus: () => call<Status>("get_status"),
  toggle: () => call<void>("dictation_toggle"),
  stop: () => call<void>("dictation_stop"),
  noteToggle: () => call<void>("note_toggle"),
  notes: (query = "") => call<Note[]>("notes", { query }),
  noteRename: (id: number, title: string) => call<void>("note_rename", { id, title }),
  noteDelete: (id: number) => call<void>("note_delete", { id }),
  cancel: () => call<void>("dictation_cancel"),
  microphones: () => call<string[]>("list_microphones"),
  models: () => call<ModelStatus[]>("list_models"),
  downloadModel: (id: string) => call<void>("download_model", { id }),
  deleteModel: (id: string) => call<void>("delete_model", { id }),
  history: (query = "", limit?: number) => call<HistoryItem[]>("history", { query, limit }),
  historyRecent: (limit: number) => call<HistoryItem[]>("history_recent", { limit }),
  historyUpdate: (id: number, text: string) => call<number>("history_update", { id, text }),
  historyRestyle: (id: number, style: StyleId) => call<RestyleResult>("history_restyle", { id, style }),
  styleTry: (style: StyleId, text: string) => call<RestyleResult>("style_try", { style, text }),
  historyPin: (id: number, pinned: boolean) => call<void>("history_pin", { id, pinned }),
  historyDelete: (id: number) => call<void>("history_delete", { id }),
  historyClear: () => call<void>("history_clear"),
  stats: () => call<Stats>("stats"),
  dictionary: () => call<DictionaryEntry[]>("dictionary"),
  dictionaryUpsert: (term: string, replacement: string | null) =>
    call<void>("dictionary_upsert", { term, replacement }),
  dictionaryDelete: (id: number) => call<void>("dictionary_delete", { id }),
  suggestions: () => call<Suggestion[]>("suggestions"),
  resolveSuggestion: (id: number, approve: boolean) => call<void>("suggestion_resolve", { id, approve }),
  snippets: () => call<Snippet[]>("snippets"),
  snippetUpsert: (trigger: string, expansion: string) => call<void>("snippet_upsert", { trigger, expansion }),
  snippetDelete: (id: number) => call<void>("snippet_delete", { id }),
  copy: (text: string) => call<void>("copy_text", { text }),
  permissions: () => call<Permissions>("permissions"),
  requestAccessibility: () => call<boolean>("request_accessibility"),
  openPermissionSettings: (kind: string) => call<void>("open_permission_settings", { kind }),
  styles: () => call<StyleInfo[]>("styles"),
  recentApps: () => call<string[]>("recent_apps"),
  defaultAiModel: (provider: AiProvider) => call<string>("default_ai_model", { provider }),
  testAi: () => call<string>("test_ai"),
  autostartEnabled: () => call<boolean>("autostart_enabled"),
  setAutostart: (enabled: boolean) => call<void>("set_autostart", { enabled }),
  appVersion: () => call<string>("app_version"),
  updateCheck: () => call<UpdateInfo | null>("update_check"),
  updateInstall: () => call<void>("update_install"),
};

export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<T>(event, (e) => handler(e.payload));
}

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
