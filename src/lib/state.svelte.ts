// One app-wide store, fed by Rust events. Components read it directly.
import { api, errorText, on, type Settings, type SettingsView, type Status, type UpdateInfo, type UpdateProgress } from "./ipc";

const idle: Status = {
  phase: "idle",
  kind: "dictate",
  locked: false,
  started_at: 0,
  message: null,
  text: null,
  app: null,
  preview: false,
};

export type Route = "home" | "history" | "notes" | "dictionary" | "snippets" | "styles" | "settings";
export type SettingsArea = "general" | "shortcuts" | "speech" | "output" | "ai" | "learning" | "updates" | "permissions";

class AppStore {
  status = $state<Status>(idle);
  view = $state<SettingsView | null>(null);
  route = $state<Route>("home");
  /** Which Settings view is open. */
  settingsArea = $state<SettingsArea>("general");
  /** Learned corrections waiting for approval (the Dictionary nav count). */
  pendingSuggestions = $state(0);
  /** Bumped whenever history, stats or the dictionary change on disk. */
  dataVersion = $state(0);
  toast = $state<{ text: string; kind: "ok" | "error" } | null>(null);
  /** A newer Hams, found by the background check or "Check now". */
  update = $state<UpdateInfo | null>(null);
  /** Download progress while an update installs, 0..1 (null when the size is unknown). */
  updateProgress = $state<number | null | undefined>(undefined);
  #toastTimer: ReturnType<typeof setTimeout> | undefined;

  get settings(): Settings | null {
    return this.view?.settings ?? null;
  }

  get platform() {
    return this.view?.platform ?? (navigator.userAgent.includes("Mac") ? "macos" : "windows");
  }

  async init() {
    await on<Status>("status", (s) => (this.status = s));
    await on("data-changed", () => {
      this.dataVersion++;
      void this.refreshSuggestions();
    });
    await on<UpdateInfo>("update-available", (u) => (this.update = u));
    await on<UpdateProgress>("update-progress", (p) => (this.updateProgress = p.total ? p.downloaded / p.total : null));
    try {
      this.view = await api.getSettings();
      this.status = await api.getStatus();
      void this.refreshSuggestions();
    } catch (error) {
      this.notify(errorText(error), "error");
    }
  }

  openSettings(area: SettingsArea) {
    this.settingsArea = area;
    this.route = "settings";
  }

  async refreshSuggestions() {
    try {
      this.pendingSuggestions = (await api.suggestions()).length;
    } catch {
      // Not fatal: the count just stays as it was.
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

  /** Download, verify and install the update; Hams restarts when it's done. */
  async installUpdate() {
    this.updateProgress = 0;
    try {
      await api.updateInstall();
    } catch (error) {
      this.updateProgress = undefined;
      this.notify(errorText(error), "error");
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
