import { getCurrentWindow } from "@tauri-apps/api/window";

export const isTauriRuntime = "__TAURI_INTERNALS__" in window;

export function currentWindowLabel() {
  if (isTauriRuntime) return getCurrentWindow().label;
  return new URLSearchParams(window.location.search).has("overlay")
    ? "island"
    : "main";
}

export function currentWindow() {
  return isTauriRuntime ? getCurrentWindow() : null;
}
