// One app-wide store, fed by Rust events. Components read it directly.
import { api, errorText, on, type Settings, type SettingsView, type Status } from "./ipc";

const idle: Status = {
  phase: "idle",
  kind: "dictate",
  locked: false,
  started_at: 0,
  message: null,
  text: null,
  app: null,
};

export type Route = "home" | "history" | "dictionary" | "snippets" | "styles" | "settings";

class AppStore {
  status = $state<Status>(idle);
  view = $state<SettingsView | null>(null);
  route = $state<Route>("home");
  /** Bumped whenever history, stats or the dictionary change on disk. */
  dataVersion = $state(0);
  toast = $state<{ text: string; kind: "ok" | "error" } | null>(null);
  #toastTimer: ReturnType<typeof setTimeout> | undefined;

  get settings(): Settings | null {
    return this.view?.settings ?? null;
  }

  get platform() {
    return this.view?.platform ?? (navigator.userAgent.includes("Mac") ? "macos" : "windows");
  }

  async init() {
    await on<Status>("status", (s) => (this.status = s));
    await on("data-changed", () => this.dataVersion++);
    try {
      this.view = await api.getSettings();
      this.status = await api.getStatus();
    } catch (error) {
      this.notify(errorText(error), "error");
    }
  }

  async reloadSettings() {
    this.view = await api.getSettings();
  }

  /** Save a partial settings change. Returns false (and shows why) if Rust rejects it. */
  async save(patch: Partial<Settings>): Promise<boolean> {
    if (!this.view) return false;
    const next = { ...this.view.settings, ...patch };
    try {
      await api.saveSettings(next);
      this.view = { ...this.view, settings: next };
      return true;
    } catch (error) {
      this.notify(errorText(error), "error");
      return false;
    }
  }

  notify(text: string, kind: "ok" | "error" = "ok") {
    this.toast = { text, kind };
    clearTimeout(this.#toastTimer);
    this.#toastTimer = setTimeout(() => (this.toast = null), kind === "error" ? 6000 : 2500);
  }

  async run(action: () => Promise<unknown>, success?: string) {
    try {
      await action();
      if (success) this.notify(success);
    } catch (error) {
      this.notify(errorText(error), "error");
    }
  }
}

export const app = new AppStore();
