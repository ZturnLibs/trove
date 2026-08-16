import { describe, expect, it } from "vitest";
import {
  buildFireAtFromParsed,
  formatParsedHint,
  mergeTagNames,
  parseTagsInput,
} from "@/lib/nl-capture";
import type { ParsedCapture } from "@/ipc/client";

describe("buildFireAtFromParsed", () => {
  it("combines date and time", () => {
    expect(buildFireAtFromParsed("2026-08-17", "15:00")).toBe(
      "2026-08-17T15:00",
    );
  });

  it("defaults time when missing", () => {
    expect(buildFireAtFromParsed("2026-08-17", null)).toBe(
      "2026-08-17T09:00",
    );
  });
});

describe("parseTagsInput", () => {
  it("splits comma-separated tags", () => {
    expect(parseTagsInput("工作, 客户")).toEqual(["工作", "客户"]);
  });
});

describe("mergeTagNames", () => {
  it("dedupes parsed and manual tags", () => {
    expect(mergeTagNames(["工作"], "工作, 客户")).toEqual(["工作", "客户"]);
  });
});

describe("formatParsedHint", () => {
  it("renders multi-field hint", () => {
    const parsed: ParsedCapture = {
      title: "回复客户",
      dueDate: "2026-08-17",
      dueTime: "15:00",
      priority: "high",
      recurrence: null,
      ambiguousFields: [],
      tagNames: ["工作"],
      raw: "明天下午 #工作 p1 回复客户",
    };
    expect(formatParsedHint(parsed)).toContain("日期 2026-08-17");
    expect(formatParsedHint(parsed)).toContain("标签 工作");
  });
});
