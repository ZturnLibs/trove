import { beforeEach, describe, expect, it } from "vitest";
import { useRecentActions } from "./recent-actions";

describe("useRecentActions", () => {
  beforeEach(() => {
    useRecentActions.setState({ actions: [] });
  });

  it("keeps at most 5 actions with newest at the end", () => {
    for (let i = 0; i < 7; i++) {
      useRecentActions.getState().push({
        label: `action-${i}`,
        undo: async () => {},
      });
    }
    const actions = useRecentActions.getState().actions;
    expect(actions).toHaveLength(5);
    expect(actions[0].label).toBe("action-2");
    expect(actions[4].label).toBe("action-6");
  });

  it("assigns unique ids and pop removes by id", async () => {
    useRecentActions.getState().push({ label: "a", undo: async () => {} });
    useRecentActions.getState().push({ label: "b", undo: async () => {} });
    const [first] = useRecentActions.getState().actions;
    useRecentActions.getState().pop(first.id);
    expect(useRecentActions.getState().actions).toHaveLength(1);
    expect(useRecentActions.getState().actions[0].label).toBe("b");
  });

  it("clear resets the stack", () => {
    useRecentActions.getState().push({ label: "x", undo: async () => {} });
    useRecentActions.getState().clear();
    expect(useRecentActions.getState().actions).toEqual([]);
  });
});
