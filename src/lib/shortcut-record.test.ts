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

  it("returns Ctrl+Alt+Space for the Ctrl+Alt+Space combination (new default)", () => {
    expect(
      eventToShortcutString(makeEvent({ key: " ", ctrlKey: true, altKey: true })),
    ).toBe("Ctrl+Alt+Space");
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

  it("returns null for system-reserved IME / launcher / window keys", () => {
    // 输入法开关 / Spotlight
    expect(eventToShortcutString(makeEvent({ key: " ", ctrlKey: true }))).toBeNull();
    // 任务切换与窗口系统菜单
    expect(eventToShortcutString(makeEvent({ key: "Tab", altKey: true }))).toBeNull();
    expect(eventToShortcutString(makeEvent({ key: " ", altKey: true }))).toBeNull();
    // 任务管理器
    expect(
      eventToShortcutString(
        makeEvent({ key: "Escape", ctrlKey: true, shiftKey: true }),
      ),
    ).toBeNull();
  });

  it("returns null for Win-key-only combos on Windows (hijacks system keys)", () => {
    // metaKey 即 Win 键；Win+E / Win+D 属于系统，不允许单独注册
    expect(eventToShortcutString(makeEvent({ key: "e", metaKey: true }))).toBeNull();
    expect(
      eventToShortcutString(makeEvent({ key: "d", metaKey: true, shiftKey: true })),
    ).toBeNull();
    // 搭配 Ctrl 后允许（非 mac 上 Ctrl 优先记录，Win 修饰被忽略）
    expect(
      eventToShortcutString(makeEvent({ key: "c", metaKey: true, ctrlKey: true })),
    ).toBe("Ctrl+C");
  });

  it("normalizes the space key to Space", () => {
    expect(
      eventToShortcutString(makeEvent({ key: " ", ctrlKey: true, altKey: true })),
    ).toBe("Ctrl+Alt+Space");
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

  it("returns Command+Alt+Space for the Command+Alt+Space combination (new default)", () => {
    expect(
      eventToShortcutString(
        makeEvent({ key: " ", metaKey: true, altKey: true }),
      ),
    ).toBe("Command+Alt+Space");
  });

  it("returns null for blocked system shortcuts (Command+Q / Command+M)", () => {
    expect(eventToShortcutString(makeEvent({ key: "q", metaKey: true }))).toBeNull();
    expect(eventToShortcutString(makeEvent({ key: "m", metaKey: true }))).toBeNull();
  });

  it("returns null for system-reserved launcher / emoji / screenshot keys", () => {
    // Spotlight 搜索
    expect(eventToShortcutString(makeEvent({ key: " ", metaKey: true }))).toBeNull();
    // 表情符号与符号面板（Ctrl+Cmd+Space）
    expect(
      eventToShortcutString(
        makeEvent({ key: " ", metaKey: true, ctrlKey: true }),
      ),
    ).toBeNull();
    // 系统截图 ⌘⇧3 / ⌘⇧4 / ⌘⇧5
    expect(
      eventToShortcutString(makeEvent({ key: "3", metaKey: true, shiftKey: true })),
    ).toBeNull();
    expect(
      eventToShortcutString(makeEvent({ key: "4", metaKey: true, shiftKey: true })),
    ).toBeNull();
    expect(
      eventToShortcutString(makeEvent({ key: "5", metaKey: true, shiftKey: true })),
    ).toBeNull();
  });

  it("keeps Ctrl as a separate modifier on Mac", () => {
    expect(eventToShortcutString(makeEvent({ key: "c", ctrlKey: true }))).toBe(
      "Ctrl+C",
    );
  });
});
