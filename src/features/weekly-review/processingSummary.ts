import type { Task, WeeklyReviewSnapshot } from "@/ipc/client";

export type ActionKind = "completed" | "deferred" | "waiting" | "skipped";

export type ProcessingSummary = {
  completed: number;
  deferred: number;
  waiting: number;
  skipped: number;
  total: number;
};

export const QUEUE_LABELS = {
  overdue: "逾期任务",
  inboxUnprocessed: "未整理收件箱",
  waitingFollowUp: "等待跟进",
  staleActive: "长期未更新",
} as const;

export type QueueName = keyof typeof QUEUE_LABELS;

export type ProcessingItem = {
  task: Task;
  queue: QueueName;
  queueLabel: string;
};

export function processingSummary(actions: ActionKind[]): ProcessingSummary {
  const summary: ProcessingSummary = {
    completed: 0,
    deferred: 0,
    waiting: 0,
    skipped: 0,
    total: actions.length,
  };
  for (const action of actions) {
    summary[action] += 1;
  }
  return summary;
}

const QUEUE_FIELDS = [
  "overdue",
  "inboxUnprocessed",
  "waitingFollowUp",
  "staleActive",
] as const satisfies readonly QueueName[];

export function flattenQueues(
  snapshot: WeeklyReviewSnapshot,
): ProcessingItem[] {
  const items: ProcessingItem[] = [];
  for (const queue of QUEUE_FIELDS) {
    for (const task of snapshot[queue]) {
      items.push({
        task,
        queue,
        queueLabel: QUEUE_LABELS[queue],
      });
    }
  }
  return items;
}
