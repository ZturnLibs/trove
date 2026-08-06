import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Pin } from "lucide-react";
import { MarkdownView } from "@/components/MarkdownView";
import { EmptyState } from "@/components/PageScaffold";
import { AttachmentsSection } from "@/design-system/patterns/AttachmentsSection";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";
import { Input } from "@/design-system/primitives/Input";
import {
  NewTaskButton,
  SplitTaskLayout,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { ipc, type Memory, type UpdateMemoryInput } from "@/ipc/client";
import { cn } from "@/lib/cn";

function MemoryDetail({
  memory,
  onDeleted,
  onArchived,
  focusTitleId,
}: {
  memory: Memory | null;
  onDeleted?: () => void;
  /** 归档/恢复成功后回调（默认视图归档后该项从列表消失，需清空选中避免幽灵项）。 */
  onArchived?: () => void;
  /** When set to the memory's id (e.g. right after "新建"), focus + select its title. */
  focusTitleId?: string | null;
}) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<UpdateMemoryInput | null>(null);
  const [tagText, setTagText] = useState("");
  const [preview, setPreview] = useState(false);
  const titleRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!memory) {
      setDraft(null);
      return;
    }
    setDraft({
      id: memory.id,
      title: memory.title,
      body: memory.body,
      pinned: memory.pinned,
      archived: memory.archived,
      quickInsert: memory.quickInsert,
      triggerWord: memory.triggerWord,
      tagNames: [...memory.tagNames],
    });
    setTagText(memory.tagNames.join(", "));
  }, [memory]);

  // Newly created memory: focus + select the title so typing replaces "新记忆".
  useEffect(() => {
    if (memory && focusTitleId && memory.id === focusTitleId) {
      requestAnimationFrame(() => {
        titleRef.current?.focus();
        titleRef.current?.select();
      });
    }
  }, [memory, focusTitleId]);

  const saveMutation = useMutation({
    mutationFn: (input: UpdateMemoryInput) => ipc.memoryUpdate(input),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["memories"] }),
  });

  const deleteMutation = useMutation({
    mutationFn: () => ipc.memoryDelete(memory!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["memories"] });
      onDeleted?.();
    },
  });

  const archiveMutation = useMutation({
    mutationFn: () =>
      ipc.memoryUpdate({ ...draft!, archived: !draft!.archived }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["memories"] });
      onArchived?.();
    },
  });

  const convertMutation = useMutation({
    mutationFn: () => ipc.memoryConvertToTask(memory!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["memories"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      alert("已转为任务，原记忆已保留");
    },
  });

  const linksQuery = useQuery({
    queryKey: ["links", "memory", memory?.id],
    queryFn: () => ipc.entityLinkList("memory", memory!.id),
    enabled: !!memory,
  });

  if (!memory || !draft) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-[12px] text-muted">
        选择一项查看详情
      </div>
    );
  }

  const save = () => {
    const tagNames = tagText
      .split(/[,，]/)
      .map((t) => t.trim())
      .filter(Boolean);
    saveMutation.mutate({ ...draft, tagNames });
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between border-b border-border px-3 py-2 text-[11px] text-muted">
        <span>记忆 · {memory.updatedAt.slice(0, 16).replace("T", " ")}</span>
        <Button size="sm" variant="ghost" onClick={() => setPreview((v) => !v)}>
          {preview ? "编辑" : "预览"}
        </Button>
      </div>
      <div className="min-h-0 flex-1 space-y-3 overflow-auto p-3">
        <Input
          ref={titleRef}
          value={draft.title}
          onChange={(e) => setDraft({ ...draft, title: e.target.value })}
          onBlur={save}
          className="h-9 text-[14px] font-medium"
        />
        <label className="block space-y-1 text-[11px] text-muted">
          标签
          <Input
            value={tagText}
            onChange={(e) => setTagText(e.target.value)}
            onBlur={save}
            placeholder="工作, 灵感"
          />
        </label>
        <label className="flex items-center gap-2 text-[12px] text-foreground">
          <input
            type="checkbox"
            checked={draft.quickInsert}
            onChange={(e) => {
              const next = { ...draft, quickInsert: e.target.checked };
              setDraft(next);
              saveMutation.mutate({
                ...next,
                tagNames: tagText
                  .split(/[,，]/)
                  .map((t) => t.trim())
                  .filter(Boolean),
              });
            }}
          />
          可快速插入（文本片段）
        </label>
        {draft.quickInsert ? (
          <label className="block space-y-1 text-[11px] text-muted">
            触发词
            <Input
              value={draft.triggerWord ?? ""}
              onChange={(e) =>
                setDraft({ ...draft, triggerWord: e.target.value || null })
              }
              onBlur={save}
              placeholder="如 addr / 签名"
            />
          </label>
        ) : null}
        {preview ? (
          <div className="rounded-[var(--radius-control)] border border-border bg-surface p-3">
            {draft.body ? (
              <MarkdownView markdown={draft.body} />
            ) : (
              <div className="text-[13px] text-muted">（空）</div>
            )}
          </div>
        ) : (
          <textarea
            value={draft.body}
            onChange={(e) => setDraft({ ...draft, body: e.target.value })}
            onBlur={save}
            rows={14}
            className="w-full resize-none rounded-[var(--radius-control)] border border-border bg-surface-raised p-2 font-mono text-[13px] text-foreground outline-none focus:ring-2 focus:ring-accent/35"
            placeholder="支持基础 Markdown 文本…"
          />
        )}

        <AttachmentsSection entityType="memory" entityId={memory.id} />
      </div>
      <div className="flex items-center justify-between gap-2 border-t border-border p-3">
        <div className="flex gap-1">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              const next = { ...draft, pinned: !draft.pinned };
              setDraft(next);
              saveMutation.mutate({
                ...next,
                tagNames: tagText
                  .split(/[,，]/)
                  .map((t) => t.trim())
                  .filter(Boolean),
              });
            }}
          >
            {draft.pinned ? "取消置顶" : "置顶"}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => void navigator.clipboard.writeText(draft.body)}
          >
            复制
          </Button>
          <ConfirmButton
            size="sm"
            confirmLabel={draft.archived ? "确认恢复？" : "确认归档？"}
            confirmVariant="secondary"
            onConfirm={() => archiveMutation.mutate()}
            resetKey={memory.id}
            disabled={archiveMutation.isPending}
          >
            {draft.archived ? "恢复" : "归档"}
          </ConfirmButton>
          <ConfirmButton
            size="sm"
            confirmLabel={
              (linksQuery.data?.length ?? 0) > 0
                ? `确认删除？(${(linksQuery.data ?? []).length} 关联)`
                : "确认删除？"
            }
            onConfirm={() => deleteMutation.mutate()}
            resetKey={memory.id}
          >
            删除
          </ConfirmButton>
        </div>
        <Button
          size="sm"
          onClick={() => convertMutation.mutate()}
          disabled={convertMutation.isPending}
        >
          转为任务
        </Button>
      </div>
    </div>
  );
}

export function MemoryPage() {
  useDomainInvalidation();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [pinnedOnly, setPinnedOnly] = useState(false);
  const [showArchived, setShowArchived] = useState(false);
  const [tagId, setTagId] = useState<string>("all");
  const [searchText, setSearchText] = useState("");
  const [search, setSearch] = useState("");
  const [createdId, setCreatedId] = useState<string | null>(null);
  const [view, setView] = useState<"memory" | "notes">("memory");
  const [noteDraft, setNoteDraft] = useState("");

  // 防抖：停止输入 250ms 后触发查询。
  useEffect(() => {
    const timer = window.setTimeout(() => setSearch(searchText), 250);
    return () => window.clearTimeout(timer);
  }, [searchText]);

  const tagsQuery = useQuery({
    queryKey: ["task-tags"],
    queryFn: () => ipc.taskListTags(),
  });

  const memoriesQuery = useQuery({
    queryKey: ["memories", { pinnedOnly, tagId, showArchived, search }],
    queryFn: () =>
      ipc.memoryQuery({
        pinnedOnly: !showArchived && pinnedOnly ? true : undefined,
        tagId: tagId === "all" ? undefined : tagId,
        includeArchived: showArchived ? true : undefined,
        search: search.trim() || undefined,
      }),
  });

  const smokeNotesQuery = useQuery({
    queryKey: ["smoke-notes"],
    queryFn: () => ipc.smokeNoteList(),
    enabled: view === "notes",
  });

  const createNoteMutation = useMutation({
    mutationFn: () => ipc.smokeNoteCreate(noteDraft),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["smoke-notes"] });
      setNoteDraft("");
    },
  });

  const deleteNoteMutation = useMutation({
    mutationFn: (id: string) => ipc.smokeNoteDelete(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["smoke-notes"] });
    },
  });

  // 新建的记忆出现在列表后解除 createdId 锁定，使后续筛选变化可正常清空选中。
  useEffect(() => {
    if (!createdId) return;
    if (memoriesQuery.data?.some((m) => m.id === createdId)) {
      setCreatedId(null);
    }
  }, [createdId, memoriesQuery.data]);

  // 选中项因搜索/标签/归档视图变化而不可见时，清空选中，避免详情显示幽灵项。
  useEffect(() => {
    const data = memoriesQuery.data;
    if (!data || !selectedId) return;
    if (createdId === selectedId) return;
    if (!data.some((m) => m.id === selectedId)) {
      setSelectedId(null);
    }
  }, [selectedId, memoriesQuery.data, createdId]);

  const createMutation = useMutation({
    mutationFn: () => ipc.memoryCreate({ title: "新记忆", body: "" }),
    onSuccess: (memory) => {
      void queryClient.invalidateQueries({ queryKey: ["memories"] });
      setSelectedId(memory.id);
      setCreatedId(memory.id);
    },
  });

  const selected = useMemo(
    () => memoriesQuery.data?.find((m) => m.id === selectedId) ?? null,
    [memoriesQuery.data, selectedId],
  );

  const hasFilters = search.trim() !== "" || tagId !== "all";

  const clearFilters = () => {
    setSearchText("");
    setSearch("");
    setTagId("all");
  };

  return (
    <SplitTaskLayout
      title="记忆"
      description="短小、可检索的信息片段"
      actions={
        <>
          <Button
            size="sm"
            variant={view === "memory" ? "default" : "secondary"}
            onClick={() => setView("memory")}
          >
            记忆
          </Button>
          <Button
            size="sm"
            variant={view === "notes" ? "default" : "secondary"}
            onClick={() => setView("notes")}
          >
            随手记
          </Button>
          {view === "memory" ? (
            <>
              <Input
                type="search"
                className="h-7 w-40"
                value={searchText}
                onChange={(e) => setSearchText(e.target.value)}
                placeholder="搜索标题/正文…"
              />
              <select
                className="h-7 max-w-28 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
                value={tagId}
                onChange={(e) => setTagId(e.target.value)}
                title="按标签筛选"
              >
                <option value="all">全部标签</option>
                {(tagsQuery.data ?? []).map((tag) => (
                  <option key={tag.id} value={tag.id}>
                    {tag.name}
                  </option>
                ))}
              </select>
              <Button
                size="sm"
                variant={showArchived ? "default" : "secondary"}
                onClick={() => {
                  setShowArchived((v) => !v);
                  setPinnedOnly(false);
                }}
              >
                归档视图
              </Button>
              {!showArchived ? (
                <Button
                  size="sm"
                  variant={pinnedOnly ? "default" : "secondary"}
                  onClick={() => setPinnedOnly((v) => !v)}
                >
                  仅置顶
                </Button>
              ) : null}
              <NewTaskButton onClick={() => createMutation.mutate()} />
            </>
          ) : null}
        </>
      }
      list={
        view === "notes" ? (
          <div className="flex h-full flex-col">
            <div className="border-b border-border p-2">
              <Input
                value={noteDraft}
                onChange={(e) => setNoteDraft(e.target.value)}
                placeholder="随手记…"
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    if (noteDraft.trim() && !createNoteMutation.isPending) {
                      createNoteMutation.mutate();
                    }
                  }
                }}
              />
            </div>
            <div className="min-h-0 flex-1 overflow-auto">
              {smokeNotesQuery.isLoading ? (
                <div className="p-4 text-[12px] text-muted">加载中…</div>
              ) : (smokeNotesQuery.data?.length ?? 0) === 0 ? (
                <EmptyState
                  title="还没有随手记"
                  body="在上方输入后按回车，快速记录一条。"
                />
              ) : (
                <ul>
                  {smokeNotesQuery.data?.map((note) => (
                    <li
                      key={note.id}
                      className="flex items-start gap-2 border-b border-border px-3 py-2"
                    >
                      <div className="min-w-0 flex-1">
                        <div className="whitespace-pre-wrap break-words text-[13px]">
                          {note.body}
                        </div>
                        <div className="text-[11px] text-muted">
                          {note.updatedAt.slice(0, 16).replace("T", " ")}
                        </div>
                      </div>
                      <ConfirmButton
                        size="sm"
                        confirmLabel="确认删除？"
                        onConfirm={() => deleteNoteMutation.mutate(note.id)}
                        resetKey={note.id}
                        disabled={deleteNoteMutation.isPending}
                      >
                        删除
                      </ConfirmButton>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        ) : memoriesQuery.isLoading ? (
          <div className="p-4 text-[12px] text-muted">加载中…</div>
        ) : (memoriesQuery.data?.length ?? 0) === 0 ? (
          hasFilters ? (
            <EmptyState
              title="没有匹配的记忆"
              body="换个关键词或标签试试。"
              secondaryAction={{
                label: "清除筛选",
                onClick: clearFilters,
              }}
            />
          ) : showArchived ? (
            <EmptyState
              title="没有已归档的记忆"
              body="在默认视图里对记忆点「归档」，可稍后在这里恢复。"
            />
          ) : (
            <EmptyState
              title="还没有记忆"
              body="适合保存短文本、链接和可检索的片段；也可稍后转为任务。"
              primaryAction={{
                label: "新建记忆",
                onClick: () => createMutation.mutate(),
              }}
              hint="快速记录里可切换到「记忆」"
            />
          )
        ) : (
          <div>
            {memoriesQuery.data?.map((memory) => (
              <button
                key={memory.id}
                type="button"
                onClick={() => setSelectedId(memory.id)}
                className={cn(
                  "flex w-full items-start gap-2 border-b border-border px-3 py-2 text-left hover:bg-row-hover",
                  selectedId === memory.id && "bg-row-active",
                )}
              >
                {memory.pinned ? (
                  <Pin className="mt-0.5 h-3.5 w-3.5 shrink-0 text-accent" />
                ) : (
                  <span className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                )}
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1.5">
                    <span className="truncate text-[13px] font-medium">
                      {memory.title}
                    </span>
                    {showArchived && memory.archived ? (
                      <span className="shrink-0 rounded border border-border bg-surface-raised px-1 py-px text-[10px] text-muted">
                        归档
                      </span>
                    ) : null}
                  </div>
                  <div className="truncate text-[11px] text-muted">
                    {memory.body || "（无正文）"}
                  </div>
                </div>
              </button>
            ))}
          </div>
        )
      }
      detail={
        view === "notes" ? (
          <div className="flex h-full items-center justify-center p-6 text-center text-[12px] text-muted">
            随手记在左侧快速记录与删除
          </div>
        ) : (
          <MemoryDetail
            memory={selected}
            onDeleted={() => setSelectedId(null)}
            onArchived={() => setSelectedId(null)}
            focusTitleId={createdId}
          />
        )
      }
    />
  );
}
