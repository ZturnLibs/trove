import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc, type AISuggestionRecord, type Task } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";

/**
 * v2.0 slice 6: one-level task checklist. Completed tasks freeze the list to
 * read-only; checking every item never auto-completes the task.
 */
export function ChecklistSection({ task }: { task: Task }) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingText, setEditingText] = useState("");

  const frozen = task.status === "completed";

  // v2.0 slice 7: AI split into checklist candidates.
  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });
  const aiAvailable =
    !!settingsQuery.data?.ai &&
    settingsQuery.data.ai.mode !== "off" &&
    settingsQuery.data.ai.features.split &&
    !frozen;

  const splitPendingQuery = useQuery({
    queryKey: ["ai", "suggestions", "split", task.id],
    queryFn: () => ipc.aiSuggestionList("split", "pending"),
    enabled: aiAvailable,
  });
  const splitRecord: AISuggestionRecord | undefined = splitPendingQuery.data?.find(
    (r) => r.sourceEntityType === "task" && r.sourceEntityId === task.id,
  );
  const [splitSelected, setSplitSelected] = useState<Set<number>>(new Set());
  useEffect(() => {
    setSplitSelected(new Set());
  }, [splitRecord?.id]);

  const splitRequestMutation = useMutation({
    mutationFn: () => ipc.aiSplitRequest(task.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });
  const splitApplyMutation = useMutation({
    mutationFn: (indices: number[]) => ipc.aiSplitApply(splitRecord!.id, indices),
    onSuccess: () => {
      invalidate();
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });
  const splitDismissMutation = useMutation({
    mutationFn: () => ipc.aiSuggestionDecide(splitRecord!.id, "reject"),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  const checklistQuery = useQuery({
    queryKey: ["tasks", "checklist", task.id],
    queryFn: () => ipc.taskChecklistList(task.id),
  });
  const items = checklistQuery.data?.items ?? [];

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["tasks", "checklist", task.id] });
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  };

  const addMutation = useMutation({
    mutationFn: (content: string) => ipc.taskChecklistAdd(task.id, content),
    onSuccess: () => {
      setDraft("");
      invalidate();
    },
  });
  const updateMutation = useMutation({
    mutationFn: (input: { id: string; content?: string; checked?: boolean }) =>
      ipc.taskChecklistUpdate(input),
    onSuccess: () => invalidate(),
  });
  const deleteMutation = useMutation({
    mutationFn: (id: string) => ipc.taskChecklistDelete(id),
    onSuccess: () => invalidate(),
  });
  const reorderMutation = useMutation({
    mutationFn: (orderedIds: string[]) => ipc.taskChecklistReorder(task.id, orderedIds),
    onSuccess: () => invalidate(),
  });

  const move = (index: number, delta: -1 | 1) => {
    const next = [...items];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    reorderMutation.mutate(next.map((i) => i.id));
  };

  return (
    <section className="space-y-2">
      <div className="flex items-center justify-between">
        <h3 className="text-[12px] font-semibold">
          检查项
          {items.length > 0 ? (
            <span className="ml-1 text-muted">
              {items.filter((i) => i.checked).length}/{items.length}
            </span>
          ) : null}
        </h3>
        {aiAvailable ? (
          <Button
            size="sm"
            variant="ghost"
            disabled={splitRequestMutation.isPending}
            onClick={() => splitRequestMutation.mutate()}
            title="用 AI 从任务说明生成候选检查项，勾选后写入"
          >
            {splitRequestMutation.isPending ? "拆分中…" : "AI 拆分"}
          </Button>
        ) : null}
      </div>

      {splitRecord ? (
        <div className="rounded border border-border p-2">
          <div className="flex items-center justify-between text-[11px] text-muted">
            <span>
              候选检查项（{splitRecord.payload.items.length} 条，{splitSelected.size} 已选）
            </span>
            <ConfirmButton
              size="sm"
              variant="ghost"
              confirmLabel="确认忽略"
              resetKey={splitRecord.id}
              onConfirm={() => splitDismissMutation.mutate()}
            >
              都不合适
            </ConfirmButton>
          </div>
          <ul className="mt-1 space-y-1">
            {splitRecord.payload.items.map((item, idx) => (
              <li key={`${splitRecord.id}-${idx}`}>
                <label className="flex items-start gap-2">
                  <input
                    type="checkbox"
                    className="mt-0.5"
                    checked={splitSelected.has(idx)}
                    onChange={() =>
                      setSplitSelected((prev) => {
                        const next = new Set(prev);
                        if (next.has(idx)) next.delete(idx);
                        else next.add(idx);
                        return next;
                      })
                    }
                  />
                  <span className="min-w-0 flex-1">
                    <span className="block">{item.title}</span>
                    {item.sourceExcerpt ? (
                      <span className="block truncate font-mono text-[10px] text-muted">
                        依据：「{item.sourceExcerpt}」
                      </span>
                    ) : null}
                  </span>
                </label>
              </li>
            ))}
          </ul>
          <div className="mt-1.5">
            <Button
              size="sm"
              variant="secondary"
              disabled={splitSelected.size === 0 || splitApplyMutation.isPending}
              onClick={() =>
                splitApplyMutation.mutate([...splitSelected].sort((a, b) => a - b))
              }
            >
              {splitApplyMutation.isPending
                ? "添加中…"
                : `添加选中（${splitSelected.size}）`}
            </Button>
            {splitApplyMutation.isError ? (
              <span className="ml-2 text-warning">{String(splitApplyMutation.error)}</span>
            ) : null}
            {splitRequestMutation.isError && !splitRecord ? (
              <span className="ml-2 text-warning">AI 拆分暂不可用。</span>
            ) : null}
          </div>
        </div>
      ) : splitRequestMutation.isError ? (
        <p className="text-[11px] text-warning">AI 拆分暂不可用，可继续手动添加。</p>
      ) : null}

      {items.length === 0 && frozen ? (
        <p className="text-[11px] text-muted">无检查项。</p>
      ) : null}

      <ul className="space-y-1">
        {items.map((item, index) => (
          <li key={item.id} className="flex items-center gap-2 text-[12px]">
            <input
              type="checkbox"
              className="mt-0"
              checked={item.checked}
              disabled={frozen || updateMutation.isPending}
              onChange={(e) =>
                updateMutation.mutate({ id: item.id, checked: e.target.checked })
              }
              aria-label={`勾选检查项：${item.content}`}
            />
            {editingId === item.id ? (
              <input
                className="min-w-0 flex-1 rounded border border-border bg-surface px-2 py-1"
                value={editingText}
                autoFocus
                onChange={(e) => setEditingText(e.target.value)}
                onBlur={() => {
                  const next = editingText.trim();
                  if (next && next !== item.content) {
                    updateMutation.mutate({ id: item.id, content: next });
                  }
                  setEditingId(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                  if (e.key === "Escape") setEditingId(null);
                }}
              />
            ) : (
              <button
                type="button"
                className="min-w-0 flex-1 truncate text-left"
                disabled={frozen}
                title={item.content}
                onClick={() => {
                  setEditingId(item.id);
                  setEditingText(item.content);
                }}
              >
                <span className={item.checked ? "text-muted line-through" : ""}>
                  {item.content}
                </span>
              </button>
            )}
            {!frozen ? (
              <span className="flex shrink-0 items-center gap-0.5">
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={index === 0}
                  onClick={() => move(index, -1)}
                  aria-label="上移"
                >
                  ↑
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={index === items.length - 1}
                  onClick={() => move(index, 1)}
                  aria-label="下移"
                >
                  ↓
                </Button>
                <ConfirmButton
                  size="sm"
                  confirmLabel="确认删除"
                  resetKey={item.id}
                  onConfirm={() => deleteMutation.mutate(item.id)}
                >
                  删除
                </ConfirmButton>
              </span>
            ) : null}
          </li>
        ))}
      </ul>

      {!frozen ? (
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            const content = draft.trim();
            if (content) addMutation.mutate(content);
          }}
        >
          <input
            className="min-w-0 flex-1 rounded border border-border bg-surface px-2 py-1 text-[12px]"
            value={draft}
            placeholder="添加检查项，回车保存"
            onChange={(e) => setDraft(e.target.value)}
            aria-label="新检查项内容"
          />
          <Button
            type="submit"
            size="sm"
            variant="secondary"
            disabled={!draft.trim() || addMutation.isPending}
          >
            添加
          </Button>
        </form>
      ) : (
        <p className="text-[11px] text-muted">任务已完成，检查项只读。</p>
      )}

      {addMutation.isError ? (
        <p className="text-[11px] text-warning">{String(addMutation.error)}</p>
      ) : null}
    </section>
  );
}

/** Progress badge for task rows: `2/5` when a checklist exists. */
export function ChecklistBadge({ taskId }: { taskId: string }) {
  const checklistQuery = useQuery({
    queryKey: ["tasks", "checklist", taskId],
    queryFn: () => ipc.taskChecklistList(taskId),
    staleTime: 30_000,
  });
  const total = checklistQuery.data?.total ?? 0;
  if (total === 0) return null;
  const done = checklistQuery.data?.checkedCount ?? 0;
  return (
    <span className="ml-1 rounded border border-border px-1 text-[10px] text-muted">
      {done}/{total}
    </span>
  );
}
