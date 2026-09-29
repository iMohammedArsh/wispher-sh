import { getCurrentWindow } from "@tauri-apps/api/window";

export const isTauriRuntime = "__TAURI_INTERNALS__" in window;

export function currentWindowLabel() {
  return isTauriRuntime ? getCurrentWindow().label : "main";
}

export function currentWindow() {
  return isTauriRuntime ? getCurrentWindow() : null;
}
