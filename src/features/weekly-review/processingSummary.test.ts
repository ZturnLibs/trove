import { describe, expect, it } from "vitest";
import { flattenQueues, processingSummary } from "./processingSummary";
import type { Task, WeeklyReviewSnapshot } from "@/ipc/client";

function makeTask(id: string, extra?: Partial<Task>): Task {
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
    createdAt: "2026-10-01T00:00:00Z",
    updatedAt: "2026-10-01T00:00:00Z",
    revision: 1,
    ...extra,
  };
}

function makeSnapshot(
  queues: Partial<
    Pick<
      WeeklyReviewSnapshot,
      "overdue" | "inboxUnprocessed" | "waitingFollowUp" | "staleActive"
    >
  >,
): WeeklyReviewSnapshot {
  const overdue = queues.overdue ?? [];
  const inbox = queues.inboxUnprocessed ?? [];
  const waiting = queues.waitingFollowUp ?? [];
  const stale = queues.staleActive ?? [];
  return {
    inboxUnprocessed: inbox,
    inboxCount: inbox.length,
    overdue,
    overdueCount: overdue.length,
    waitingFollowUp: waiting,
    waitingFollowUpCount: waiting.length,
    staleActive: stale,
    staleActiveCount: stale.length,
    completedLast7Days: [],
    completedLast7DaysCount: 0,
    upcomingRecurringReminders: [],
    upcomingRecurringCount: 0,
    largeClipboardItems: [],
    largeClipboardCount: 0,
  };
}

describe("processingSummary", () => {
  it("returns all zeros for empty actions", () => {
    expect(processingSummary([])).toEqual({
      completed: 0,
      deferred: 0,
      waiting: 0,
      skipped: 0,
      total: 0,
    });
  });

  it("counts mixed actions", () => {
    expect(
      processingSummary([
        "completed",
        "deferred",
        "completed",
        "waiting",
        "skipped",
        "deferred",
        "completed",
      ]),
    ).toEqual({
      completed: 3,
      deferred: 2,
      waiting: 1,
      skipped: 1,
      total: 7,
    });
  });

  it("counts all-skipped sessions", () => {
    expect(processingSummary(["skipped", "skipped"])).toEqual({
      completed: 0,
      deferred: 0,
      waiting: 0,
      skipped: 2,
      total: 2,
    });
  });
});

describe("flattenQueues", () => {
  it("returns empty array for empty snapshot", () => {
    expect(flattenQueues(makeSnapshot({}))).toEqual([]);
  });

  it("orders overdue → inbox → waiting → staleActive and skips empty queues", () => {
    const items = flattenQueues(
      makeSnapshot({
        overdue: [makeTask("o1"), makeTask("o2")],
        inboxUnprocessed: [makeTask("i1")],
        waitingFollowUp: [],
        staleActive: [makeTask("s1")],
      }),
    );
    expect(items.map((item) => item.task.id)).toEqual([
      "o1",
      "o2",
      "i1",
      "s1",
    ]);
    expect(items[0].queueLabel).toBe("逾期任务");
    expect(items[2].queueLabel).toBe("未整理收件箱");
    expect(items[3].queueLabel).toBe("长期未更新");
  });
});
