import { useEffect, useMemo, useState, useCallback } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Star } from "lucide-react";
import { EmptyState } from "@/components/PageScaffold";
import { PermissionBanner } from "@/components/PermissionBanner";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";
import { Input } from "@/design-system/primitives/Input";
import { SplitTaskLayout } from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import {
  ipc,
  type ClipboardItem,
  type ClipboardSmartContext,
  type ClipboardTaskDraftInput,
} from "@/ipc/client";
import { cn } from "@/lib/cn";
import {
  ACTION_LABEL,
  actionsForKindHint,
  KIND_HINT_LABEL,
  type SmartAction,
} from "@/lib/clipboard-smart";
import {
  PagedListFooter,
  usePagedQuery,
} from "@/features/shared/usePagedQuery";

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

function taskDraftFromContext(
  ctx: ClipboardSmartContext | undefined,
): ClipboardTaskDraftInput | null {
  if (!ctx?.taskDraft) return null;
  const d = ctx.taskDraft;
  return {
    title: d.title || null,
    notes: d.raw || null,
    dueDate: d.dueDate ?? null,
    dueTime: d.dueTime ?? null,
    priority: d.priority === "none" ? null : d.priority,
  };
}

function ClipboardDetail({
  item,
  smartEnabled,
  onDeleted,
}: {
  item: ClipboardItem | null;
  smartEnabled: boolean;
  onDeleted?: () => void;
}) {
  const queryClient = useQueryClient();
  const [notice, setNotice] = useState<string | null>(null);

  const smartQuery = useQuery({
    queryKey: ["clipboard", "smart", item?.id],
    queryFn: () => ipc.clipboardSmartContext(item!.id),
    enabled: Boolean(item && smartEnabled),
  });

  useEffect(() => {
    setNotice(null);
  }, [item?.id]);

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
    mutationFn: (draft: ClipboardTaskDraftInput | null) =>
      ipc.clipboardConvertToTask(item!.id, draft),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["clipboard"] });
      setNotice("已转为任务（收件箱）");
    },
  });

  const toMemoryMutation = useMutation({
    mutationFn: () => ipc.clipboardConvertToMemory(item!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["memories"] });
      void queryClient.invalidateQueries({ queryKey: ["clipboard"] });
      setNotice("已保存为记忆");
    },
  });

  const linkMutation = useMutation({
    mutationFn: (taskId: string) => ipc.clipboardLinkToTask(item!.id, taskId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["clipboard", "smart"] });
      setNotice("已关联到任务");
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
  const smart = smartQuery.data;
  const draft = taskDraftFromContext(smart);

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      {notice ? (
        <p className="rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 py-1.5 text-[12px] text-foreground">
          {notice}
        </p>
      ) : null}
      {smart?.linkedTaskId ? (
        <p className="text-[11px] text-muted">已关联任务 · ID {smart.linkedTaskId.slice(0, 8)}…</p>
      ) : null}
      {smart?.linkedMemoryId ? (
        <p className="text-[11px] text-muted">已存为记忆 · ID {smart.linkedMemoryId.slice(0, 8)}…</p>
      ) : null}
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
          onClick={() => toTaskMutation.mutate(draft)}
          disabled={toTaskMutation.isPending || Boolean(smart?.linkedTaskId)}
        >
          {item.kindHint === "error" ? "新建排查任务" : "转为任务"}
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => toMemoryMutation.mutate()}
          disabled={toMemoryMutation.isPending || Boolean(smart?.linkedMemoryId)}
        >
          保存为记忆
        </Button>
        <ConfirmButton
          size="sm"
          confirmLabel="确认删除？"
          onConfirm={() => deleteMutation.mutate()}
          resetKey={item.id}
        >
          删除
        </ConfirmButton>
      </div>
      {draft && smartEnabled ? (
        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-3 text-[12px]">
          <h3 className="font-medium">任务草稿预览（NL 解析）</h3>
          <dl className="mt-2 space-y-1 text-muted">
            <div className="flex gap-2">
              <dt className="shrink-0">标题</dt>
              <dd className="text-foreground">{draft.title}</dd>
            </div>
            {draft.dueDate ? (
              <div className="flex gap-2">
                <dt className="shrink-0">截止</dt>
                <dd className="text-foreground">
                  {draft.dueDate}
                  {draft.dueTime ? ` ${draft.dueTime}` : ""}
                </dd>
              </div>
            ) : null}
          </dl>
          {smart?.taskDraft?.ambiguousFields.length ? (
            <p className="mt-2 text-[11px] text-amber-600 dark:text-amber-400">
              待确认：{smart.taskDraft.ambiguousFields.join("、")}
            </p>
          ) : null}
        </section>
      ) : null}
      {smartEnabled && (smart?.similarTasks.length ?? 0) > 0 ? (
        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-3">
          <h3 className="text-[12px] font-medium">相似任务建议</h3>
          <ul className="mt-2 space-y-1">
            {smart!.similarTasks.map((hit) => (
              <li key={hit.taskId} className="flex items-center justify-between gap-2 text-[12px]">
                <span className="truncate">{hit.title}</span>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={linkMutation.isPending}
                  onClick={() => linkMutation.mutate(hit.taskId)}
                >
                  关联
                </Button>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      <dl className="grid grid-cols-2 gap-2 text-[11px] text-muted">
        <div>
          <dt>类型</dt>
          <dd className="text-foreground">{isImage ? "图片" : "文本"}</dd>
        </div>
        <div>
          <dt>智能分类</dt>
          <dd className="text-foreground">{KIND_HINT_LABEL[item.kindHint]}</dd>
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

function ClipboardListRow({
  item,
  selected,
  smartEnabled,
  onSelect,
  onAction,
}: {
  item: ClipboardItem;
  selected: boolean;
  smartEnabled: boolean;
  onSelect: () => void;
  onAction: (action: SmartAction, item: ClipboardItem) => void;
}) {
  const src = thumbSrc(item);
  const actions = smartEnabled ? actionsForKindHint(item.kindHint) : [];

  return (
    <li>
      <button
        type="button"
        className={cn(
          "flex w-full items-start gap-2 border-b border-border px-3 py-2.5 text-left hover:bg-row-hover",
          selected && "bg-row-active",
        )}
        onClick={onSelect}
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
          <div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-muted">
            <span>
              {item.kind === "image" ? "图片 · " : ""}
              {KIND_HINT_LABEL[item.kindHint]}
              {" · "}
              {item.createdAt}
              {item.useCount > 0 ? ` · 用过 ${item.useCount} 次` : ""}
            </span>
          </div>
          {actions.length > 0 ? (
            <div className="mt-1.5 flex flex-wrap gap-1">
              {actions.map((action) => (
                <button
                  key={action}
                  type="button"
                  className="rounded-full border border-border bg-surface-raised px-2 py-0.5 text-[10px] hover:bg-row-hover"
                  onClick={(e) => {
                    e.stopPropagation();
                    onAction(action, item);
                  }}
                >
                  {action === "task" && item.kindHint === "error"
                    ? "排查任务"
                    : ACTION_LABEL[action]}
                </button>
              ))}
            </div>
          ) : null}
        </div>
      </button>
    </li>
  );
}

export function ClipboardPage() {
  useDomainInvalidation();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [codeOnly, setCodeOnly] = useState(false);
  const [sourceApp, setSourceApp] = useState<string>("all");
  const [dateRange, setDateRange] = useState<"all" | "7d" | "30d">("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [listNotice, setListNotice] = useState<string | null>(null);

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });

  const healthQuery = useQuery({
    queryKey: ["app", "health"],
    queryFn: () => ipc.appHealth(),
  });

  const sourceAppsQuery = useQuery({
    queryKey: ["clipboard", "source-apps"],
    queryFn: () => ipc.clipboardListSourceApps(),
  });

  const dateFilter = useMemo(() => {
    if (dateRange === "all") return { dateFrom: undefined, dateTo: undefined };
    const to = new Date();
    const from = new Date();
    from.setDate(from.getDate() - (dateRange === "7d" ? 7 : 30));
    const fmt = (d: Date) => d.toISOString().slice(0, 10);
    return { dateFrom: fmt(from), dateTo: fmt(to) };
  }, [dateRange]);

  const fetchClipboard = useCallback(
    (offset: number, limit: number) =>
      ipc.clipboardQuery({
        favoritesOnly: favoritesOnly || undefined,
        search: search.trim() || undefined,
        sourceApp: sourceApp === "all" ? undefined : sourceApp,
        dateFrom: dateFilter.dateFrom,
        dateTo: dateFilter.dateTo,
        kindHint: codeOnly ? "code" : undefined,
        limit,
        offset,
      }),
    [codeOnly, dateFilter.dateFrom, dateFilter.dateTo, favoritesOnly, search, sourceApp],
  );

  const {
    items,
    total: clipTotal,
    hasMore: clipHasMore,
    loading: clipLoading,
    loadingMore: clipLoadingMore,
    loadMore: loadMoreClipboard,
  } = usePagedQuery(
    ["clipboard", favoritesOnly, codeOnly, search, sourceApp, dateRange],
    fetchClipboard,
    300,
  );
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
  const smartEnabled = settingsQuery.data?.clipboardSmartActionsEnabled ?? true;

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

  const handleListAction = useCallback(
    async (action: SmartAction, item: ClipboardItem) => {
      setSelectedId(item.id);
      try {
        if (action === "copy") {
          await ipc.clipboardCopy(item.id);
          setListNotice("已复制到系统剪贴板");
        } else if (action === "memory") {
          await ipc.clipboardConvertToMemory(item.id);
          void queryClient.invalidateQueries({ queryKey: ["memories"] });
          setListNotice("已保存为记忆");
        } else if (action === "task") {
          const ctx = smartEnabled
            ? await ipc.clipboardSmartContext(item.id)
            : null;
          const draft = taskDraftFromContext(ctx ?? undefined);
          await ipc.clipboardConvertToTask(item.id, draft);
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
          setListNotice("已转为任务");
        }
        void queryClient.invalidateQueries({ queryKey: ["clipboard"] });
      } catch {
        setListNotice("操作失败，请重试");
      }
    },
    [queryClient, smartEnabled],
  );

  return (
    <>
      {!capturing ? (
        <PermissionBanner
          kind="clipboard_paused"
          title="剪切板采集已暂停"
          body="不会记录新的复制内容；历史仍可浏览。"
          primaryAction={{
            label: "恢复采集",
            onClick: () => toggleCapture.mutate(true),
          }}
        />
      ) : null}
      {healthQuery.data?.capabilities.ocr.available === false ? (
        <p className="border-b border-border px-4 py-1.5 text-[11px] text-muted">
          当前平台不支持图片文字识别（OCR），图片无法按文字搜索。
        </p>
      ) : null}
      {listNotice ? (
        <p className="border-b border-border px-4 py-1.5 text-[11px] text-muted">{listNotice}</p>
      ) : null}
      <SplitTaskLayout
        title="剪切板"
        description={
          capturing
            ? smartEnabled
              ? "记录中 · 智能行动已开启"
              : "记录中 · 仅本地保存"
            : "已暂停采集"
        }
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
            <label className="flex items-center gap-1.5 text-[12px] text-muted">
              <input
                type="checkbox"
                checked={codeOnly}
                onChange={(e) => setCodeOnly(e.target.checked)}
              />
              代码片段
            </label>
            <Input
              className="h-8 w-44"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="搜索历史…"
            />
            <select
              className="h-8 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
              value={sourceApp}
              onChange={(e) => setSourceApp(e.target.value)}
            >
              <option value="all">全部来源</option>
              {(sourceAppsQuery.data ?? []).map((app) => (
                <option key={app} value={app}>
                  {app}
                </option>
              ))}
            </select>
            <select
              className="h-8 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
              value={dateRange}
              onChange={(e) =>
                setDateRange(e.target.value as "all" | "7d" | "30d")
              }
            >
              <option value="all">全部时间</option>
              <option value="7d">最近 7 天</option>
              <option value="30d">最近 30 天</option>
            </select>
            <Button
              size="sm"
              variant={capturing ? "secondary" : "default"}
              onClick={() => toggleCapture.mutate(!capturing)}
            >
              {capturing ? "暂停" : "恢复"}
            </Button>
            <ConfirmButton
              size="sm"
              confirmLabel="确认清空？"
              confirmTitle="清空所有非收藏剪切板记录？收藏条目将保留。"
              onConfirm={() => clearMutation.mutate()}
            >
              清空…
            </ConfirmButton>
          </>
        }
        list={
          clipLoading ? (
            <div className="p-4 text-[12px] text-muted">加载中…</div>
          ) : items.length === 0 ? (
            <EmptyState
              title={codeOnly ? "暂无代码片段" : favoritesOnly ? "暂无收藏" : "暂无剪切板历史"}
              body={
                codeOnly
                  ? "复制含代码特征的内容后会自动归类为代码片段。"
                  : favoritesOnly
                    ? "在历史条目上点亮星标，收藏不会随过期清理删除。"
                    : capturing
                      ? "复制文本或图片后会出现在这里。密码管理器等应用默认已排除。"
                      : "采集已暂停，恢复后才会记录新的复制内容。"
              }
              primaryAction={
                codeOnly
                  ? { label: "显示全部", onClick: () => setCodeOnly(false) }
                  : favoritesOnly
                    ? {
                        label: "显示全部历史",
                        onClick: () => setFavoritesOnly(false),
                      }
                    : !capturing
                      ? {
                          label: "恢复采集",
                          onClick: () => toggleCapture.mutate(true),
                        }
                      : undefined
              }
            />
          ) : (
            <ul>
              {items.map((item) => (
                <ClipboardListRow
                  key={item.id}
                  item={item}
                  selected={selectedId === item.id}
                  smartEnabled={smartEnabled}
                  onSelect={() => setSelectedId(item.id)}
                  onAction={handleListAction}
                />
              ))}
              <PagedListFooter
                shown={items.length}
                total={clipTotal}
                hasMore={clipHasMore}
                loadingMore={clipLoadingMore}
                onLoadMore={loadMoreClipboard}
              />
            </ul>
          )
        }
        detail={
          <ClipboardDetail
            item={selected}
            smartEnabled={smartEnabled}
            onDeleted={() => setSelectedId(null)}
          />
        }
      />
    </>
  );
}
