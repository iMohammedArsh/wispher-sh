import { describe, expect, it } from "vitest";
import { calendarWeeks, compactNumber, duration, lastDays, lengthLabel, niceCeiling, relativeTime, shortcutLabel, timeParts } from "./format";

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

describe("calendarWeeks", () => {
  it("ends with the current week and grades days by quartile", () => {
    const now = new Date(2026, 10, 5); // Thursday 5 Nov 2026
    const weeks = calendarWeeks([["2026-11-05", 900], ["2026-11-04", 100], ["2026-11-02", 400], ["2026-10-01", 50]], 3, now);
    expect(weeks).toHaveLength(3);
    const last = weeks[2];
    expect(last[0].date.getDay()).toBe(1); // Monday first
    expect(last[3].isToday).toBe(true);
    expect(last[3].level).toBe(4);
    expect(last[2].level).toBeGreaterThan(0);
    expect(last[4].inRange).toBe(false); // Friday hasn't happened yet
    expect(weeks.flat().filter((d) => d.words > 0)).toHaveLength(3); // 1 Oct is outside the 3 weeks
  });
});

describe("durations", () => {
  it("formats timers and lengths", () => {
    expect(duration(65_000)).toBe("1:05");
    expect(duration(3_725_000)).toBe("1:02:05");
    expect(timeParts(312)).toEqual([[5, "h"], [12, "m"]]);
    expect(lengthLabel(23 * 60_000)).toBe("23 min");
    expect(lengthLabel(64 * 60_000)).toBe("1 h 4 min");
  });
});

describe("week strip", () => {
  it("lists the last days oldest first, with zeros for quiet days", () => {
    const now = new Date(2026, 10, 6, 15);
    const days = lastDays([["2026-11-06", 87], ["2026-11-04", 300]], 7, now);
    expect(days).toHaveLength(7);
    expect(days[6]).toMatchObject({ words: 87, isToday: true });
    expect(days[4].words).toBe(300);
    expect(days[5].words).toBe(0);
    expect(days[0].date.getDate()).toBe(31);
  });

  it("rounds the axis top up to a readable number", () => {
    expect(niceCeiling(0)).toBe(100);
    expect(niceCeiling(870)).toBe(1000);
    expect(niceCeiling(1340)).toBe(1500);
    expect(niceCeiling(200)).toBe(200);
  });
});
