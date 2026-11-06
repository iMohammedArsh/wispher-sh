// Hams' local clean-up, ported from src-tauri/src/text/mod.rs (`text::clean` with
// fillers and voice commands on). It runs on every dictation before the optional
// AI polish. clean.test.ts holds the same cases as the Rust tests; keep them in step.

const FILLERS = /\b(?:um+|uh+|uhm+|erm+|hmm+|mhm)\b[,.]?\s*/gi;
const SCRATCH = /[,.!?]?\s*\b(?:scratch that|delete that)\b[,.!?]?\s*/i;
const NEW_PARAGRAPH = /\s*\bnew paragraph\b[,.]?\s*/gi;
const NEW_LINE = /\s*\bnew line\b[,.]?\s*/gi;

const capitalizeFirst = (text: string) => (text ? text[0].toLocaleUpperCase() + text.slice(1) : "");

/** Drop everything after the last sentence end ("Send it. Tomorrow" → "Send it."). */
function dropLastSentence(text: string): string {
  const trimmed = text.trimEnd();
  const index = Math.max(...[".", "!", "?", "\n"].map((c) => trimmed.lastIndexOf(c)));
  return index < 0 ? "" : trimmed.slice(0, index + 1).trimEnd();
}

/** "Send it tomorrow. Scratch that. Send it Friday." → "Send it Friday." */
function applyScratchThat(text: string): string {
  const [first = "", ...rest] = text.split(SCRATCH);
  let result = first;
  for (const part of rest) {
    result = dropLastSentence(result);
    if (!result || /[.!?\n]$/.test(result)) {
      if (result && part) result += " ";
      result += capitalizeFirst(part);
    } else {
      result += " " + part;
    }
  }
  return result;
}

const tidySpaces = (text: string) =>
  text
    .split("\n")
    .map((line) => line.split(/\s+/).filter(Boolean).join(" "))
    .join("\n")
    .trim();

export function clean(raw: string): string {
  let text = raw.trim();
  const first = text.match(/\p{L}/u)?.[0];
  const startedUpper = !!first && first !== first.toLocaleLowerCase();
  text = text.replace(FILLERS, "");
  text = applyScratchThat(text);
  text = text.replace(NEW_PARAGRAPH, "\n\n").replace(NEW_LINE, "\n");
  text = text.split("\n").map(capitalizeFirst).join("\n");
  text = tidySpaces(text);
  return startedUpper ? capitalizeFirst(text) : text;
}
