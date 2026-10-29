import { describe, expect, it } from "vitest";
import type { Task } from "@/ipc/client";
import { resolveBoardDrag } from "./boardDrag";
import type { BoardColumns } from "./splitBoardColumns";

function task(
  id: string,
  overrides: Partial<Pick<Task, "status" | "boardStage">> = {},
): Task {
  const base: Task = {
    id,
    title: id,
    notes: "",
    status: "todo",
    boardStage: null,
    priority: "none",
    listId: "inbox",
    listName: "收件箱",
    listKind: "inbox",
    dueDate: null,
    dueTime: null,
    completedAt: null,
    sortOrder: 0,
    seriesId: null,
    parentId: null,
    childOrder: 0,
    tagIds: [],
    tagNames: [],
    workflowState: "active",
    availableAt: null,
    waitingFor: null,
    followUpDate: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    revision: 0,
  };
  return { ...base, ...overrides };
}

function columns(partial: Partial<BoardColumns>): BoardColumns {
  return {
    todo: [],
    doing: [],
    done: [],
    ...partial,
  };
}

describe("resolveBoardDrag", () => {
  const cols = columns({
    todo: [task("t1"), task("t2")],
    doing: [task("d1", { boardStage: "doing" })],
    done: [task("x1", { status: "completed" })],
  });

  it("跨列：todo 卡拖入 doing 列 → stage=doing", () => {
    expect(resolveBoardDrag(cols, task("t1"), "col:doing")).toEqual({
      kind: "stage",
      id: "t1",
      stage: "doing",
    });
  });

  it("跨列：已在 doing 的卡拖回 doing → no-op（stage 守卫）", () => {
    const doing = columns({ doing: [task("d1", { boardStage: "doing" })] });
    expect(
      resolveBoardDrag(doing, task("d1", { boardStage: "doing" }), "col:doing"),
    ).toEqual({ kind: "none" });
  });

  it("跨列：completed 卡拖回 todo 列 → uncomplete", () => {
    const withDone = columns({
      done: [task("x1", { status: "completed" })],
      todo: [task("t1")],
    });
    expect(resolveBoardDrag(withDone, task("x1", { status: "completed" }), "col:todo")).toEqual({
      kind: "uncomplete",
      id: "x1",
    });
  });

  it("跨列：doing 态卡拖回 todo 列 → stage=null", () => {
    const withDoing = columns({
      doing: [task("d1", { boardStage: "doing" })],
      todo: [task("t1")],
    });
    expect(
      resolveBoardDrag(withDoing, task("d1", { boardStage: "doing" }), "col:todo"),
    ).toEqual({ kind: "stage", id: "d1", stage: null });
  });

  it("跨列：todo 卡拖入 done 列 → complete", () => {
    expect(resolveBoardDrag(cols, task("t1"), "col:done")).toEqual({
      kind: "complete",
      id: "t1",
    });
  });

  it("同列：拖到兄弟卡上 → reorder（整段新序）", () => {
    expect(resolveBoardDrag(cols, task("t1"), "t2")).toEqual({
      kind: "reorder",
      activeId: "t1",
      overId: "t2",
      orderedIds: ["t2", "t1"],
    });
  });

  it("同列：drop-on-self → no-op", () => {
    expect(resolveBoardDrag(cols, task("t1"), "t1")).toEqual({ kind: "none" });
  });

  it("同列：完成列不支持手动重排 → no-op", () => {
    expect(resolveBoardDrag(cols, task("x1", { status: "completed" }), "col:done")).toEqual({
      kind: "none",
    });
  });

  it("落点无法解析（未知 id）→ no-op", () => {
    expect(resolveBoardDrag(cols, task("t1"), "unknown-id")).toEqual({
      kind: "none",
    });
  });

  it("被拖卡不在任何列 → no-op", () => {
    expect(resolveBoardDrag(cols, task("ghost"), "t2")).toEqual({ kind: "none" });
  });
});
