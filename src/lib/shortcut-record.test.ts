import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { eventToShortcutString } from "@/lib/shortcut-record";

type KeyEvent = Pick<
  KeyboardEvent,
  "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey"
>;

function makeEvent(overrides: Partial<KeyEvent> = {}): KeyboardEvent {
  return {
    key: "",
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    ...overrides,
  } as KeyboardEvent;
}

function stubPlatform(platform: string) {
  vi.stubGlobal("navigator", { platform });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("eventToShortcutString (non-Mac)", () => {
  beforeEach(() => {
    stubPlatform("Windows");
  });

  it("returns Ctrl+Shift+Space for the Ctrl+Shift+Space combination", () => {
    expect(
      eventToShortcutString(makeEvent({ key: " ", ctrlKey: true, shiftKey: true })),
    ).toBe("Ctrl+Shift+Space");
  });

  it("returns null for a single key without modifiers", () => {
    expect(eventToShortcutString(makeEvent({ key: "a" }))).toBeNull();
  });

  it("returns null when only a modifier key is pressed", () => {
    expect(eventToShortcutString(makeEvent({ key: "Meta", metaKey: true }))).toBeNull();
    expect(eventToShortcutString(makeEvent({ key: "Control", ctrlKey: true }))).toBeNull();
    expect(eventToShortcutString(makeEvent({ key: "Shift", shiftKey: true }))).toBeNull();
    expect(eventToShortcutString(makeEvent({ key: "Alt", altKey: true }))).toBeNull();
  });

  it("returns null for blocked system shortcuts (Ctrl+Q / Ctrl+W / Alt+F4)", () => {
    expect(eventToShortcutString(makeEvent({ key: "q", ctrlKey: true }))).toBeNull();
    expect(eventToShortcutString(makeEvent({ key: "w", ctrlKey: true }))).toBeNull();
    expect(
      eventToShortcutString(makeEvent({ key: "F4", altKey: true })),
    ).toBeNull();
  });

  it("normalizes the space key to Space", () => {
    expect(eventToShortcutString(makeEvent({ key: " ", ctrlKey: true }))).toBe(
      "Ctrl+Space",
    );
  });

  it("uppercases single character keys", () => {
    expect(eventToShortcutString(makeEvent({ key: "a", ctrlKey: true }))).toBe(
      "Ctrl+A",
    );
  });
});

describe("eventToShortcutString (Mac)", () => {
  beforeEach(() => {
    stubPlatform("MacIntel");
  });

  it("returns Command+Shift+Space for the Command+Shift+Space combination", () => {
    expect(
      eventToShortcutString(
        makeEvent({ key: " ", metaKey: true, shiftKey: true }),
      ),
    ).toBe("Command+Shift+Space");
  });

  it("returns null for blocked system shortcuts (Command+Q / Command+M)", () => {
    expect(eventToShortcutString(makeEvent({ key: "q", metaKey: true }))).toBeNull();
    expect(eventToShortcutString(makeEvent({ key: "m", metaKey: true }))).toBeNull();
  });

  it("keeps Ctrl as a separate modifier on Mac", () => {
    expect(eventToShortcutString(makeEvent({ key: "c", ctrlKey: true }))).toBe(
      "Ctrl+C",
    );
  });
});
