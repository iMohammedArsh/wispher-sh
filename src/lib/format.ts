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

/** 12,345 with the locale's separators. */
export function groupedNumber(n: number): string {
  return n.toLocaleString();
}

/** Elapsed time as m:ss, or h:mm:ss past an hour. */
export function duration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

/** 312 minutes → [[5, "h"], [12, "m"]]; 0 → [[0, "m"]]. */
export function timeParts(minutes: number): [number, string][] {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (!h) return [[m, "m"]];
  return m ? [[h, "h"], [m, "m"]] : [[h, "h"]];
}

/** A long note's length in words: "23 min", "1 h 4 min", "40 s". */
export function lengthLabel(ms: number): string {
  const minutes = Math.round(ms / 60_000);
  if (ms < 60_000) return `${Math.max(1, Math.round(ms / 1000))} s`;
  return minutes >= 60 ? `${Math.floor(minutes / 60)} h ${minutes % 60} min` : `${minutes} min`;
}

/** "Today", "Yesterday", "Monday 3 November". */
export function dayLabel(ms: number, now = Date.now()): string {
  const day = (t: number) => new Date(t).toDateString();
  if (day(ms) === day(now)) return "Today";
  if (day(ms) === day(now - 86_400_000)) return "Yesterday";
  const sameYear = new Date(ms).getFullYear() === new Date(now).getFullYear();
  return new Date(ms).toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric", year: sameYear ? undefined : "numeric" });
}

export function clockTime(ms: number): string {
  return new Date(ms).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

export interface CalendarDay {
  date: Date;
  words: number;
  level: 0 | 1 | 2 | 3 | 4;
  inRange: boolean;
  isToday: boolean;
}

/**
 * Columns of Monday-first weeks ending with the current week, for the
 * activity calendar. `activity` holds ("YYYY-MM-DD" local, words) pairs.
 * Levels split the active days into quartiles, so a few huge days don't
 * wash out the rest.
 */
export function calendarWeeks(activity: [string, number][], weeks: number, now = new Date()): CalendarDay[][] {
  const words = new Map(activity);
  const key = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const mondayOffset = (today.getDay() + 6) % 7;
  const start = new Date(today);
  start.setDate(today.getDate() - mondayOffset - (weeks - 1) * 7);

  const active = activity.map(([, w]) => w).filter((w) => w > 0).sort((a, b) => a - b);
  const q = (p: number) => active[Math.floor(p * (active.length - 1))] ?? 0;
  const [q1, q2, q3] = [q(0.25), q(0.5), q(0.75)];
  const level = (w: number): CalendarDay["level"] => (w <= 0 ? 0 : w <= q1 ? 1 : w <= q2 ? 2 : w <= q3 ? 3 : 4);

  const columns: CalendarDay[][] = [];
  for (let c = 0; c < weeks; c++) {
    const column: CalendarDay[] = [];
    for (let r = 0; r < 7; r++) {
      const date = new Date(start);
      date.setDate(start.getDate() + c * 7 + r);
      const inRange = date <= today;
      const w = inRange ? (words.get(key(date)) ?? 0) : 0;
      column.push({ date, words: w, level: level(w), inRange, isToday: date.getTime() === today.getTime() });
    }
    columns.push(column);
  }
  return columns;
}
