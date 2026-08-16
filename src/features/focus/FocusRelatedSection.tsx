import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Paperclip } from "lucide-react";
import { ipc } from "@/ipc/client";

const PREVIEW_LIMIT = 5;

export function FocusRelatedSection({ taskId }: { taskId: string }) {
  const [expanded, setExpanded] = useState(false);

  const assetsQuery = useQuery({
    queryKey: ["links", "task", taskId, "assets"],
    queryFn: () => ipc.entityLinkAssets("task", taskId),
  });

  const linksQuery = useQuery({
    queryKey: ["links", "task", taskId],
    queryFn: () => ipc.entityLinkList("task", taskId),
  });

  const memoryIds = useMemo(() => {
    const ids: string[] = [];
    for (const link of linksQuery.data ?? []) {
      if (link.targetType === "memory") ids.push(link.targetId);
      if (link.sourceType === "memory") ids.push(link.sourceId);
    }
    return [...new Set(ids)];
  }, [linksQuery.data]);

  const memoriesQuery = useQuery({
    queryKey: ["memories", "focus-related", memoryIds],
    queryFn: async () => {
      const items = await Promise.all(
        memoryIds.map((id) => ipc.memoryGet(id).catch(() => null)),
      );
      return items.filter((m): m is NonNullable<typeof m> => m !== null);
    },
    enabled: memoryIds.length > 0,
  });

  const assets = assetsQuery.data ?? [];
  const memories = memoriesQuery.data ?? [];
  const total = assets.length + memories.length;

  if (total === 0) {
    return (
      <p className="text-[12px] text-muted">暂无关联记忆或附件。</p>
    );
  }

  const limit = expanded ? total : PREVIEW_LIMIT;
  let shown = 0;

  const assetSlice = assets.slice(0, Math.max(0, limit - shown));
  shown += assetSlice.length;
  const memorySlice = memories.slice(0, Math.max(0, limit - shown));

  return (
    <div className="space-y-3">
      {assetSlice.length > 0 ? (
        <ul className="grid grid-cols-4 gap-2">
          {assetSlice.map((asset) => {
            const src = asset.thumbBase64
              ? `data:image/png;base64,${asset.thumbBase64}`
              : null;
            return (
              <li
                key={asset.linkId}
                className="overflow-hidden rounded-[var(--radius-control)] border border-border bg-surface"
              >
                {src ? (
                  <img
                    src={src}
                    alt=""
                    className="aspect-square w-full object-cover"
                  />
                ) : (
                  <div className="flex aspect-square items-center justify-center text-[10px] text-muted">
                    附件
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      ) : null}

      {memorySlice.length > 0 ? (
        <ul className="space-y-1">
          {memorySlice.map((memory) => (
            <li
              key={memory.id}
              className="truncate rounded-[var(--radius-control)] border border-border bg-surface px-2 py-1 text-[12px]"
            >
              {memory.title || "无标题记忆"}
            </li>
          ))}
        </ul>
      ) : null}

      {total > PREVIEW_LIMIT ? (
        <button
          type="button"
          className="flex items-center gap-1 text-[11px] text-accent hover:underline"
          onClick={() => setExpanded((v) => !v)}
        >
          <Paperclip className="size-3" />
          {expanded ? "收起" : `展开全部 (${total})`}
        </button>
      ) : null}
    </div>
  );
}
