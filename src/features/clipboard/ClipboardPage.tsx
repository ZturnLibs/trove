import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Star } from "lucide-react";
import { EmptyState } from "@/components/PageScaffold";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import { SplitTaskLayout } from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { ipc, type ClipboardItem } from "@/ipc/client";
import { cn } from "@/lib/cn";

function previewLine(item: ClipboardItem) {
  if (item.kind === "image") {
    const text = item.content.replace(/\s+/g, " ").trim();
    if (text.startsWith("[图片]") || text.length === 0) {
      return item.width && item.height
        ? `图片 ${item.width}×${item.height}`
        : "图片";
    }
    return text.slice(0, 120);
  }
  return item.content.replace(/\s+/g, " ").trim().slice(0, 120) || "（空文本）";
}

function thumbSrc(item: ClipboardItem) {
  if (!item.thumbBase64) return null;
  return `data:image/png;base64,${item.thumbBase64}`;
}

function ClipboardDetail({
  item,
  onDeleted,
}: {
  item: ClipboardItem | null;
  onDeleted?: () => void;
}) {
  const queryClient = useQueryClient();

  const favoriteMutation = useMutation({
    mutationFn: (favorite: boolean) => ipc.clipboardSetFavorite(item!.id, favorite),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["clipboard"] }),
  });

  const copyMutation = useMutation({
    mutationFn: () => ipc.clipboardCopy(item!.id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["clipboard"] }),
  });

  const deleteMutation = useMutation({
    mutationFn: () => ipc.clipboardDelete(item!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["clipboard"] });
      onDeleted?.();
    },
  });

  const toTaskMutation = useMutation({
    mutationFn: () => ipc.clipboardConvertToTask(item!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      alert("已转为任务（收件箱）");
    },
  });

  const toMemoryMutation = useMutation({
    mutationFn: () => ipc.clipboardConvertToMemory(item!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["memories"] });
      alert("已保存为记忆");
    },
  });

  if (!item) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-[12px] text-muted">
        选择一条历史查看详情
      </div>
    );
  }

  const imageSrc = thumbSrc(item);
  const isImage = item.kind === "image";
  const ocrText = item.ocrText?.trim() ?? "";

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          variant={item.favorite ? "default" : "secondary"}
          onClick={() => favoriteMutation.mutate(!item.favorite)}
        >
          {item.favorite ? "取消收藏" : "收藏"}
        </Button>
        <Button
          size="sm"
          onClick={() => copyMutation.mutate()}
          disabled={copyMutation.isPending}
        >
          再次复制
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => toTaskMutation.mutate()}
          disabled={toTaskMutation.isPending}
        >
          转为任务
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => toMemoryMutation.mutate()}
          disabled={toMemoryMutation.isPending}
        >
          保存为记忆
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => {
            if (confirm("删除此条剪切板记录？")) deleteMutation.mutate();
          }}
        >
          删除
        </Button>
      </div>
      <dl className="grid grid-cols-2 gap-2 text-[11px] text-muted">
        <div>
          <dt>类型</dt>
          <dd className="text-foreground">{isImage ? "图片" : "文本"}</dd>
        </div>
        <div>
          <dt>复制时间</dt>
          <dd className="text-foreground">{item.createdAt}</dd>
        </div>
        <div>
          <dt>使用次数</dt>
          <dd className="text-foreground">{item.useCount}</dd>
        </div>
        {isImage && item.width && item.height ? (
          <div>
            <dt>尺寸</dt>
            <dd className="text-foreground">
              {item.width}×{item.height}
            </dd>
          </div>
        ) : null}
        {item.sourceApp ? (
          <div className="col-span-2">
            <dt>来源应用</dt>
            <dd className="text-foreground">{item.sourceApp}</dd>
          </div>
        ) : null}
      </dl>
      {isImage ? (
        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-auto">
          {imageSrc ? (
            <img
              src={imageSrc}
              alt="剪切板图片预览"
              className="max-h-64 w-fit max-w-full rounded-[var(--radius-panel)] border border-border object-contain"
            />
          ) : (
            <div className="rounded-[var(--radius-panel)] border border-border bg-surface p-6 text-[12px] text-muted">
              暂无缩略图
            </div>
          )}
          <div>
            <div className="mb-1 text-[11px] text-muted">识别文本（本地 OCR）</div>
            <pre className="min-h-24 overflow-auto whitespace-pre-wrap break-words rounded-[var(--radius-panel)] border border-border bg-surface p-3 text-[13px] leading-relaxed">
              {ocrText || "（未识别到文字，或 OCR 不可用）"}
            </pre>
            {ocrText ? (
              <Button
                size="sm"
                variant="ghost"
                className="mt-2"
                onClick={() => void navigator.clipboard.writeText(ocrText)}
              >
                复制识别文本
              </Button>
            ) : null}
          </div>
        </div>
      ) : (
        <pre className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-words rounded-[var(--radius-panel)] border border-border bg-surface p-3 text-[13px] leading-relaxed">
          {item.content}
        </pre>
      )}
    </div>
  );
}

export function ClipboardPage() {
  useDomainInvalidation();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });

  const listQuery = useQuery({
    queryKey: ["clipboard", favoritesOnly, search],
    queryFn: () =>
      ipc.clipboardQuery({
        favoritesOnly: favoritesOnly || undefined,
        search: search.trim() || undefined,
        limit: 300,
      }),
  });

  const items = listQuery.data ?? [];
  const selected = useMemo(
    () => items.find((item) => item.id === selectedId) ?? null,
    [items, selectedId],
  );

  useEffect(() => {
    if (selectedId && !items.some((item) => item.id === selectedId)) {
      setSelectedId(items[0]?.id ?? null);
    } else if (!selectedId && items[0]) {
      setSelectedId(items[0].id);
    }
  }, [items, selectedId]);

  const capturing = settingsQuery.data?.clipboardCaptureEnabled ?? true;

  const toggleCapture = useMutation({
    mutationFn: (enabled: boolean) => ipc.clipboardSetCaptureEnabled(enabled),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });

  const clearMutation = useMutation({
    mutationFn: () => ipc.clipboardClearNonFavorites(),
    onSuccess: (count) => {
      void queryClient.invalidateQueries({ queryKey: ["clipboard"] });
      alert(`已清空 ${count} 条非收藏记录（收藏已保留）`);
    },
  });

  return (
    <SplitTaskLayout
      title="剪切板"
      description={capturing ? "记录中 · 仅本地保存" : "已暂停采集"}
      actions={
        <>
          <label className="flex items-center gap-1.5 text-[12px] text-muted">
            <input
              type="checkbox"
              checked={favoritesOnly}
              onChange={(e) => setFavoritesOnly(e.target.checked)}
            />
            仅收藏
          </label>
          <Input
            className="h-8 w-44"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索历史…"
          />
          <Button
            size="sm"
            variant={capturing ? "secondary" : "default"}
            onClick={() => toggleCapture.mutate(!capturing)}
          >
            {capturing ? "暂停" : "恢复"}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              if (
                confirm(
                  "清空所有非收藏剪切板记录？\n收藏条目将保留。此操作不可撤销。",
                )
              ) {
                clearMutation.mutate();
              }
            }}
          >
            清空…
          </Button>
        </>
      }
      list={
        listQuery.isLoading ? (
          <div className="p-4 text-[12px] text-muted">加载中…</div>
        ) : items.length === 0 ? (
          <EmptyState
            title={favoritesOnly ? "暂无收藏" : "暂无剪切板历史"}
            body={
              capturing
                ? "复制文本或图片后会出现在这里。密码管理器等应用默认已排除。"
                : "采集已暂停。恢复后才会记录新的复制内容。"
            }
          />
        ) : (
          <ul>
            {items.map((item) => {
              const src = thumbSrc(item);
              return (
                <li key={item.id}>
                  <button
                    type="button"
                    className={cn(
                      "flex w-full items-start gap-2 border-b border-border px-3 py-2.5 text-left hover:bg-row-hover",
                      selectedId === item.id && "bg-row-active",
                    )}
                    onClick={() => setSelectedId(item.id)}
                  >
                    {item.favorite ? (
                      <Star className="mt-0.5 size-3.5 shrink-0 fill-current text-accent" />
                    ) : (
                      <span className="mt-0.5 size-3.5 shrink-0" />
                    )}
                    {item.kind === "image" && src ? (
                      <img
                        src={src}
                        alt=""
                        className="mt-0.5 size-10 shrink-0 rounded border border-border object-cover"
                      />
                    ) : null}
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[13px]">{previewLine(item)}</div>
                      <div className="mt-0.5 text-[11px] text-muted">
                        {item.kind === "image" ? "图片 · " : ""}
                        {item.createdAt}
                        {item.useCount > 0 ? ` · 用过 ${item.useCount} 次` : ""}
                      </div>
                    </div>
                  </button>
                </li>
              );
            })}
          </ul>
        )
      }
      detail={
        <ClipboardDetail
          item={selected}
          onDeleted={() => setSelectedId(null)}
        />
      }
    />
  );
}
