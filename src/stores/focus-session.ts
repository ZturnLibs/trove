import { create } from "zustand";
import { ipc, type FocusSession, type Task } from "@/ipc/client";
import { useRecentActions } from "@/stores/recent-actions";

export type FocusEndOutcome = Exclude<
  import("@/ipc/client").FocusOutcome,
  "inProgress"
>;

type FocusSessionState = {
  session: FocusSession | null;
  task: Task | null;
  open: boolean;
  /** Shown once after recovering a stale in_progress session on startup. */
  abandonedNotice: string | null;
  defaultPlannedMinutes: number | null;
  starting: boolean;
  ending: boolean;
  setDefaultPlannedMinutes: (minutes: number | null) => void;
  start: (taskId: string, plannedMinutes?: number | null) => Promise<void>;
  end: (outcome: FocusEndOutcome, progressNote?: string | null) => Promise<void>;
  dismissAbandonedNotice: () => void;
  recoverStaleSession: () => Promise<void>;
};

export const useFocusSession = create<FocusSessionState>((set, get) => ({
  session: null,
  task: null,
  open: false,
  abandonedNotice: null,
  defaultPlannedMinutes: 25,
  starting: false,
  ending: false,

  setDefaultPlannedMinutes: (minutes) =>
    set({ defaultPlannedMinutes: minutes }),

  start: async (taskId, plannedMinutes) => {
    if (get().starting) return;
    set({ starting: true });
    try {
      const minutes =
        plannedMinutes !== undefined
          ? plannedMinutes
          : get().defaultPlannedMinutes;
      const session = await ipc.focusStart(taskId, minutes);
      const task = await ipc.taskGet(taskId);
      set({ session, task, open: true, starting: false });
    } catch (err) {
      set({ starting: false });
      throw err;
    }
  },

  end: async (outcome, progressNote) => {
    const { session, ending } = get();
    if (!session || ending) return;
    set({ ending: true });
    try {
      const taskId = session.taskId;
      const wasCompleted = outcome === "completed";
      await ipc.focusEnd(session.id, outcome, progressNote ?? null);
      set({ session: null, task: null, open: false, ending: false });

      if (wasCompleted) {
        useRecentActions.getState().push({
          label: "完成专注",
          undo: async () => {
            await ipc.taskUncomplete(taskId);
          },
        });
      }
    } catch (err) {
      set({ ending: false });
      throw err;
    }
  },

  dismissAbandonedNotice: () => set({ abandonedNotice: null }),

  recoverStaleSession: async () => {
    const active = await ipc.focusActive();
    if (!active) return;
    await ipc.focusEnd(active.id, "abandoned");
    set({
      abandonedNotice: "上次专注未正常结束，已保存为放弃",
      session: null,
      task: null,
      open: false,
    });
  },
}));

/** Best-effort abandon when the window closes mid-session. */
export function abandonActiveFocusBestEffort() {
  const { session, open } = useFocusSession.getState();
  if (!open || !session) return;
  void ipc.focusEnd(session.id, "abandoned").catch(() => {
    /* ignore */
  });
}
