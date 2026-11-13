import { describe, expect, it } from "vitest";
import {
  quickAddSubmitHint,
  shouldSubmitQuickAdd,
  splitQuickAddLines,
  submitComboLabel,
} from "./quickAddSubmit";

describe("splitQuickAddLines", () => {
  it("单行输入返回长度 1", () => {
    expect(splitQuickAddLines("回复客户")).toEqual(["回复客户"]);
  });

  it("按行拆分并去除空行与首尾空白", () => {
    expect(splitQuickAddLines("  任务一  \r\n\r\n任务二\n   \n任务三 ")).toEqual([
      "任务一",
      "任务二",
      "任务三",
    ]);
  });

  it("纯空白输入返回空数组", () => {
    expect(splitQuickAddLines("  \n ")).toEqual([]);
  });
});

describe("shouldSubmitQuickAdd", () => {
  it("enter 模式：裸 Enter 提交", () => {
    expect(shouldSubmitQuickAdd("Enter", false, "enter")).toBe(true);
  });

  it("enter 模式：⌘/Ctrl+Enter 同样提交（既有习惯超集）", () => {
    expect(shouldSubmitQuickAdd("Enter", true, "enter")).toBe(true);
  });

  it("enter 模式：其他键不提交", () => {
    expect(shouldSubmitQuickAdd("Escape", false, "enter")).toBe(false);
  });

  it("modEnter 模式：裸 Enter 不提交", () => {
    expect(shouldSubmitQuickAdd("Enter", false, "modEnter")).toBe(false);
  });

  it("modEnter 模式：⌘/Ctrl+Enter 提交", () => {
    expect(shouldSubmitQuickAdd("Enter", true, "modEnter")).toBe(true);
  });

  it("modEnter 模式：其他键不提交", () => {
    expect(shouldSubmitQuickAdd("Escape", true, "modEnter")).toBe(false);
  });
});

describe("submitComboLabel", () => {
  it("enter 模式 → Enter", () => {
    expect(submitComboLabel("enter")).toBe("Enter");
  });

  it("modEnter 模式：mac → ⌘↩，其他平台 → Ctrl+↩", () => {
    const original = navigator.platform;
    Object.defineProperty(navigator, "platform", {
      value: "MacIntel",
      configurable: true,
    });
    expect(submitComboLabel("modEnter")).toBe("⌘↩");
    Object.defineProperty(navigator, "platform", {
      value: "Win32",
      configurable: true,
    });
    expect(submitComboLabel("modEnter")).toBe("Ctrl+↩");
    Object.defineProperty(navigator, "platform", {
      value: original,
      configurable: true,
    });
  });
});

describe("quickAddSubmitHint", () => {
  it("enter 模式提示 Enter 添加", () => {
    expect(quickAddSubmitHint("enter")).toBe("Enter 添加");
  });

  it("modEnter 模式：mac 提示 ⌘↩，其他平台提示 Ctrl+↩", () => {
    const original = navigator.platform;
    Object.defineProperty(navigator, "platform", {
      value: "MacIntel",
      configurable: true,
    });
    expect(quickAddSubmitHint("modEnter")).toBe("⌘↩ 添加");
    Object.defineProperty(navigator, "platform", {
      value: "Win32",
      configurable: true,
    });
    expect(quickAddSubmitHint("modEnter")).toBe("Ctrl+↩ 添加");
    Object.defineProperty(navigator, "platform", {
      value: original,
      configurable: true,
    });
  });
});
