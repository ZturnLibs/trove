import { describe, expect, it } from "vitest";
import { addDays, deferPresets, nextMonday } from "@/lib/defer";

describe("defer date helpers", () => {
  it("addDays crosses month boundary", () => {
    expect(addDays("2026-08-30", 2)).toBe("2026-09-01");
  });

  it("nextMonday from Friday", () => {
    expect(nextMonday("2026-08-14")).toBe("2026-08-17");
  });

  it("presets include clear defer", () => {
    const presets = deferPresets("2026-08-15");
    expect(presets[0].value).toBeNull();
    expect(presets[1].value).toBe("2026-08-16");
  });
});
