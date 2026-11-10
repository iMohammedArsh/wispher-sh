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

/** History, set as a board: what came in today, one dictation per row. `open` is the row shown open at load. */
export type BoardRow = { app: string; time: string; words: number; style: string; status: "Pasted" | "Copied" | "Note"; raw: string; text: string };
export const BOARD_OPEN = 3;
export const BOARD: BoardRow[] = [
  { app: "Linear", time: "8:52", words: 7, style: "Message", status: "Pasted", raw: "overlay jumps uh when live preview turns on", text: "Overlay jumps when live preview turns on" },
  { app: "Slack", time: "9:26", words: 14, style: "Message", status: "Pasted", raw: "um pushed the overlay fix uh can you pull and check it on your intel machine", text: "Pushed the overlay fix, can you pull and check it on your Intel machine" },
  { app: "Cursor", time: "9:39", words: 14, style: "Code", status: "Pasted", raw: "rename handle submit to commit prompt and keep the trailing debounce at 300 ms", text: "rename handle submit to commit prompt and keep the trailing debounce at 300 ms" },
  { app: "Mail", time: "10:04", words: 11, style: "Email", status: "Pasted", raw: "hi priya thanks for the notes um yeah I'll send the draft by friday", text: "Hi Priya,\n\nThanks for the notes. I'll send the draft by Friday." },
  { app: "Messages", time: "11:17", words: 9, style: "Message", status: "Pasted", raw: "Um, running like ten minutes late, uh, save me a seat.", text: "Running like ten minutes late, save me a seat" },
  { app: "Notion", time: "12:30", words: 16, style: "Document", status: "Pasted", raw: "the launch moves from tuesday to friday so um the build gets two more days of testing", text: "The launch moves from Tuesday to Friday, so the build gets two more days of testing." },
  { app: "Hams", time: "13:02", words: 3, style: "Message", status: "Copied", raw: "uh tauri spelled t a u r i", text: "Tauri spelled T-A-U-R-I" },
  { app: "Design review", time: "14:00", words: 4212, style: "Note", status: "Note", raw: "okay so the main thing for thursday is that the overlay has to feel finished", text: "Okay, so the main thing for Thursday is that the overlay has to feel finished." },
  { app: "WhatsApp", time: "18:45", words: 9, style: "Message", status: "Pasted", raw: "landing at six um no need to pick me up", text: "Landing at six, no need to pick me up" },
];

/** What you can switch on, each shown working when it's on. */
export type Switch = {
  id: string;
  says: string;
  then: string;
  on: boolean;
  show: { raw: string; text: string } | { src: string; width: number; height: number; alt: string; focus: string };
};
export const SWITCHES: Switch[] = [
  {
    id: "polish",
    says: "Polish it with AI",
    then: "One call to the provider you pick, with your own key. Grammar and punctuation fixed, your words kept.",
    on: false,
    show: { raw: "so um i think we should uh move the launch to friday because the the build isn't ready", text: "I think we should move the launch to Friday, because the build isn't ready." },
  },
  {
    id: "styles",
    says: "Write the way each app expects",
    then: "Email gets a greeting on its own line, chat stays short, and code editors get exactly what you said.",
    on: true,
    show: { src: "img/styles.jpg", width: 1340, height: 893, alt: "The Styles screen in Hams: the Messaging style and its instructions for the AI.", focus: "50% 20%" },
  },
  {
    id: "learn",
    says: "Learn how I spell names",
    then: "Teach “Tauri” once and Hams stops writing “tory”. Fix a word in History and it suggests the change.",
    on: false,
    show: { src: "img/dictionary.jpg", width: 1680, height: 1260, alt: "The Dictionary in Hams: learned spellings waiting for approval, and your words.", focus: "50% 0%" },
  },
  {
    id: "notes",
    says: "Turn long talks into notes",
    then: "A 38-minute meeting comes back as a title, the key points and the whole transcript.",
    on: true,
    show: { src: "img/notes.jpg", width: 1260, height: 1008, alt: "A voice note in Hams: key points and action items above the full transcript.", focus: "50% 0%" },
  },
  {
    id: "languages",
    says: "Only hear the languages I speak",
    then: "English, Malayalam, Malay, Hindi, Tamil, Arabic and more. Tick yours, so an accent can’t turn English into Malay.",
    on: false,
    show: { src: "img/speech.jpg", width: 1420, height: 640, alt: "Speech settings in Hams with English and Malayalam ticked as the languages you speak.", focus: "50% 40%" },
  },
];

/** Everything Hams sends over the network, and when. */
export const LEDGER = [
  { what: "Your voice", where: "Never leaves this Mac", local: true },
  { what: "Speech model", where: "Downloaded once, 466 MB", local: false },
  { what: "Update check", where: "GitHub, every 6 hours", local: false },
  { what: "Account", where: "There isn’t one", local: true },
];

/** One dictation, step by step, with how long each step takes. */
export const STEPS: { time: string; says: string; body: string; pill: PillState; window: "empty" | "heard" | "diff" | "written" }[] = [
  { time: "As long as you talk", says: "Hold the shortcut and talk", body: "A small pill at the bottom of the screen shows Hams is listening. It never takes the focus from your app.", pill: { kind: "recording", time: "0:04" }, window: "empty" },
  { time: "Half a second", says: "Whisper writes it down", body: "The speech model runs inside Hams, on your Mac’s own chip. Nothing is uploaded.", pill: { kind: "working", label: "Hearing" }, window: "heard" },
  { time: "About a second", says: "The ums go", body: "Fillers and “scratch that” are taken out, and AI polish runs if you turned it on.", pill: { kind: "working", label: "Polishing" }, window: "diff" },
  { time: "Instantly", says: "It lands at your cursor", body: "Hams pastes into the app you were in, then puts your clipboard back as it was.", pill: { kind: "done", label: "Pasted into Notes" }, window: "written" },
];
export const STEP_TEXT = {
  raw: "so um the launch moves to friday uh scratch that the launch moves to thursday",
  text: "The launch moves to Thursday.",
};

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
