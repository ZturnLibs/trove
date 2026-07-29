import { create } from "zustand";

export type QuickMode = "capture" | "search" | "clip";

type UiState = {
  selectedId: string | null;
  quickMode: QuickMode;
  setSelectedId: (id: string | null) => void;
  setQuickMode: (mode: QuickMode) => void;
};

export const useUiStore = create<UiState>((set) => ({
  selectedId: null,
  quickMode: "capture",
  setSelectedId: (id) => set({ selectedId: id }),
  setQuickMode: (mode) => set({ quickMode: mode }),
}));
