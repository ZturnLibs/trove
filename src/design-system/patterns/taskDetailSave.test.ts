import { describe, expect, it } from "vitest";
import type { UpdateTaskInput } from "@/ipc/client";
import { buildDetailSaveInput, parseTagNames } from "./taskDetailSave";

const draft: UpdateTaskInput = {
  id: "t1",
  title: "标题",
  notes: "",
  priority: "none",
  listId: "inbox",
  dueDate: null,
  dueTime: null,
  tagNames: [],
};

describe("parseTagNames", () => {
  it("英文逗号分隔", () => {
    expect(parseTagNames("work,life")).toEqual(["work", "life"]);
  });

  it("中文逗号分隔", () => {
    expect(parseTagNames("工作，生活")).toEqual(["工作", "生活"]);
  });

  it("逐项 trim 并去空项", () => {
    expect(parseTagNames(" 工作 , ， 生活 ,, ")).toEqual(["工作", "生活"]);
  });

  it("空文本 → 空数组", () => {
    expect(parseTagNames("")).toEqual([]);
  });
});

describe("buildDetailSaveInput", () => {
  it("draft 字段全量透传，标签由解析替换", () => {
    expect(buildDetailSaveInput(draft, "a,b")).toEqual({ ...draft, tagNames: ["a", "b"] });
  });

  it("不改 draft 的其余字段值", () => {
    const out = buildDetailSaveInput({ ...draft, title: "新标题" }, "");
    expect(out.title).toBe("新标题");
    expect(out.listId).toBe("inbox");
    expect(out.tagNames).toEqual([]);
  });
});
