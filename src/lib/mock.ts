// Sample data for `vite dev` in a plain browser (no Tauri). Never bundled in
// production: ipc.ts only imports this behind `import.meta.env.DEV`.
// Totals are computed from the rows, so the screens show figures that add up.
import type { DictionaryEntry, HistoryItem, ModelStatus, Note, Settings, SettingsView, Snippet, Stats, StyleInfo, Suggestion } from "./ipc";

const DAY = 86_400_000;
const now = Date.now();
const at = (daysAgo: number, hour: number, minute = 0) => {
  const d = new Date(now - daysAgo * DAY);
  d.setHours(hour, minute, 0, 0);
  return Math.min(d.getTime(), now - 60_000 * (daysAgo === 0 ? 3 : 0));
};

const settings: Settings = {
  dictate_shortcut: "Meta+Shift+Space",
  command_shortcut: "Ctrl+Shift+Space",
  paste_last_shortcut: "Meta+Alt+Shift+V",
  double_tap_lock: true,
  microphone: null,
  keep_mic_warm: false,
  language: "auto",
  model: "small",
  delivery: "paste_restore",
  remove_fillers: true,
  voice_commands: true,
  live_preview: false,
  app_styles: true,
  style_overrides: { messaging: { prompt: null, polish: null } },
  app_rules: [{ app: "Linear", style: "docs" }],
  learn_from_edits: true,
  learn_from_fields: false,
  ai: { provider: "gemini", model: "", base_url: "", smart_skip_words: 4, timeout_ms: 6000 },
  check_updates: true,
  onboarding_done: true,
};

const rows: [number, number, number, string, string, string][] = [
  [0, 10, 42, "Slack", "messaging", "Pushed the overlay fix, can you pull and check it on your Intel machine"],
  [0, 10, 18, "Mail", "email", "Hi Priya,\n\nThanks for the notes from Tuesday. I've moved the launch review to Thursday at 3 so the design changes can land first. I'll send the build by Wednesday evening.\n\nBest,\nArsh"],
  [0, 9, 55, "Cursor", "code", "rename handleSubmit to commitPrompt and keep the trailing debounce at 300 ms"],
  [0, 9, 31, "Notion", "docs", "The live preview re-transcribes the last twenty seconds once a second. It's display only, so the pasted text still comes from a single pass over the whole recording."],
  [1, 18, 4, "Messages", "messaging", "On my way, ten minutes"],
  [1, 16, 47, "Linear", "docs", "Overlay sometimes appears on the wrong display when the cursor sits on the menu bar of the second monitor. Repro: two displays, cursor on the secondary menu bar, start a dictation."],
  [1, 11, 12, "Slack", "messaging", "Can we move standup to 10:30 tomorrow"],
  [2, 15, 20, "Mail", "email", "Hi team,\n\nThe Malayalam test set is in the shared drive. Small handles mixed speech well; base drops most of the English loanwords.\n\nArsh"],
  [3, 14, 2, "Safari", "default", "best mechanical keyboard with low profile switches for mac"],
  [4, 12, 40, "WhatsApp", "messaging", "Sounds good, see you at 7"],
];
let history: HistoryItem[] = rows.map(([days, h, m, app, style, text], i) => ({
  id: rows.length - i,
  created_at: at(days, h, m),
  text,
  app,
  style,
  delivery: i === 6 ? "copied" : "pasted",
  duration_ms: Math.round(text.split(/\s+/).length * 400),
  words: text.split(/\s+/).length,
  pinned: i === 1,
}));

// 20 weeks of words per day, deterministic, quieter at weekends.
const activity: [string, number][] = [];
for (let d = 0; d < 140; d++) {
  const date = new Date(now - d * DAY);
  const weekend = date.getDay() === 0 || date.getDay() === 6;
  const seed = Math.sin(d * 12.9898) * 43758.5453;
  const r = seed - Math.floor(seed);
  const words = d === 0 ? history.filter((x) => now - x.created_at < DAY && new Date(x.created_at).getDate() === date.getDate()).reduce((a, x) => a + x.words, 0)
    : r < (weekend ? 0.55 : 0.12) ? 0 : Math.round((weekend ? 80 : 260) + r * (weekend ? 220 : 1100));
  if (words > 0) activity.push([date.toLocaleDateString("en-CA"), words]);
}

function stats(): Stats {
  const total_words = activity.reduce((a, [, w]) => a + w, 0);
  const wpm = 146;
  let current = 0;
  for (let d = 0; ; d++) {
    const key = new Date(now - d * DAY).toLocaleDateString("en-CA");
    if (activity.some(([k]) => k === key)) current++;
    else if (d > 0) break;
  }
  return {
    total_words,
    total_dictations: Math.round(total_words / 38),
    words_today: activity[0]?.[0] === new Date(now).toLocaleDateString("en-CA") ? activity[0][1] : 0,
    average_wpm: wpm,
    minutes_saved: Math.round(total_words / 40 - total_words / wpm),
    current_streak: current,
    longest_streak: Math.max(current, 23),
    activity: activity.slice(0, 120),
  };
}

let notes: Note[] = [
  {
    id: 3,
    created_at: at(0, 8, 30),
    duration_ms: 23 * 60_000 + 14_000,
    title: "Launch review prep",
    transcript:
      "Okay, so the main thing for Thursday is that the overlay has to feel finished. Right now the pill jumps when the preview turns on, and that's the first thing people will notice.\n\nSecond, the onboarding. Priya said the permissions step is confusing because macOS only asks for the microphone on first use, so the screen looks stuck. We should say that in words.\n\nThird, pricing isn't part of this release. We keep it free while it's local only. I'll write the release notes tonight and send the build to the testers on Wednesday.",
    summary:
      "- The overlay must feel finished: the pill jumps when live preview turns on\n- Onboarding: explain that macOS asks for the microphone on first use\n- Pricing stays out of this release; Hams stays free while it's local only\nAction items\n- Write the release notes tonight\n- Send the build to testers on Wednesday",
    words: 98,
  },
  {
    id: 2,
    created_at: at(2, 17, 5),
    duration_ms: 51 * 60_000 + 40_000,
    title: "Signals and systems, lecture 9",
    transcript:
      "Today we look at the Fourier transform as the limit of the Fourier series when the period goes to infinity. The spectrum stops being a set of lines and becomes a continuous function.\n\nWe proved the convolution property: convolution in time is multiplication in frequency. That's why filtering is so much cheaper in the frequency domain for long signals.",
    summary:
      "- The Fourier transform is the limit of the Fourier series as the period goes to infinity\n- The spectrum becomes continuous instead of discrete lines\n- Convolution in time equals multiplication in frequency\n- So filtering long signals is cheaper in the frequency domain",
    words: 64,
  },
  {
    id: 1,
    created_at: at(6, 21, 40),
    duration_ms: 4 * 60_000 + 2_000,
    title: "Ideas for the dictionary screen",
    transcript: "Maybe the dictionary should show where each word came from, like whether I added it or it was learned from an edit. And the suggestions should be the first thing on the screen because they need a decision.",
    summary: "",
    words: 41,
  },
];

let dictionary: DictionaryEntry[] = [
  { id: 1, term: "Hams", replacement: null, source: "user" },
  { id: 2, term: "Tauri", replacement: null, source: "user" },
  { id: 3, term: "whisper cpp", replacement: "whisper.cpp", source: "user" },
  { id: 4, term: "priya", replacement: "Priya", source: "learned" },
  { id: 5, term: "Svelte", replacement: null, source: "imported" },
  { id: 6, term: "cursor ai", replacement: "Cursor", source: "learned" },
];
let suggestions: Suggestion[] = [
  { id: 1, heard: "hams app", correct: "Hams app", count: 3 },
  { id: 2, heard: "mala yalam", correct: "Malayalam", count: 1 },
];
let snippets: Snippet[] = [
  { id: 1, trigger: "my email", expansion: "wuslateam@gmail.com" },
  { id: 2, trigger: "meeting link", expansion: "https://meet.example.com/hams-standup" },
  { id: 3, trigger: "sign off", expansion: "Thanks,\nArsh" },
];

const models: ModelStatus[] = [
  { id: "base", file: "ggml-base.bin", label: "Base", size_mb: 148, multilingual: true, note: "Fastest. Good for clear English.", installed: false },
  { id: "small", file: "ggml-small.bin", label: "Small", size_mb: 488, multilingual: true, note: "Recommended balance. Handles Malayalam and mixed speech.", installed: true },
  { id: "large-v3-turbo-q5", file: "ggml-large-v3-turbo-q5_0.bin", label: "Large v3 Turbo", size_mb: 574, multilingual: true, note: "Most accurate. Needs Apple Silicon or a fast CPU.", installed: false },
];

const styles: StyleInfo[] = [
  { id: "messaging", label: "Messaging", polish: true, prompt: "Casual chat message. Keep it short and conversational, sentence case, no sign-off. Omit the final period of a single-sentence message." },
  { id: "email", label: "Email", polish: true, prompt: "Professional email tone with proper paragraphs. Do not invent a greeting or sign-off that wasn't spoken." },
  { id: "code", label: "Code & terminal", polish: false, prompt: "Developer context. Preserve code identifiers, file names, CLI commands and symbols exactly; spoken symbols like 'dash dash' or 'dot' become characters when clearly part of code. Minimal punctuation changes." },
  { id: "docs", label: "Documents", polish: true, prompt: "Well-structured document prose with complete sentences and paragraphs." },
  { id: "default", label: "Default", polish: true, prompt: "Clear, natural writing." },
];

let view: SettingsView = { settings, has_api_key: true, platform: "macos", hotkey_error: null };

export function mockCall(command: string, args: Record<string, unknown> = {}): unknown {
  switch (command) {
    case "get_settings": return structuredClone(view);
    case "save_settings": view = { ...view, settings: args.settings as Settings }; return;
    case "get_status": return { phase: "idle", kind: "dictate", locked: false, started_at: 0, message: null, text: history[0].text, app: null, preview: false };
    case "stats": return stats();
    case "history": {
      const q = String(args.query ?? "").toLowerCase();
      const list = history.filter((h) => !q || h.text.toLowerCase().includes(q));
      return list.sort((a, b) => Number(b.pinned) - Number(a.pinned) || b.created_at - a.created_at).slice(0, Number(args.limit ?? 300));
    }
    case "history_pin": history = history.map((h) => (h.id === args.id ? { ...h, pinned: Boolean(args.pinned) } : h)); return;
    case "history_delete": history = history.filter((h) => h.id !== args.id); return;
    case "history_update": history = history.map((h) => (h.id === args.id ? { ...h, text: String(args.text) } : h)); return 0;
    case "history_clear": history = history.filter((h) => h.pinned); return;
    case "notes": {
      const q = String(args.query ?? "").toLowerCase();
      return notes.filter((n) => !q || `${n.title} ${n.transcript}`.toLowerCase().includes(q));
    }
    case "note_rename": notes = notes.map((n) => (n.id === args.id ? { ...n, title: String(args.title) } : n)); return;
    case "note_delete": notes = notes.filter((n) => n.id !== args.id); return;
    case "dictionary": return dictionary;
    case "dictionary_delete": dictionary = dictionary.filter((d) => d.id !== args.id); return;
    case "dictionary_upsert": dictionary = [...dictionary, { id: Date.now(), term: String(args.term), replacement: (args.replacement as string) ?? null, source: "user" }]; return;
    case "suggestions": return suggestions;
    case "suggestion_resolve": suggestions = suggestions.filter((s) => s.id !== args.id); return;
    case "snippets": return snippets;
    case "snippet_delete": snippets = snippets.filter((s) => s.id !== args.id); return;
    case "snippet_upsert": snippets = [...snippets, { id: Date.now(), trigger: String(args.trigger), expansion: String(args.expansion) }]; return;
    case "list_models": return models;
    case "list_microphones": return ["MacBook Pro Microphone", "AirPods Pro"];
    case "permissions": return { accessibility: true, applicable: true };
    case "styles": return styles;
    case "recent_apps": return ["Slack", "Mail", "Cursor", "Notion", "Linear", "Messages", "Safari"];
    case "default_ai_model": return "gemini-3.1-flash-lite";
    case "autostart_enabled": return true;
    case "app_version": return "0.2.0";
    case "update_check": return { version: "0.3.0", current_version: "0.2.0", notes: "Rewrite past dictations in another style." };
    case "history_restyle": {
      const item = history.find((h) => h.id === args.id);
      const text = String(args.style) === "messaging" ? (item?.text ?? "").replace(/\.$/, "") : (item?.text ?? "");
      history = history.map((h) => (h.id === args.id ? { ...h, text, style: String(args.style) } : h));
      return { text, polished: true };
    }
    default: return;
  }
}
