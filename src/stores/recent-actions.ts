import { create } from "zustand";

export type RecentAction = {
  id: number;
  label: string;
  undo: () => Promise<void>;
};

type RecentActionsState = {
  /** 最近动作栈，上限 5，最新动作在末尾。 */
  actions: RecentAction[];
  push: (action: Omit<RecentAction, "id">) => void;
  pop: (id: number) => void;
  clear: () => void;
};

// 单调递增 id，避免同毫秒内 Date.now() 碰撞导致误 pop。
let nextActionId = 1;

export const useRecentActions = create<RecentActionsState>((set) => ({
  actions: [],
  push: (action) =>
    set((state) => ({
      actions: [...state.actions, { ...action, id: nextActionId++ }].slice(-5),
    })),
  pop: (id) =>
    set((state) => ({
      actions: state.actions.filter((action) => action.id !== id),
    })),
  clear: () => set({ actions: [] }),
}));
