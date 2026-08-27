import { describe, expect, it } from "vitest";
import { buildForest, flattenVisible, siblingIds } from "./taskTree";
import type { Task } from "@/ipc/client";

function makeTask(id: string, parentId: string | null, extra?: Partial<Task>): Task {
  return {
    id,
    title: id,
    notes: "",
    status: "todo",
    priority: "none",
    listId: "l1",
    listName: "清单",
    listKind: "custom",
    dueDate: null,
    dueTime: null,
    completedAt: null,
    sortOrder: 0,
    seriesId: null,
    parentId,
    childOrder: 0,
    tagIds: [],
    tagNames: [],
    workflowState: "active",
    availableAt: null,
    waitingFor: null,
    followUpDate: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    revision: 1,
    ...extra,
  };
}

describe("buildForest", () => {
  it("groups children under parents and computes progress", () => {
    const forest = buildForest([
      makeTask("a", null, { sortOrder: 0 }),
      makeTask("b", null, { sortOrder: 1 }),
      makeTask("a1", "a", { childOrder: 0, status: "completed" }),
      makeTask("a2", "a", { childOrder: 1 }),
    ]);
    expect(forest.roots.map((r) => r.task.id)).toEqual(["a", "b"]);
    expect(forest.roots[0].children.map((c) => c.task.id)).toEqual(["a1", "a2"]);
    expect(forest.childCount.get("a")).toBe(2);
    expect(forest.progress.get("a")).toEqual({ done: 1, total: 2 });
    expect(forest.childCount.get("b")).toBeUndefined();
  });
});

describe("flattenVisible", () => {
  it("hides collapsed children and includes depth", () => {
    const forest = buildForest([
      makeTask("a", null),
      makeTask("a1", "a"),
      makeTask("a1x", "a1"),
      makeTask("b", null),
    ]);
    const all = flattenVisible(forest, new Set());
    expect(all.map((r) => r.task.id)).toEqual(["a", "a1", "a1x", "b"]);
    expect(all.find((r) => r.task.id === "a1")?.depth).toBe(1);
    expect(all.find((r) => r.task.id === "a1x")?.depth).toBe(2);

    const collapsedA1 = flattenVisible(forest, new Set(["a1"]));
    expect(collapsedA1.map((r) => r.task.id)).toEqual(["a", "a1", "b"]);
  });

  it("renders tasks with soft-deleted parents as roots", () => {
    // Parent id "ghost" is absent from the set (soft-deleted upstream).
    const forest = buildForest([makeTask("child", "ghost")]);
    expect(forest.roots.map((r) => r.task.id)).toEqual(["child"]);
    expect(flattenVisible(forest, new Set()).map((r) => r.task.id)).toEqual([
      "child",
    ]);
  });
});

describe("siblingIds", () => {
  it("returns parent-scoped sibling ids or roots", () => {
    const forest = buildForest([
      makeTask("a", null),
      makeTask("b", null),
      makeTask("a1", "a"),
      makeTask("a2", "a"),
    ]);
    expect(siblingIds(forest, "a1")).toEqual(["a1", "a2"]);
    expect(siblingIds(forest, "a")).toEqual(["a", "b"]);
    expect(siblingIds(forest, "nope")).toBeNull();
  });
});
