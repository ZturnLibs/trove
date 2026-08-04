import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Pin } from "lucide-react";
import { EmptyState } from "@/components/PageScaffold";
import { AttachmentsSection } from "@/design-system/patterns/AttachmentsSection";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import {
  NewTaskButton,
  SplitTaskLayout,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { ipc, type Memory, type UpdateMemoryInput } from "@/ipc/client";
import { cn } from "@/lib/cn";

function linkify(text: string) {
  const parts = text.split(/(https?:\/\/[^\s]+)/g);
  return parts.map((part, index) =>
    /^https?:\/\//.test(part) ? (
      <a
        key={`${part}-${index}`}
        href={part}
        className="text-accent underline"
        onClick={(e) => {
          e.preventDefault();
          void navigator.clipboard.writeText(part);
        }}
        title="点击复制链接"
      >
        {part}
      </a>
    ) : (
      <span key={`${index}`}>{part}</span>
    ),
  );
}

function MemoryDetail({
  memory,
  onDeleted,
  focusTitleId,
}: {
  memory: Memory | null;
  onDeleted?: () => void;
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
          <div className="whitespace-pre-wrap rounded-[var(--radius-control)] border border-border bg-surface p-3 text-[13px] leading-relaxed">
            {linkify(draft.body || "（空）")}
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
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              const count = (linksQuery.data ?? []).length;
              const message =
                count > 0
                  ? `确认删除此记忆？\n将移除 ${count} 个关联资源；资源文件按保留规则保留。`
                  : "确认删除此记忆？";
              if (confirm(message)) deleteMutation.mutate();
            }}
          >
            删除
          </Button>
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
  const [createdId, setCreatedId] = useState<string | null>(null);

  const memoriesQuery = useQuery({
    queryKey: ["memories", { pinnedOnly }],
    queryFn: () => ipc.memoryQuery({ pinnedOnly }),
  });

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

  return (
    <SplitTaskLayout
      title="记忆"
      description="短小、可检索的信息片段"
      actions={
        <>
          <Button
            size="sm"
            variant={pinnedOnly ? "default" : "secondary"}
            onClick={() => setPinnedOnly((v) => !v)}
          >
            仅置顶
          </Button>
          <NewTaskButton onClick={() => createMutation.mutate()} />
        </>
      }
      list={
        memoriesQuery.isLoading ? (
          <div className="p-4 text-[12px] text-muted">加载中…</div>
        ) : (memoriesQuery.data?.length ?? 0) === 0 ? (
          <EmptyState
            title="还没有记忆"
            body="适合保存短文本、链接和可检索的片段；也可稍后转为任务。"
            primaryAction={{
              label: "新建记忆",
              onClick: () => createMutation.mutate(),
            }}
            hint="快速记录里可切换到「记忆」"
          />
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
                  <div className="truncate text-[13px] font-medium">{memory.title}</div>
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
        <MemoryDetail
          memory={selected}
          onDeleted={() => setSelectedId(null)}
          focusTitleId={createdId}
        />
      }
    />
  );
}
