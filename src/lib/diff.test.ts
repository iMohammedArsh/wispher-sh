import { describe, expect, it } from "vitest";
import { changeSummary, diffWords } from "./diff";

const text = (runs: { kind: string; words: string[] }[], kind: string) =>
  runs.filter((r) => r.kind === kind).map((r) => r.words.join(" "));

describe("diffWords", () => {
  it("marks fillers as dropped and ignores case and punctuation", () => {
    const d = diffWords("um so see you tomorrow", "So, see you tomorrow.");
    expect(text(d.heard, "drop")).toEqual(["um"]);
    expect(d.written.every((r) => r.kind === "keep")).toBe(true);
    expect(d.dropped).toBe(1);
    expect(d.added).toBe(0);
  });

  it("shows a dictionary fix as a drop in heard and an add in written", () => {
    const d = diffWords("see you on tory tomorrow", "See you on Tauri tomorrow");
    expect(text(d.heard, "drop")).toEqual(["tory"]);
    expect(text(d.written, "add")).toEqual(["Tauri"]);
    expect(changeSummary(d)).toBe("1 word removed · 1 word changed or added");
  });

  it("keeps runs together and handles empty sides", () => {
    expect(diffWords("", "Hello there").written).toEqual([{ kind: "add", words: ["Hello", "there"] }]);
    expect(diffWords("hello there", "").heard).toEqual([{ kind: "drop", words: ["hello", "there"] }]);
    expect(changeSummary(diffWords("same words", "Same words."))).toBe("");
  });

  it("does not count punctuation-only tokens as words", () => {
    const d = diffWords("first second", "First — second");
    expect(d.added).toBe(0);
  });
});

describe("line breaks", () => {
  it("keeps them as tokens that never count as changes", () => {
    const d = diffWords("hi priya thanks", "Hi Priya,\n\nThanks");
    expect(d.added).toBe(0);
    expect(d.written.flatMap((r) => r.words)).toEqual(["Hi", "Priya,", "\n", "\n", "Thanks"]);
  });
});
