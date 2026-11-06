// Everything the page says, in one place.

export const SITE = "https://hams.wusla.co";
export const REPO = "https://github.com/iMohammedArsh/wispher-sh";
/** Redirects (site/public/_redirects) to the fixed-name DMG in the newest GitHub release. */
export const DMG = `${SITE}/download`;
/** The one-line install: curl doesn't mark files as downloaded, so macOS doesn't stop them. */
export const INSTALL_COMMAND = `curl -fsSL ${SITE}/install | bash`;
export const PRIVACY = "/privacy/mac/";

export type PillState =
  | { kind: "recording"; time: string }
  | { kind: "working"; label: string }
  | { kind: "done"; label: string };

/** The dictations that pass through the hero's sentence. The first only shows as the strip's top edge. */
export type Frame =
  | { app: string; pill: PillState; live: string }
  | { app: string; pill: PillState; raw: string; text: string }
  | { app: string; pill: PillState; note: { title: string; points: string[] } };

export const FRAMES: Frame[] = [
  {
    app: "Slack · #design",
    raw: "um pushed the overlay fix uh can you pull and check it on your intel machine",
    text: "Pushed the overlay fix, can you pull and check it on your Intel machine",
    pill: { kind: "done", label: "Pasted into Slack" },
  },
  { app: "Messages · to Sam", live: "running like ten minutes late save me a seat", pill: { kind: "recording", time: "0:04" } },
  {
    app: "Messages · to Sam",
    raw: "Um, running like ten minutes late, uh, save me a seat.",
    text: "Running like ten minutes late, save me a seat",
    pill: { kind: "done", label: "Pasted into Messages" },
  },
  {
    app: "Mail · Re: Launch notes",
    raw: "hi priya thanks for the notes um yeah I'll send the draft by friday",
    text: "Hi Priya,\n\nThanks for the notes. I'll send the draft by Friday.",
    pill: { kind: "done", label: "Pasted into Mail" },
  },
  {
    app: "Hams · Notes · 38 min meeting",
    note: {
      title: "Launch review with the design team",
      points: ["The launch moves from Tuesday to Friday.", "Priya sends the draft notes tonight.", "Sam books the room for the demo at ten."],
    },
    pill: { kind: "done", label: "Note saved" },
  },
];

export const EXAMPLES = [
  { label: "“…new paragraph…”", text: "Hi Priya new paragraph uh thanks for the notes, I'll send the draft tonight." },
  { label: "“Uh, can you grab um milk…”", text: "Uh, can you grab um milk and bread on the way home?" },
  { label: "“…scratch that…”", text: "So, um, Tuesday at ten. Scratch that. Friday at ten." },
];

export const FEATURES = [
  {
    title: "Polite in Mail, brief in Messages",
    body: "Hams sees which app you’re in. Email gets full sentences and a greeting on its own line, chat stays short, and code editors get exactly what you said. You can rewrite each style or pin an app to one.",
    image: { src: "img/styles.jpg", width: 1340, height: 893, max: 640, alt: "The Styles screen in Hams: the Messaging style, its instructions for the AI and a Try it panel." },
  },
  {
    title: "Long talks become notes",
    body: "Record a lecture or a meeting for as long as it runs. Hams writes it down half a minute at a time while you listen, then saves the transcript with a title and the key points.",
    image: { src: "img/notes.jpg", width: 1260, height: 1008, max: 540, alt: "A voice note in Hams: key points and action items above the full transcript." },
  },
  {
    title: "It learns how you spell things",
    body: "If Hams keeps writing “tory” when you mean Tauri, teach it once. Fix a word in History and it suggests the change for next time.",
    image: { src: "img/dictionary.jpg", width: 1680, height: 1260, max: 600, alt: "The Dictionary in Hams: learned spellings waiting for approval, and your words." },
  },
  {
    title: "Every dictation, kept and searchable",
    body: "History shows what was heard and what was written, side by side. Copy one again, rewrite it as an email, or pin it.",
    image: { src: "img/history.jpg", width: 1830, height: 780, max: 760, alt: "A dictation opened in History, showing what was heard and what was written." },
  },
  {
    title: "Your languages, and only yours",
    body: "English, Malayalam, Malay, Hindi, Tamil, Arabic and more, mixed if you like. Tick the ones you speak, so a strong accent can’t turn your English into another language.",
    image: { src: "img/speech.jpg", width: 1420, height: 640, max: 620, alt: "Speech settings in Hams with English and Malayalam ticked as the languages you speak." },
  },
];

/** How long each step of one dictation takes; widths follow the real durations. */
export const STAGES = [
  { id: "speak", grow: 6, time: "As long as you talk", clause: "First, hold the shortcut and speak. A small pill shows Hams is listening." },
  { id: "hear", grow: 0.5, time: "Half a second", clause: "Then Whisper writes it down, on your Mac." },
  { id: "tidy", grow: 1, time: "About a second", clause: "Next, the ums and “scratch that” go, and AI polish runs if it’s on." },
  { id: "paste", grow: 0.1, time: "Instantly", clause: "Last, it’s pasted where your cursor was." },
];

/** Keys are written as lists so each one is drawn as a key cap. */
export const COMMANDS: { kind: string; keys?: string[]; prefix?: string; say?: string; does: string; group?: boolean }[] = [
  { kind: "Press", prefix: "Hold", keys: ["⌘", "⇧", "Space"], does: "Dictate while held" },
  { kind: "Press", say: "Tap it twice", does: "Hands-free until you tap again" },
  { kind: "Press", keys: ["Esc"], does: "Cancel, nothing is pasted" },
  { kind: "Press", keys: ["⌘", "⌥", "⇧", "V"], does: "Paste the last one again" },
  { kind: "Select", prefix: "Hold", keys: ["⌃", "⇧", "Space"], does: "“Make this more formal”", group: true },
  { kind: "Say", say: "“scratch that”", does: "Drops the last sentence" },
  { kind: "Say", say: "“new line”", does: "Starts a new line" },
  { kind: "Say", say: "“new paragraph”", does: "Leaves a blank line" },
  { kind: "Say", say: "A snippet’s name", does: "Inserts your saved text", group: true },
];

export const FAQ: { q: string; a: string[] }[] = [
  {
    q: "How do I install it?",
    a: [
      "The quickest way is the one line for Terminal under “Ready to talk?” below. It downloads Hams, puts it in Applications and opens it, with no warnings.",
      "Or download the DMG and drag Hams into Applications. Because Hams isn’t registered with Apple, macOS says it can’t check the app the first time you open it. Open System Settings → Privacy & Security, scroll down and click Open Anyway. You do this once; updates install from inside Hams.",
    ],
  },
  {
    q: "Why does macOS warn about it?",
    a: [
      "Apple only trusts apps from developers who pay for its yearly program, and Hams is a free project that doesn’t. The warning isn’t about anything Hams does: its code is public on GitHub, and you can build it yourself.",
    ],
  },
  {
    q: "What does it cost?",
    a: ["Nothing. AI polish is optional and uses your own key with Gemini, OpenAI, Anthropic, Groq or OpenRouter, or a free local model through Ollama."],
  },
  {
    q: "Which Macs does it run on?",
    a: ["Any Mac on macOS 12 Monterey or later, Apple Silicon or Intel. On first launch Hams downloads its speech model (466 MB for the recommended one). A Windows version is in progress."],
  },
  {
    q: "What permissions does it ask for?",
    a: ["The microphone, to hear you, and Accessibility, so the shortcut works in every app and Hams can paste at your cursor. The first launch walks you through both."],
  },
  {
    q: "Does anything go online?",
    a: ["Not your voice. Hams downloads the speech model once and checks GitHub for updates, which install only when you click. If you turn on AI polish, the written text goes to the provider you chose."],
  },
];
