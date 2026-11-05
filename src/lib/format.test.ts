import { describe, expect, it } from "vitest";
import { compactNumber, relativeTime, shortcutLabel } from "./format";

describe("shortcutLabel", () => {
  it("uses symbols on macOS", () => {
    expect(shortcutLabel("Meta+Shift+Space", "macos")).toBe("⌘ ⇧ Space");
    expect(shortcutLabel("RightAlt", "macos")).toBe("Right ⌥");
  });
  it("uses words on Windows", () => {
    expect(shortcutLabel("Ctrl+Shift+Space", "windows")).toBe("Ctrl + Shift + Space");
  });
});

describe("relativeTime", () => {
  it("rounds to friendly units", () => {
    const now = 1_000_000_000;
    expect(relativeTime(now - 5_000, now)).toBe("just now");
    expect(relativeTime(now - 5 * 60_000, now)).toBe("5 min ago");
    expect(relativeTime(now - 3 * 3_600_000, now)).toBe("3 h ago");
  });
});

describe("compactNumber", () => {
  it("abbreviates large numbers", () => {
    expect(compactNumber(950)).toBe("950");
    expect(compactNumber(12_345)).toBe("12.3k");
  });
});
