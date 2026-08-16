import { describe, expect, it } from "vitest";
import { actionsForKindHint, KIND_HINT_LABEL } from "./clipboard-smart";

describe("clipboard-smart", () => {
  it("maps kind hints to labels", () => {
    expect(KIND_HINT_LABEL.code).toBe("代码");
    expect(KIND_HINT_LABEL.error).toBe("报错");
  });

  it("suggests fewer actions for phone and code", () => {
    expect(actionsForKindHint("phone")).toEqual(["task", "copy"]);
    expect(actionsForKindHint("code")).toEqual(["memory", "copy"]);
    expect(actionsForKindHint("plain")).toContain("memory");
  });
});
