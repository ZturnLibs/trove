import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc, type AISuggestionRecord } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";

/**
 * v2.0 slice 5: daily work suggestions. The model picks 1–3 candidates from
 * the deterministic today pool with feature-cited reasons; joining the focus
 * list is the user's action (reusing the existing focus add + undo stack).
 * The card hides itself when there is nothing to suggest.
 */
export function DailySuggestionsCard() {
  const queryClient = useQueryClient();

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });
  const aiAvailable =
    !!settingsQuery.data?.ai &&
    settingsQuery.data.ai.mode !== "off" &&
    settingsQuery.data.ai.features.suggest;

  const pendingQuery = useQuery({
    queryKey: ["ai", "suggestions", "suggest"],
    queryFn: () => ipc.aiSuggestionList("suggest", "pending"),
    enabled: aiAvailable,
  });
  const record: AISuggestionRecord | undefined = pendingQuery.data?.find(
    (r) => r.sourceEntityType === "review" && r.sourceEntityId === "daily",
  );

  const requestMutation = useMutation({
    mutationFn: () => ipc.aiDailySuggestRequest(),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  const joinMutation = useMutation({
    mutationFn: async ({ index, taskId }: { index: number; taskId: string }) => {
      await ipc.dailyFocusAdd(taskId); // existing undoable action
      return ipc.aiDailySuggestAccept(record!.id, index);
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
  });

  const skipMutation = useMutation({
    mutationFn: (index: number) => ipc.aiDailySuggestSkip(record!.id, index),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  if (!aiAvailable) return null;

  const items = record?.payload.items ?? [];
  const busy = joinMutation.isPending || skipMutation.isPending;

  if (!record) {
    if (requestMutation.isPending) {
      return (
        <div className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-3 text-[12px] text-muted">
          正在生成今日建议…
        </div>
      );
    }
    if (requestMutation.isError) {
      return (
        <div className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-3 text-[12px] text-muted">
          今日建议暂不可用，今日页功能不受影响。
          <Button
            size="sm"
            variant="ghost"
            className="ml-2"
            onClick={() => requestMutation.mutate()}
          >
            重试
          </Button>
        </div>
      );
    }
    return (
      <div className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-3">
        <div className="flex items-center justify-between">
          <h3 className="text-[12px] font-semibold">今日工作建议（AI）</h3>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => requestMutation.mutate()}
          >
            获取今日建议
          </Button>
        </div>
      </div>
    );
  }

  if (items.length === 0) return null;

  return (
    <div className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-3">
      <h3 className="text-[12px] font-semibold">今日工作建议（AI）</h3>
      <ul className="mt-2 space-y-2">
        {items.map((item, idx) => {
          const source = record.sources[idx];
          return (
            <li
              key={`${record.id}-${idx}`}
              className="rounded border border-border p-2 text-[12px]"
            >
              <span className="block font-medium">{item.title}</span>
              {item.detail ? (
                <span className="mt-0.5 block text-muted">{item.detail}</span>
              ) : null}
              {item.sourceExcerpt ? (
                <span className="mt-1 block truncate font-mono text-[10px] text-muted">
                  {item.sourceExcerpt}
                </span>
              ) : null}
              <div className="mt-1.5 flex gap-2">
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy || !source}
                  onClick={() =>
                    source &&
                    joinMutation.mutate({ index: idx, taskId: source.entityId })
                  }
                >
                  加入重点
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => skipMutation.mutate(idx)}
                >
                  跳过
                </Button>
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
