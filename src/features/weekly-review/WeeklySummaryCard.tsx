import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc, type AISuggestionRecord } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";

/**
 * v2.0 slice 3: AI prose summary of the week's deterministic numbers.
 * The badges next to the text are the ground truth from snapshot(); the
 * model only organizes prose (§9.3).
 */
export function WeeklySummaryCard({
  snap,
}: {
  snap: {
    inboxCount: number;
    overdueCount: number;
    waitingFollowUpCount: number;
    completedLast7DaysCount: number;
  };
}) {
  const queryClient = useQueryClient();

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });
  const aiAvailable =
    !!settingsQuery.data?.ai &&
    settingsQuery.data.ai.mode !== "off" &&
    settingsQuery.data.ai.features.summary;

  const pendingQuery = useQuery({
    queryKey: ["ai", "suggestions", "summary"],
    queryFn: () => ipc.aiSuggestionList("summary", "pending"),
    enabled: aiAvailable,
  });
  const record: AISuggestionRecord | undefined = pendingQuery.data?.find(
    (r) => r.sourceEntityType === "review",
  );

  const generateMutation = useMutation({
    mutationFn: () => ipc.aiWeeklySummaryRequest(),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  const dismissMutation = useMutation({
    mutationFn: () => ipc.aiSuggestionDecide(record!.id, "dismiss"),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  if (!aiAvailable) return null;

  const badges: [string, number][] = [
    ["收件箱", snap.inboxCount],
    ["逾期", snap.overdueCount],
    ["等待", snap.waitingFollowUpCount],
    ["完成", snap.completedLast7DaysCount],
  ];

  return (
    <div className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-[13px] font-semibold">本周小结（AI）</h2>
        <div className="flex items-center gap-2">
          {record ? (
            <>
              <Button
                size="sm"
                variant="ghost"
                disabled={generateMutation.isPending}
                onClick={() => generateMutation.mutate()}
              >
                重新生成
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={dismissMutation.isPending}
                onClick={() => dismissMutation.mutate()}
              >
                忽略
              </Button>
            </>
          ) : (
            <Button
              size="sm"
              variant="secondary"
              disabled={generateMutation.isPending}
              onClick={() => generateMutation.mutate()}
            >
              {generateMutation.isPending ? "生成中…" : "生成 AI 摘要"}
            </Button>
          )}
        </div>
      </div>

      {generateMutation.isError && !record ? (
        <p className="mt-2 text-[12px] text-muted">
          AI 摘要暂不可用，回顾功能不受影响。
        </p>
      ) : null}

      {record?.payload.summary ? (
        <div className="mt-2 space-y-2">
          <p className="text-[12px] leading-relaxed">{record.payload.summary}</p>
          <div className="flex flex-wrap gap-1.5">
            {badges.map(([label, count]) => (
              <span
                key={label}
                className="rounded border border-border px-1.5 py-0.5 text-[11px] text-muted"
              >
                {label} {count}
              </span>
            ))}
          </div>
          <p className="text-[11px] text-muted">
            摘要基于本周确定性统计生成，数字以上方徽标为准。
          </p>
        </div>
      ) : null}
    </div>
  );
}
