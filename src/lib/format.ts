export function relativeTime(ms: number, now = Date.now()): string {
  const seconds = Math.round((now - ms) / 1000);
  if (seconds < 45) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.round(hours / 24);
  if (days < 7) return `${days} d ago`;
  return new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

/// "Meta+Shift+Space" → "⌘ ⇧ Space" on macOS, "Ctrl + Shift + Space" on Windows.
export function shortcutLabel(shortcut: string, platform: "macos" | "windows"): string {
  const mac: Record<string, string> = {
    meta: "⌘", cmd: "⌘", command: "⌘", ctrl: "⌃", control: "⌃", alt: "⌥", option: "⌥", shift: "⇧",
    fn: "Fn", rightalt: "Right ⌥", rightmeta: "Right ⌘", rightctrl: "Right ⌃", rightshift: "Right ⇧",
  };
  const win: Record<string, string> = {
    meta: "Win", cmd: "Win", ctrl: "Ctrl", control: "Ctrl", alt: "Alt", option: "Alt", shift: "Shift",
    rightalt: "Right Alt", rightmeta: "Right Win", rightctrl: "Right Ctrl", rightshift: "Right Shift",
  };
  const names = platform === "macos" ? mac : win;
  const parts = shortcut
    .split("+")
    .map((p) => p.trim())
    .filter(Boolean)
    .map((p) => names[p.toLowerCase().replace(/[\s_-]/g, "")] ?? p);
  return parts.join(platform === "macos" ? " " : " + ");
}

export function compactNumber(n: number): string {
  return n >= 10_000 ? `${(n / 1000).toFixed(n >= 100_000 ? 0 : 1)}k` : n.toLocaleString();
}
