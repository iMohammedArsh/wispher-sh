// Typed bridge to the Rust commands in src-tauri/src/commands.rs.
// Keep these types in sync with the serde structs on the Rust side.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const inTauri = "__TAURI_INTERNALS__" in window;

export type Phase = "idle" | "recording" | "transcribing" | "polishing" | "delivering" | "done" | "error";

export interface Status {
  phase: Phase;
  kind: "dictate" | "command";
  locked: boolean;
  started_at: number;
  message: string | null;
  text: string | null;
  app: string | null;
}

export type DeliveryMode = "paste_restore" | "paste" | "copy_only";
export type AiProvider = "off" | "gemini" | "open_ai" | "groq" | "open_router" | "anthropic" | "ollama" | "custom";

export interface AiSettings {
  provider: AiProvider;
  model: string;
  base_url: string;
  smart_skip_words: number;
  timeout_ms: number;
}

export interface Settings {
  dictate_shortcut: string;
  command_shortcut: string;
  paste_last_shortcut: string;
  double_tap_lock: boolean;
  microphone: string | null;
  language: string;
  model: string;
  delivery: DeliveryMode;
  remove_fillers: boolean;
  voice_commands: boolean;
  app_styles: boolean;
  learn_from_edits: boolean;
  ai: AiSettings;
  onboarding_done: boolean;
}

export interface SettingsView {
  settings: Settings;
  has_api_key: boolean;
  platform: "macos" | "windows";
  hotkey_error: string | null;
}

export interface HistoryItem {
  id: number;
  created_at: number;
  text: string;
  app: string;
  style: string;
  delivery: string;
  duration_ms: number;
  words: number;
  pinned: boolean;
}

export interface Stats {
  total_words: number;
  total_dictations: number;
  words_today: number;
  average_wpm: number;
  minutes_saved: number;
  current_streak: number;
  longest_streak: number;
  activity: [string, number][];
}

export interface DictionaryEntry {
  id: number;
  term: string;
  replacement: string | null;
  source: string;
}

export interface Snippet {
  id: number;
  trigger: string;
  expansion: string;
}

export interface Suggestion {
  id: number;
  heard: string;
  correct: string;
  count: number;
}

export interface ModelStatus {
  id: string;
  file: string;
  label: string;
  size_mb: number;
  multilingual: boolean;
  note: string;
  installed: boolean;
}

export interface StyleInfo {
  id: string;
  label: string;
  prompt: string;
  polish: boolean;
}

export interface Permissions {
  accessibility: boolean;
  applicable: boolean;
}

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject(new Error("Hams' native core isn't running (browser preview)."));
  return invoke<T>(command, args);
}

export const api = {
  getSettings: () => call<SettingsView>("get_settings"),
  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),
  setApiKey: (provider: AiProvider, key: string) => call<void>("set_api_key", { provider, key }),
  getStatus: () => call<Status>("get_status"),
  toggle: () => call<void>("dictation_toggle"),
  stop: () => call<void>("dictation_stop"),
  cancel: () => call<void>("dictation_cancel"),
  microphones: () => call<string[]>("list_microphones"),
  models: () => call<ModelStatus[]>("list_models"),
  downloadModel: (id: string) => call<void>("download_model", { id }),
  deleteModel: (id: string) => call<void>("delete_model", { id }),
  history: (query = "", limit?: number) => call<HistoryItem[]>("history", { query, limit }),
  historyUpdate: (id: number, text: string) => call<number>("history_update", { id, text }),
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
  defaultAiModel: (provider: AiProvider) => call<string>("default_ai_model", { provider }),
  testAi: () => call<string>("test_ai"),
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
