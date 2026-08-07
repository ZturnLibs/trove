import { describe, expect, it } from "vitest";
import {
  defaultRecurrence,
  recurrenceLabel,
  toggleWeekday,
  withRecurrenceFrequency,
} from "./recurrence";

describe("recurrenceLabel", () => {
  it("labels daily and weekdays", () => {
    expect(
      recurrenceLabel({
        version: 1,
        frequency: "daily",
        interval: 1,
        timezone: "UTC",
      }),
    ).toBe("每天");
    expect(
      recurrenceLabel({
        version: 1,
        frequency: "weekdays",
        interval: 1,
        timezone: "UTC",
      }),
    ).toBe("工作日");
  });

  it("labels weekly with weekday names", () => {
    expect(
      recurrenceLabel({
        version: 1,
        frequency: "weekly",
        interval: 1,
        weekdays: [1, 5],
        timezone: "UTC",
      }),
    ).toBe("每周 一、五");
  });

  it("labels monthly and everyN", () => {
    expect(
      recurrenceLabel({
        version: 1,
        frequency: "monthly",
        interval: 1,
        monthday: 15,
        timezone: "UTC",
      }),
    ).toBe("每月 15 日");
    expect(
      recurrenceLabel({
        version: 1,
        frequency: "everyNDays",
        interval: 3,
        timezone: "UTC",
      }),
    ).toBe("每 3 天");
  });
});

describe("defaultRecurrence", () => {
  it("seeds weekly with a weekday", () => {
    const rule = defaultRecurrence("weekly", new Date("2026-08-03")); // Monday
    expect(rule.frequency).toBe("weekly");
    expect(rule.weekdays).toEqual([1]);
  });
});

describe("withRecurrenceFrequency", () => {
  it("preserves timezone when switching frequency", () => {
    const current = defaultRecurrence("daily");
    current.timezone = "Asia/Tokyo";
    const next = withRecurrenceFrequency(current, "weekly");
    expect(next.timezone).toBe("Asia/Tokyo");
    expect(next.frequency).toBe("weekly");
  });
});

describe("toggleWeekday", () => {
  it("keeps at least one weekday selected", () => {
    const rule = defaultRecurrence("weekly");
    rule.weekdays = [1];
    const next = toggleWeekday(rule, 1);
    expect(next.weekdays).toEqual([1]);
  });
});
