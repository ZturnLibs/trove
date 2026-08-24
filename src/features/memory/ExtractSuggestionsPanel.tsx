import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc, type AISuggestionRecord } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";

/**
 * v2.0 slice 2: task-draft review panel for an extract suggestion.
 * Every draft shows its source excerpt; ambiguous dates are surfaced as
 * "to confirm" and are never guessed into the created task.
 */
export function ExtractSuggestionsPanel({
  memoryId,
  onApplied,
}: {
  memoryId: string;
  /** Notifies the parent so task lists can be refreshed / navigated. */
  onApplied?: (taskTitles: string[]) => void;
}) {
  const queryClient = useQueryClient();
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [appliedTitles, setAppliedTitles] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const pendingQuery = useQuery({
    queryKey: ["ai", "suggestions", "extract", memoryId],
    queryFn: () => ipc.aiSuggestionList("extract", "pending"),
  });

  const record: AISuggestionRecord | undefined = useMemo(
    () => pendingQuery.data?.find((r) => r.sourceEntityId === memoryId),
    [pendingQuery.data, memoryId],
  );

  const applyMutation = useMutation({
    mutationFn: (indices: number[]) =>
      ipc.aiSuggestionApply(record!.id, indices),
    onSuccess: (result) => {
      setAppliedTitles(result.tasks.map((t) => t.title));
      setSelected(new Set());
      setError(null);
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["links"] });
      onApplied?.(result.tasks.map((t) => t.title));
    },
    onError: (e) => setError(String(e)),
  });

  const rejectMutation = useMutation({
    mutationFn: () => ipc.aiSuggestionDecide(record!.id, "reject"),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  const toggle = (idx: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) next.delete(idx);
      else next.add(idx);
      return next;
    });
  };

  if (appliedTitles) {
    return (
      <div className="rounded border border-border bg-surface p-3 text-[12px]">
        <p className="font-medium">已创建 {appliedTitles.length} 个任务：</p>
        <ul className="mt-1 list-inside list-disc text-muted">
          {appliedTitles.map((t) => (
            <li key={t}>{t}</li>
          ))}
        </ul>
        <p className="mt-1 text-muted">已放入收件箱，可在任务页继续整理。</p>
      </div>
    );
  }

  if (!record) {
    return (
      <div className="rounded border border-border bg-surface p-3 text-[12px] text-muted">
        没有识别出任务草稿。
      </div>
    );
  }

  const items = record.payload.items;

  return (
    <div className="rounded border border-border bg-surface p-3 text-[12px]">
      <div className="flex items-center justify-between">
        <p className="font-medium">
          识别出 {items.length} 条候选任务（{selected.size} 已选）
        </p>
        <ConfirmButton
          size="sm"
          variant="ghost"
          confirmLabel="确认忽略"
          confirmTitle="忽略后可重新提取"
          resetKey={record.id}
          onConfirm={() => rejectMutation.mutate()}
          disabled={rejectMutation.isPending}
        >
          都不合适
        </ConfirmButton>
      </div>

      <ul className="mt-2 space-y-2">
        {items.map((item, idx) => (
          <li key={idx} className="rounded border border-border p-2">
            <label className="flex items-start gap-2">
              <input
                type="checkbox"
                className="mt-0.5"
                checked={selected.has(idx)}
                onChange={() => toggle(idx)}
              />
              <span className="min-w-0 flex-1">
                <span className="block font-medium">{item.title}</span>
                {item.detail ? (
                  <span className="mt-0.5 block text-muted">{item.detail}</span>
                ) : null}
                <span className="mt-1 block text-[11px]">
                  {item.ambiguous ? (
                    <span className="text-warning">日期待确认，创建时不填</span>
                  ) : item.dueDate ? (
                    <span className="text-muted">
                      {item.dueDate}
                      {item.dueTime ? ` ${item.dueTime}` : ""}
                    </span>
                  ) : (
                    <span className="text-muted">无日期</span>
                  )}
                </span>
                <span className="mt-1 block truncate font-mono text-[10px] text-muted">
                  原文：「{item.sourceExcerpt}」
                </span>
              </span>
            </label>
          </li>
        ))}
      </ul>

      <div className="mt-2 flex items-center gap-2">
        <Button
          size="sm"
          variant="secondary"
          disabled={selected.size === 0 || applyMutation.isPending}
          onClick={() => applyMutation.mutate([...selected].sort((a, b) => a - b))}
        >
          {applyMutation.isPending ? "创建中…" : `创建选中任务（${selected.size}）`}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          disabled={selected.size === items.length}
          onClick={() => setSelected(new Set(items.map((_, i) => i)))}
        >
          全选
        </Button>
        {error ? <span className="text-warning">{error}</span> : null}
      </div>
    </div>
  );
}
