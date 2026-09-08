import { describe, expect, it } from "vitest";
import {
  buildSidebarRows,
  buildTodayScopes,
  listNameInOverview,
} from "./listGroupTree";
import type { TaskListGroupOverview } from "@/ipc/client";

function makeOverview(): TaskListGroupOverview {
  return {
    inbox: {
      id: "inbox",
      name: "收件箱",
      kind: "inbox",
      groupId: null,
      openCount: 3,
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
            id: "l-a",
            name: "项目A",
            kind: "custom",
            groupId: "g-work",
            openCount: 2,
          },
          {
            id: "l-b",
            name: "项目B",
            kind: "custom",
            groupId: "g-work",
            openCount: 0,
          },
        ],
        openCount: 2,
      },
      {
        group: {
          id: "g-life",
          name: "个人",
          sortOrder: 1,
          createdAt: "",
          updatedAt: "",
          revision: 1,
        },
        lists: [],
        openCount: 0,
      },
    ],
    ungrouped: [
      {
        id: "l-c",
        name: "散装清单",
        kind: "custom",
        groupId: null,
        openCount: 5,
      },
    ],
  };
}

describe("buildSidebarRows", () => {
  it("orders groups first, then ungrouped lists, with counts", () => {
    const rows = buildSidebarRows(makeOverview());
    expect(rows.map((r) => r.kind)).toEqual(["group", "group", "list"]);
    expect(rows[0]).toMatchObject({ id: "g-work", name: "工作", openCount: 2 });
    const group = rows[0] as Extract<typeof rows[0], { kind: "group" }>;
    expect(group.lists.map((l) => l.id)).toEqual(["l-a", "l-b"]);
    expect(group.lists[0]).toMatchObject({ groupId: "g-work", openCount: 2 });
    expect(rows[2]).toMatchObject({
      kind: "list",
      id: "l-c",
      groupId: null,
      openCount: 5,
    });
  });

  it("handles an empty overview (fresh migration)", () => {
    const rows = buildSidebarRows({
      groups: [],
      ungrouped: [],
      inbox: {
        id: "inbox",
        name: "收件箱",
        kind: "inbox",
        groupId: null,
        openCount: 0,
      },
    });
    expect(rows).toEqual([]);
  });
});

describe("buildTodayScopes", () => {
  it("builds 全部 + 各分组 + 未分组 chips", () => {
    const scopes = buildTodayScopes(makeOverview());
    expect(scopes.map((s) => s.label)).toEqual([
      "全部",
      "工作",
      "个人",
      "未分组",
    ]);
    expect(scopes[0].scope).toBeNull();
    expect(scopes[1].scope).toEqual({ kind: "group", groupId: "g-work" });
    expect(scopes[3].scope).toEqual({ kind: "ungrouped" });
  });
});

describe("listNameInOverview", () => {
  it("resolves inbox, grouped and ungrouped list names", () => {
    const overview = makeOverview();
    expect(listNameInOverview(overview, "inbox")).toBe("收件箱");
    expect(listNameInOverview(overview, "l-a")).toBe("项目A");
    expect(listNameInOverview(overview, "l-c")).toBe("散装清单");
    expect(listNameInOverview(overview, "missing", "任务")).toBe("任务");
  });
});
