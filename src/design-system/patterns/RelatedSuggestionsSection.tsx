import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc, type AISuggestionRecord } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";

/**
 * v2.0 slice 4: related-content suggestions for a task. Suggestions are a
 * standalone review area — nothing is linked until the user confirms each
 * item; "不相关" persists a rejection that filters future suggestions.
 */
export function RelatedSuggestionsSection({ taskId }: { taskId: string }) {
  const queryClient = useQueryClient();
  const [handledCount, setHandledCount] = useState(0);
  const [seenSuggestion, setSeenSuggestion] = useState<string | null>(null);

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });
  const aiAvailable =
    !!settingsQuery.data?.ai &&
    settingsQuery.data.ai.mode !== "off" &&
    settingsQuery.data.ai.features.related;

  const pendingQuery = useQuery({
    queryKey: ["ai", "suggestions", "related", taskId],
    queryFn: () => ipc.aiSuggestionList("related", "pending"),
    enabled: aiAvailable,
  });
  const record: AISuggestionRecord | undefined = pendingQuery.data?.find(
    (r) => r.sourceEntityType === "task" && r.sourceEntityId === taskId,
  );

  useEffect(() => {
    if (record && record.id !== seenSuggestion) {
      setSeenSuggestion(record.id);
      setHandledCount(0);
    }
  }, [record?.id, record, seenSuggestion]);

  const requestMutation = useMutation({
    mutationFn: () => ipc.aiRelatedRequest(taskId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  const confirmMutation = useMutation({
    mutationFn: (index: number) => ipc.aiRelatedConfirm(record!.id, [index], taskId),
    onSuccess: () => {
      setHandledCount((n) => n + 1);
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
      void queryClient.invalidateQueries({ queryKey: ["links"] });
    },
  });

  const rejectMutation = useMutation({
    mutationFn: (index: number) => ipc.aiRelatedRejectItem(record!.id, index),
    onSuccess: () => {
      setHandledCount((n) => n + 1);
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  if (!aiAvailable) return null;

  const busy = confirmMutation.isPending || rejectMutation.isPending;
  const items = record?.payload.items ?? [];
  const done = !record || items.length === 0;

  return (
    <div className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-3">
      <div className="flex items-center justify-between">
        <h3 className="text-[12px] font-semibold">相关内容建议（AI）</h3>
        {!record ? (
          <Button
            size="sm"
            variant="secondary"
            disabled={requestMutation.isPending}
            onClick={() => requestMutation.mutate()}
          >
            {requestMutation.isPending ? "寻找中…" : "寻找相关内容"}
          </Button>
        ) : null}
      </div>

      {requestMutation.isError && !record ? (
        <p className="mt-2 text-[12px] text-muted">AI 建议暂不可用，任务功能不受影响。</p>
      ) : null}

      {done ? (
        <p className="mt-2 text-[12px] text-muted">
          {record
            ? `已处理 ${handledCount} 条${handledCount > 0 ? "，已关联内容见下方关联区" : ""}。`
            : "未找到可能相关的内容。"}
        </p>
      ) : (
        <ul className="mt-2 space-y-2">
          {items.map((item, idx) => (
            <li
              key={`${record!.id}-${idx}`}
              className="rounded border border-border p-2 text-[12px]"
            >
              <span className="block font-medium">{item.title}</span>
              {item.detail ? (
                <span className="mt-0.5 block text-muted">{item.detail}</span>
              ) : null}
              {item.sourceExcerpt ? (
                <span className="mt-1 block truncate font-mono text-[10px] text-muted">
                  「{item.sourceExcerpt}」
                </span>
              ) : null}
              <div className="mt-1.5 flex gap-2">
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy}
                  onClick={() => confirmMutation.mutate(idx)}
                >
                  关联
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => rejectMutation.mutate(idx)}
                >
                  不相关
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
