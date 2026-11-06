import { describe, expect, it } from "vitest";
import { clean } from "./clean";

// The same cases as the Rust tests in src-tauri/src/text/mod.rs.
describe("clean", () => {
  it("drops fillers but not words that contain them", () => {
    expect(clean("Um, so I think, uh, we should go.")).toBe("So I think, we should go.");
    expect(clean("The drum is loud.")).toBe("The drum is loud.");
    expect(clean("Hummus is good.")).toBe("Hummus is good.");
  });

  it("scratch that removes the sentence before it", () => {
    expect(clean("Send it tomorrow. Scratch that. Send it Friday.")).toBe("Send it Friday.");
    expect(clean("Hi team. Meet at 5, scratch that, meet at 6.")).toBe("Hi team. Meet at 6.");
    expect(clean("scratch that")).toBe("");
  });

  it("new line and new paragraph break the text", () => {
    expect(clean("Dear Sam, new line. Thanks for the notes.")).toBe("Dear Sam,\nThanks for the notes.");
    expect(clean("First part. New paragraph. second part.")).toBe("First part.\n\nSecond part.");
  });

  it("handles the page's examples", () => {
    expect(clean("So, um, Tuesday at ten. Scratch that. Friday at ten.")).toBe("Friday at ten.");
    expect(clean("Uh, can you grab um milk and bread on the way home?")).toBe("Can you grab milk and bread on the way home?");
  });
});
