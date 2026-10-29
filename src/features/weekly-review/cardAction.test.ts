import { describe, expect, it } from "vitest";
import { resolveCardAction } from "./cardAction";

describe("resolveCardAction", () => {
  it("todo 状态 → 正常出牌", () => {
    expect(resolveCardAction({ status: "todo", title: "任意" })).toEqual({
      proceed: true,
    });
  });

  it("已完结 → 跳过并提示（含标题）", () => {
    const decision = resolveCardAction({ status: "completed", title: "已完任务" });
    expect(decision).toEqual({
      proceed: false,
      notice: "「已完任务」已不在待办状态，已自动跳过",
    });
  });

  it("归档等非 todo 状态同样跳过", () => {
    expect(
      resolveCardAction({ status: "archived", title: "x" }).proceed,
    ).toBe(false);
  });
});
