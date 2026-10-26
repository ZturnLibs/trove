import { describe, expect, it } from "vitest";
import {
  buildGroupByListId,
  defaultListIdForScope,
  splitBoardColumns,
} from "./splitBoardColumns";
import type { Task, TaskListGroupOverview } from "@/ipc/client";

function makeTask(
  id: string,
  extra?: Partial<Task>,
): Task {
  return {
    id,
    title: id,
    notes: "",
    status: "todo",
    boardStage: null,
    priority: "none",
    listId: "l1",
    listName: "清单A",
    listKind: "custom",
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
    revision: 1,
    ...extra,
  };
}

function makeOverview(): TaskListGroupOverview {
  return {
    inbox: {
      id: "inbox",
      name: "收件箱",
      kind: "inbox",
      groupId: null,
      openCount: 0,
    },
    groups: [
      {
        group: {
          id: "g-work",
          name: "工作",
          sortOrder: 0,
          createdAt: "",
          updatedAt: "",
          revision: 1,
        },
        lists: [
          {
            id: "l1",
            name: "清单A",
            kind: "custom",
            groupId: "g-work",
            openCount: 0,
          },
        ],
        openCount: 0,
      },
    ],
    ungrouped: [
      {
        id: "l2",
        name: "清单B",
        kind: "custom",
        groupId: null,
        openCount: 0,
      },
    ],
  };
}

describe("splitBoardColumns", () => {
  it("splits todo tasks by boardStage and passes completed through", () => {
    const todoTasks = [
      makeTask("t1"),
      makeTask("d1", { boardStage: "doing" }),
      makeTask("t2"),
    ];
    const completed = [
      makeTask("c1", { status: "completed", completedAt: "2026-01-02T00:00:00Z" }),
    ];
    const cols = splitBoardColumns(todoTasks, completed, null, new Map());
    expect(cols.todo.map((t) => t.id)).toEqual(["t1", "t2"]);
    expect(cols.doing.map((t) => t.id)).toEqual(["d1"]);
    expect(cols.done.map((t) => t.id)).toEqual(["c1"]);
  });

  it("preserves query order within each column", () => {
    const todoTasks = [
      makeTask("a", { sortOrder: 2 }),
      makeTask("b", { sortOrder: 1 }),
      makeTask("c", { sortOrder: 3, boardStage: "doing" }),
    ];
    const cols = splitBoardColumns(todoTasks, [], null, new Map());
    expect(cols.todo.map((t) => t.id)).toEqual(["a", "b"]);
    expect(cols.doing.map((t) => t.id)).toEqual(["c"]);
  });

  it("drops rows whose status left todo (optimistic complete patch moves them out immediately)", () => {
    const todoSource = [
      makeTask("t1"),
      makeTask("t2", { status: "completed", boardStage: "doing" }),
    ];
    const doneSource = [
      makeTask("d1", { status: "completed" }),
      makeTask("d2", { status: "todo" }),
    ];
    const cols = splitBoardColumns(todoSource, doneSource, null, new Map());
    expect(cols.todo.map((t) => t.id)).toEqual(["t1"]);
    expect(cols.doing).toEqual([]);
    expect(cols.done.map((t) => t.id)).toEqual(["d1"]);
  });

  it("filters ungrouped scope by listId map (inbox counts as ungrouped)", () => {
    const todoTasks = [
      makeTask("grouped", { listId: "l1" }),
      makeTask("ungrouped", { listId: "l2" }),
      makeTask("inbox-task", { listId: "inbox", listKind: "inbox" }),
    ];
    const map = buildGroupByListId(makeOverview());
    const cols = splitBoardColumns(
      todoTasks,
      [makeTask("done-ungrouped", { listId: "l2", status: "completed" })],
      { kind: "ungrouped" },
      map,
    );
    expect(cols.todo.map((t) => t.id)).toEqual(["ungrouped", "inbox-task"]);
    expect(cols.doing).toEqual([]);
    expect(cols.done.map((t) => t.id)).toEqual(["done-ungrouped"]);
  });

  it("passes group scope through to the server-side listGroupId filter", () => {
    const todoTasks = [makeTask("t1", { listId: "l1" })];
    // 空映射（overview 未加载）也不应吞掉服务端已过滤的结果。
    const cols = splitBoardColumns(todoTasks, [], { kind: "group", groupId: "g-work" }, new Map());
    expect(cols.todo.map((t) => t.id)).toEqual(["t1"]);
  });
});

describe("defaultListIdForScope", () => {
  const overview = makeOverview();

  it("returns the first list of the scoped group", () => {
    expect(
      defaultListIdForScope({ kind: "group", groupId: "g-work" }, overview),
    ).toBe("l1");
  });

  it("falls back to null (backend inbox) for empty group, ungrouped and null scope", () => {
    expect(
      defaultListIdForScope({ kind: "group", groupId: "g-empty" }, overview),
    ).toBeNull();
    expect(defaultListIdForScope({ kind: "ungrouped" }, overview)).toBeNull();
    expect(defaultListIdForScope(null, overview)).toBeNull();
  });
});

describe("buildGroupByListId", () => {
  it("maps grouped lists to their group and inbox/ungrouped to null", () => {
    const map = buildGroupByListId(makeOverview());
    expect(map.get("l1")).toBe("g-work");
    expect(map.get("l2")).toBeNull();
    expect(map.get("inbox")).toBeNull();
  });
});
