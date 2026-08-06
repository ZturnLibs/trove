import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { formatShortcutLabel } from "@/lib/shortcuts";

function stubPlatform(platform: string) {
  vi.stubGlobal("navigator", { platform });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("formatShortcutLabel (non-Mac)", () => {
  beforeEach(() => {
    stubPlatform("Windows");
  });

  it("returns an empty string for null / undefined / empty input", () => {
    expect(formatShortcutLabel(null)).toBe("");
    expect(formatShortcutLabel(undefined)).toBe("");
    expect(formatShortcutLabel("")).toBe("");
  });

  it("maps Command to Ctrl", () => {
    expect(formatShortcutLabel("Command+Shift+Space")).toBe("Ctrl+Shift+Space");
  });

  it("maps Ctrl / Alt / Shift to their text labels", () => {
    expect(formatShortcutLabel("Ctrl+Alt+Shift+X")).toBe("Ctrl+Alt+Shift+X");
  });

  it("uppercases single character keys", () => {
    expect(formatShortcutLabel("Ctrl+a")).toBe("Ctrl+A");
    expect(formatShortcutLabel("a")).toBe("A");
  });

  it("keeps multi-character keys as-is (Enter / Esc / Space / F4)", () => {
    expect(formatShortcutLabel("Enter")).toBe("Enter");
    expect(formatShortcutLabel("Esc")).toBe("Esc");
    expect(formatShortcutLabel("Space")).toBe("Space");
    expect(formatShortcutLabel("Alt+F4")).toBe("Alt+F4");
  });

  it("renders Backspace with its text label on non-Mac", () => {
    expect(formatShortcutLabel("Backspace")).toBe("Backspace");
  });
});

describe("formatShortcutLabel (Mac)", () => {
  beforeEach(() => {
    stubPlatform("MacIntel");
  });

  it("maps Command to ⌘ and joins without separators", () => {
    expect(formatShortcutLabel("Command+Shift+Space")).toBe("⌘⇧Space");
  });

  it("maps Ctrl / Alt / Shift to symbols", () => {
    expect(formatShortcutLabel("Ctrl+Alt+Shift+X")).toBe("⌃⌥⇧X");
  });

  it("renders Backspace as ⌫", () => {
    expect(formatShortcutLabel("Backspace")).toBe("⌫");
  });
});
