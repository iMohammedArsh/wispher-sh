// Word-level diff between what Whisper heard and what Hams wrote, for the
// Heard / Written pair. Words are compared without case or punctuation, so
// "um see you" → "See you." reads as one dropped word, not three changes.

export type Run = { kind: "keep" | "drop" | "add"; words: string[] };

export interface WordDiff {
  /** The heard text as runs of kept and dropped words. */
  heard: Run[];
  /** The written text as runs of kept and added words. */
  written: Run[];
  /** Words dropped from what was heard (fillers, "scratch that", rewrites). */
  dropped: number;
  /** Words added or changed in what was written. */
  added: number;
}

/** Words, with each line break kept as its own "\n" token so paragraphs survive. */
const tokens = (text: string) => text.replace(/\r?\n/g, " \n ").split(/[ \t]+/).filter(Boolean);

/** Compare words without case or surrounding punctuation. */
export function wordKey(word: string): string {
  return word.toLocaleLowerCase().replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, "");
}

function push(runs: Run[], kind: Run["kind"], word: string) {
  const last = runs[runs.length - 1];
  if (last?.kind === kind) last.words.push(word);
  else runs.push({ kind, words: [word] });
}

export function diffWords(heardText: string, writtenText: string): WordDiff {
  const a = tokens(heardText);
  const b = tokens(writtenText);
  const ka = a.map(wordKey);
  const kb = b.map(wordKey);
  // Longest common subsequence table, filled from the end.
  const n = a.length;
  const m = b.length;
  const lcs: Uint16Array[] = Array.from({ length: n + 1 }, () => new Uint16Array(m + 1));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      lcs[i][j] = ka[i] === kb[j] && ka[i] !== "" ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
    }
  }
  const heard: Run[] = [];
  const written: Run[] = [];
  let dropped = 0;
  let added = 0;
  let i = 0;
  let j = 0;
  while (i < n || j < m) {
    if (i < n && j < m && ka[i] === kb[j] && ka[i] !== "") {
      push(heard, "keep", a[i++]);
      push(written, "keep", b[j++]);
    } else if (j < m && (i >= n || lcs[i][j + 1] >= lcs[i + 1][j])) {
      // A token that is only punctuation ("—", "-") isn't a real addition.
      if (kb[j] === "") push(written, "keep", b[j++]);
      else {
        push(written, "add", b[j++]);
        added++;
      }
    } else {
      if (ka[i] === "") push(heard, "keep", a[i++]);
      else {
        push(heard, "drop", a[i++]);
        dropped++;
      }
    }
  }
  return { heard, written, dropped, added };
}

/** "2 words removed · 1 fixed", or "" when nothing changed but the punctuation. */
export function changeSummary(diff: WordDiff): string {
  const parts: string[] = [];
  if (diff.dropped) parts.push(`${diff.dropped} ${diff.dropped === 1 ? "word" : "words"} removed`);
  if (diff.added) parts.push(`${diff.added} ${diff.added === 1 ? "word" : "words"} changed or added`);
  return parts.join(" · ");
}
