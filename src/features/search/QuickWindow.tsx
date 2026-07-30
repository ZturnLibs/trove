import { useEffect, useMemo, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import {
  ipc,
  type SearchEntityType,
  type SearchHit,
  type TaskPriority,
} from "@/ipc/client";
import { useUiStore, type QuickMode } from "@/stores/ui";
import { cn } from "@/lib/cn";

const modes: { id: QuickMode; label: string }[] = [
  { id: "capture", label: "记录" },
  { id: "search", label: "搜索" },
  { id: "clip", label: "剪切板" },
];

type CaptureType = "task" | "reminder" | "memory";

const typeLabel: Record<SearchEntityType, string> = {
  task: "任务",
  reminder: "提醒",
  memory: "记忆",
  clipboard: "剪切板",
};

export function QuickWindow() {
  const mode = useUiStore((s) => s.quickMode);
  const setQuickMode = useUiStore((s) => s.setQuickMode);
  const [captureType, setCaptureType] = useState<CaptureType>("task");
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [dueDate, setDueDate] = useState("");
  const [fireAt, setFireAt] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("none");
  const [daily, setDaily] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchText, setSearchText] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [clipSearch, setClipSearch] = useState("");
  const [clipIndex, setClipIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<string>("quick://set-mode", (event) => {
      const next = event.payload;
      if (next === "capture" || next === "search" || next === "clip") {
        setQuickMode(next);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [setQuickMode]);

  useEffect(() => {
    inputRef.current?.focus();
    setError(null);
  }, [mode, captureType]);

  const searchQuery = useQuery({
    queryKey: ["search", searchText],
    queryFn: () => ipc.searchQuery(searchText),
    enabled: mode === "search" && searchText.trim().length > 0,
  });

  const clipQuery = useQuery({
    queryKey: ["clipboard", "quick", clipSearch],
    queryFn: () =>
      ipc.clipboardQuery({
        search: clipSearch.trim() || undefined,
        limit: 40,
      }),
    enabled: mode === "clip",
  });

  const flatResults = useMemo(() => {
    const data = searchQuery.data;
    if (!data) return [] as SearchHit[];
    return [
      ...data.tasks,
      ...data.reminders,
      ...data.memories,
      ...data.clipboard,
    ];
  }, [searchQuery.data]);

  const clipItems = clipQuery.data ?? [];

  useEffect(() => {
    setSelectedIndex(0);
  }, [searchText, flatResults.length]);

  useEffect(() => {
    setClipIndex(0);
  }, [clipSearch, clipItems.length]);

  const openHit = async (hit: SearchHit) => {
    if (hit.entityType === "clipboard") {
      await ipc.clipboardCopy(hit.entityId);
      await ipc.windowHideQuick();
      return;
    }
    await ipc.windowShowMain();
    const path =
      hit.entityType === "task"
        ? "/inbox"
        : hit.entityType === "memory"
          ? "/memory"
          : "/today";
    await emit("main://navigate", path);
    await ipc.windowHideQuick();
  };

  const reuseClip = async (id: string) => {
    await ipc.clipboardCopy(id);
    await ipc.windowHideQuick();
  };

  const submit = async () => {
    if (mode !== "capture") return;
    const value = title.trim();
    if (!value) return;
    setSaving(true);
    setError(null);
    try {
      if (captureType === "task") {
        await ipc.taskCreate({
          title: value,
          dueDate: dueDate || null,
          priority: priority === "none" ? undefined : priority,
        });
      } else if (captureType === "reminder") {
        if (!fireAt) throw new Error("请选择提醒时间");
        const normalized = fireAt.length === 16 ? `${fireAt}:00` : fireAt;
        await ipc.reminderCreate({
          title: value,
          fireAt: normalized.replace(" ", "T"),
          timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          recurrence: daily
            ? {
                version: 1,
                frequency: "daily",
                interval: 1,
                timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
              }
            : null,
        });
      } else {
        await ipc.memoryCreate({
          title: value,
          body: body || undefined,
        });
      }
      setTitle("");
      setBody("");
      setDueDate("");
      setFireAt("");
      setPriority("none");
      setDaily(false);
      await ipc.windowHideQuick();
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建失败");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex h-full flex-col bg-surface text-foreground">
      <div className="flex items-center gap-1 border-b border-border px-2 py-2">
        {modes.map((item) => (
          <Button
            key={item.id}
            size="sm"
            variant={mode === item.id ? "default" : "ghost"}
            onClick={() => setQuickMode(item.id)}
          >
            {item.label}
          </Button>
        ))}
        <div className="flex-1" />
        <Button
          size="sm"
          variant="ghost"
          onClick={() => void ipc.windowHideQuick()}
        >
          Esc
        </Button>
      </div>

      <div className="flex flex-1 flex-col gap-3 p-3">
        {mode === "capture" ? (
          <>
            <div className="flex gap-1 text-[12px]">
              {(["task", "reminder", "memory"] as const).map((type) => (
                <button
                  key={type}
                  type="button"
                  className={cn(
                    "rounded-[var(--radius-control)] px-2 py-0.5",
                    captureType === type
                      ? "bg-row-active text-foreground"
                      : "text-muted hover:bg-row-hover",
                  )}
                  onClick={() => setCaptureType(type)}
                >
                  {type === "task" ? "任务" : type === "reminder" ? "提醒" : "记忆"}
                </button>
              ))}
            </div>
            <Input
              ref={inputRef}
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder={
                captureType === "task"
                  ? "快速记录任务…"
                  : captureType === "reminder"
                    ? "快速记录提醒…"
                    : "快速记录记忆标题…"
              }
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  if (title || body) {
                    setTitle("");
                    setBody("");
                  } else void ipc.windowHideQuick();
                }
                if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                  event.preventDefault();
                  void submit();
                }
              }}
            />
            {captureType === "task" ? (
              <div className="grid grid-cols-2 gap-2">
                <label className="space-y-1 text-[11px] text-muted">
                  截止日期
                  <Input
                    type="date"
                    value={dueDate}
                    onChange={(e) => setDueDate(e.target.value)}
                  />
                </label>
                <label className="space-y-1 text-[11px] text-muted">
                  优先级
                  <select
                    className="h-8 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[13px] text-foreground"
                    value={priority}
                    onChange={(e) => setPriority(e.target.value as TaskPriority)}
                  >
                    <option value="none">无</option>
                    <option value="low">低</option>
                    <option value="medium">中</option>
                    <option value="high">高</option>
                  </select>
                </label>
              </div>
            ) : null}
            {captureType === "reminder" ? (
              <div className="grid grid-cols-2 gap-2">
                <label className="space-y-1 text-[11px] text-muted">
                  提醒时间
                  <Input
                    type="datetime-local"
                    value={fireAt}
                    onChange={(e) => setFireAt(e.target.value)}
                  />
                </label>
                <label className="flex items-end gap-2 pb-1 text-[12px] text-muted">
                  <input
                    type="checkbox"
                    checked={daily}
                    onChange={(e) => setDaily(e.target.checked)}
                  />
                  每天重复
                </label>
              </div>
            ) : null}
            {captureType === "memory" ? (
              <textarea
                value={body}
                onChange={(e) => setBody(e.target.value)}
                rows={6}
                className="w-full resize-none rounded-[var(--radius-control)] border border-border bg-surface-raised p-2 text-[13px] outline-none focus:ring-2 focus:ring-accent/35"
                placeholder="记忆正文（可选）…"
              />
            ) : null}
            {error ? <p className="text-[12px] text-danger">{error}</p> : null}
            <div className="mt-auto flex items-center justify-between text-[11px] text-muted">
              <span>⌘/Ctrl + Enter 创建 · Esc 取消</span>
              <Button
                size="sm"
                disabled={
                  saving ||
                  !title.trim() ||
                  (captureType === "reminder" && !fireAt)
                }
                onClick={() => void submit()}
              >
                {captureType === "task"
                  ? "创建任务"
                  : captureType === "reminder"
                    ? "创建提醒"
                    : "创建记忆"}
              </Button>
            </div>
          </>
        ) : null}

        {mode === "search" ? (
          <>
            <Input
              ref={inputRef}
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
              placeholder="搜索任务、提醒、记忆、剪切板…"
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  if (searchText) setSearchText("");
                  else void ipc.windowHideQuick();
                }
                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setSelectedIndex((i) =>
                    Math.min(i + 1, Math.max(flatResults.length - 1, 0)),
                  );
                }
                if (e.key === "ArrowUp") {
                  e.preventDefault();
                  setSelectedIndex((i) => Math.max(i - 1, 0));
                }
                if (e.key === "Enter" && flatResults[selectedIndex]) {
                  e.preventDefault();
                  void openHit(flatResults[selectedIndex]);
                }
              }}
            />
            <div className="min-h-0 flex-1 overflow-auto rounded-[var(--radius-panel)] border border-border">
              {!searchText.trim() ? (
                <div className="p-4 text-center text-[12px] text-muted">
                  输入关键词开始搜索
                </div>
              ) : searchQuery.isLoading ? (
                <div className="p-4 text-[12px] text-muted">搜索中…</div>
              ) : flatResults.length === 0 ? (
                <div className="p-4 text-center text-[12px] text-muted">无结果</div>
              ) : (
                <ul>
                  {flatResults.map((hit, index) => (
                    <li key={`${hit.entityType}-${hit.entityId}`}>
                      <button
                        type="button"
                        className={cn(
                          "flex w-full flex-col gap-0.5 border-b border-border px-3 py-2 text-left hover:bg-row-hover",
                          index === selectedIndex && "bg-row-active",
                        )}
                        onClick={() => void openHit(hit)}
                        onMouseEnter={() => setSelectedIndex(index)}
                      >
                        <div className="flex items-center gap-2 text-[13px]">
                          <span className="rounded bg-row-hover px-1.5 text-[10px] text-muted">
                            {typeLabel[hit.entityType]}
                          </span>
                          <span className="truncate font-medium">{hit.title}</span>
                        </div>
                        {hit.snippet ? (
                          <div className="truncate text-[11px] text-muted">
                            {hit.snippet}
                          </div>
                        ) : null}
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </>
        ) : null}

        {mode === "clip" ? (
          <>
            <Input
              ref={inputRef}
              value={clipSearch}
              onChange={(e) => setClipSearch(e.target.value)}
              placeholder="筛选剪切板历史…"
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  if (clipSearch) setClipSearch("");
                  else void ipc.windowHideQuick();
                }
                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setClipIndex((i) =>
                    Math.min(i + 1, Math.max(clipItems.length - 1, 0)),
                  );
                }
                if (e.key === "ArrowUp") {
                  e.preventDefault();
                  setClipIndex((i) => Math.max(i - 1, 0));
                }
                if (e.key === "Enter" && clipItems[clipIndex]) {
                  e.preventDefault();
                  void reuseClip(clipItems[clipIndex].id);
                }
              }}
            />
            <div className="min-h-0 flex-1 overflow-auto rounded-[var(--radius-panel)] border border-border">
              {clipQuery.isLoading ? (
                <div className="p-4 text-[12px] text-muted">加载中…</div>
              ) : clipItems.length === 0 ? (
                <div className="p-4 text-center text-[12px] text-muted">
                  暂无记录。复制文本后会出现在这里。
                </div>
              ) : (
                <ul>
                  {clipItems.map((item, index) => (
                    <li key={item.id}>
                      <button
                        type="button"
                        className={cn(
                          "flex w-full flex-col gap-0.5 border-b border-border px-3 py-2 text-left hover:bg-row-hover",
                          index === clipIndex && "bg-row-active",
                        )}
                        onClick={() => void reuseClip(item.id)}
                        onMouseEnter={() => setClipIndex(index)}
                      >
                        <div className="truncate text-[13px] font-medium">
                          {item.content.replace(/\s+/g, " ").trim().slice(0, 100)}
                        </div>
                        <div className="text-[11px] text-muted">
                          {item.favorite ? "★ " : ""}
                          {item.createdAt}
                          {item.useCount > 0 ? ` · ${item.useCount} 次` : ""}
                        </div>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div className="flex items-center justify-between text-[11px] text-muted">
              <span>Enter 再次复制 · Esc 关闭</span>
              <Button
                size="sm"
                variant="ghost"
                onClick={async () => {
                  await ipc.windowShowMain();
                  await emit("main://navigate", "/clipboard");
                  await ipc.windowHideQuick();
                }}
              >
                打开主窗
              </Button>
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}
