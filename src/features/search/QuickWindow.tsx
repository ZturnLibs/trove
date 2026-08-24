import { useEffect, useMemo, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { useQuery } from "@tanstack/react-query";
import { RecurrencePicker } from "@/design-system/patterns/RecurrencePicker";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import {
  ipc,
  type SemanticHit,
  type SearchEntityType,
  type SearchHit,
  type TaskPriority,
  type RecurrenceRule,
} from "@/ipc/client";
import {
  buildFireAtFromParsed,
  formatParsedHint,
  mergeTagNames,
} from "@/lib/nl-capture";
import { useUiStore, type QuickMode } from "@/stores/ui";
import { cn } from "@/lib/cn";

const modes: { id: QuickMode; label: string }[] = [
  { id: "capture", label: "记录" },
  { id: "search", label: "搜索" },
  { id: "clip", label: "剪切板" },
];

type CaptureType = "task" | "reminder" | "memory" | "note";

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
  const [dueTime, setDueTime] = useState("");
  const [tagText, setTagText] = useState("");
  const [fireAt, setFireAt] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("none");
  const [recurrence, setRecurrence] = useState<RecurrenceRule | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchText, setSearchText] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [clipSearch, setClipSearch] = useState("");
  const [clipIndex, setClipIndex] = useState(0);
  const [ambiguous, setAmbiguous] = useState<string[]>([]);
  const [parsedHint, setParsedHint] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const dueDateRef = useRef<HTMLInputElement>(null);
  const dueTimeRef = useRef<HTMLInputElement>(null);
  const tagsRef = useRef<HTMLInputElement>(null);
  const priorityRef = useRef<HTMLSelectElement>(null);

  const focusCaptureField = (index: number) => {
    const refs = [inputRef, dueDateRef, dueTimeRef, tagsRef, priorityRef];
    refs[index]?.current?.focus();
  };

  const handleCaptureFieldShortcut = (
    event: React.KeyboardEvent,
  ) => {
    if (!(event.metaKey || event.ctrlKey)) return;
    const num = Number.parseInt(event.key, 10);
    if (num >= 1 && num <= 5) {
      event.preventDefault();
      focusCaptureField(num - 1);
    }
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<string>("quick://set-mode", (event) => {
      const next = event.payload;
      if (next === "capture" || next === "search" || next === "clip") {
        setQuickMode(next);
      }
      // Every show re-emits this event — use it as the "just invoked" hook.
      if (next === "capture") {
        // Start each capture fresh: clear any text left from a dismissed-attempt.
        setTitle("");
        setBody("");
        setDueDate("");
        setDueTime("");
        setTagText("");
        setFireAt("");
        setPriority("none");
        setRecurrence(null);
        setAmbiguous([]);
        setParsedHint(null);
        setError(null);
      }
      // Focus the active mode's input after the (possible) mode switch renders.
      requestAnimationFrame(() => inputRef.current?.focus());
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [setQuickMode]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<string>("quick://set-search-query", (event) => {
      setSearchText(event.payload ?? "");
      requestAnimationFrame(() => inputRef.current?.focus());
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    inputRef.current?.focus();
    setError(null);
    setRecurrence(null);
  }, [mode, captureType]);

  useEffect(() => {
    if (
      mode !== "capture" ||
      (captureType !== "task" && captureType !== "reminder")
    ) {
      setAmbiguous([]);
      setParsedHint(null);
      return;
    }
    const value = title.trim();
    if (!value) {
      setAmbiguous([]);
      setParsedHint(null);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void ipc.nlParseCapture(value).then((parsed) => {
        if (cancelled) return;
        if (parsed.dueDate) setDueDate(parsed.dueDate);
        if (parsed.dueTime) {
          if (captureType === "task") setDueTime(parsed.dueTime);
        }
        setPriority(parsed.priority);
        if (parsed.recurrence) setRecurrence(parsed.recurrence);
        if (parsed.tagNames?.length) {
          setTagText(parsed.tagNames.join(", "));
        }
        if (captureType === "reminder" && (parsed.dueDate || parsed.dueTime)) {
          setFireAt(buildFireAtFromParsed(parsed.dueDate, parsed.dueTime));
        }
        setAmbiguous(parsed.ambiguousFields);
        setParsedHint(formatParsedHint(parsed));
      });
    }, 250);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [title, mode, captureType]);

  const templatesQuery = useQuery({
    queryKey: ["templates"],
    queryFn: () => ipc.templateList(),
    enabled: mode === "search",
  });

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

  const snippetsQuery = useQuery({
    queryKey: ["memories", { quickInsertOnly: true }],
    queryFn: () => ipc.memoryQuery({ quickInsertOnly: true }),
    enabled: mode === "capture",
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

  const clipItems = clipQuery.data?.items ?? [];

  // 快速记录模式下：输入恰好等于某 quickInsert 记忆的触发词时，提供「展开」入口。
  const snippetHit = useMemo(() => {
    // 随手记单次回车即提交，不提供片段展开入口。
    if (mode !== "capture" || captureType === "note") return null;
    const q = title.trim().toLowerCase();
    if (!q) return null;
    return (
      (snippetsQuery.data?.items ?? []).find(
        (m) => m.triggerWord && m.triggerWord.trim().toLowerCase() === q,
      ) ?? null
    );
  }, [mode, title, captureType, snippetsQuery.data]);

  const expandSnippet = () => {
    if (!snippetHit) return;
    // 仅替换标题输入，不改 captureType/dueDate/priority/daily 等既有表单状态。
    setTitle(snippetHit.body || snippetHit.title);
  };

  type PaletteItem =
    | { kind: "hit"; hit: SearchHit }
    | { kind: "command"; id: string; label: string; run: () => Promise<void> };

  const paletteItems = useMemo(() => {
    const commandItems: Extract<PaletteItem, { kind: "command" }>[] = [
      {
        kind: "command",
        id: "new-task",
        label: "新建任务",
        run: async () => {
          setQuickMode("capture");
          setCaptureType("task");
        },
      },
      {
        kind: "command",
        id: "open-today",
        label: "打开今日",
        run: async () => {
          await ipc.windowShowMain();
          await emit("main://navigate", "/today");
          await ipc.windowHideQuick();
        },
      },
      {
        kind: "command",
        id: "open-settings",
        label: "打开设置",
        run: async () => {
          await ipc.windowShowMain();
          await emit("main://navigate", "/settings");
          await ipc.windowHideQuick();
        },
      },
      {
        kind: "command",
        id: "open-clipboard",
        label: "打开剪切板浮层",
        run: async () => setQuickMode("clip"),
      },
      {
        kind: "command",
        id: "toggle-clipboard",
        label: "暂停/恢复剪切板采集",
        run: async () => {
          const settings = await ipc.settingsGet();
          await ipc.clipboardSetCaptureEnabled(!settings.clipboardCaptureEnabled);
          await ipc.windowHideQuick();
        },
      },
    ];

    for (const tpl of templatesQuery.data ?? []) {
      commandItems.push({
        kind: "command",
        id: `tpl-${tpl.id}`,
        label: `模板：${tpl.name}`,
        run: async () => {
          await ipc.templateApply(tpl.id);
          await ipc.windowHideQuick();
        },
      });
    }

    const q = searchText.trim().toLowerCase();
    const matchedCommands = q
      ? commandItems.filter((c) => c.label.toLowerCase().includes(q))
      : commandItems;
    const hits: PaletteItem[] = flatResults.map((hit) => ({
      kind: "hit" as const,
      hit,
    }));
    return [...matchedCommands, ...hits];
  }, [flatResults, searchText, templatesQuery.data, setQuickMode]);

  const commandPaletteItems = useMemo(
    () => paletteItems.filter((item) => item.kind === "command"),
    [paletteItems],
  );
  const hitPaletteItems = useMemo(
    () => paletteItems.filter((item) => item.kind === "hit"),
    [paletteItems],
  );
  const semanticHits: SemanticHit[] = useMemo(
    () =>
      (searchQuery.data?.semantic ?? []).filter(
        (s) =>
          !flatResults.some(
            (k) => k.entityType === s.entityType && k.entityId === s.entityId,
          ),
      ),
    [searchQuery.data, flatResults],
  );

  const renderPaletteRow = (item: PaletteItem, index: number) => (
    <li
      key={
        item.kind === "command"
          ? item.id
          : `${item.hit.entityType}-${item.hit.entityId}`
      }
    >
      <button
        type="button"
        className={cn(
          "flex w-full flex-col gap-0.5 border-b border-border px-3 py-2 text-left hover:bg-row-hover",
          index === selectedIndex && "bg-row-active",
        )}
        onClick={() => void runPaletteItem(item)}
        onMouseEnter={() => setSelectedIndex(index)}
      >
        <div className="flex items-center gap-2 text-[13px]">
          <span
            className={cn(
              "rounded px-1.5 text-[10px]",
              item.kind === "command"
                ? "bg-accent/15 text-accent"
                : "bg-row-hover text-muted",
            )}
          >
            {item.kind === "command" ? "命令" : typeLabel[item.hit.entityType]}
          </span>
          <span className="truncate font-medium">
            {item.kind === "command" ? item.label : item.hit.title}
          </span>
        </div>
        {item.kind === "hit" && item.hit.snippet ? (
          <div className="truncate text-[11px] text-muted">
            {item.hit.snippet.includes("[来自图片识别]")
              ? "来自图片识别 · "
              : ""}
            {item.hit.snippet.replace(/^\[来自图片识别\]\s*/u, "")}
          </div>
        ) : null}
      </button>
    </li>
  );

  useEffect(() => {
    setSelectedIndex(0);
  }, [searchText, paletteItems.length]);

  const openHit = async (hit: SearchHit) => {
    if (hit.entityType === "clipboard") {
      await ipc.clipboardCopy(hit.entityId);
      await ipc.windowHideQuick();
      return;
    }
    if (hit.entityType === "memory") {
      // Prefer copy for quick-insert style reuse.
      const memories = await ipc.memoryQuery({ quickInsertOnly: true });
      const snippet = memories.items.find((m) => m.id === hit.entityId);
      if (snippet) {
        await navigator.clipboard.writeText(
          snippet.body || snippet.title,
        );
        await ipc.windowHideQuick();
        return;
      }
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

  const openSemanticHit = async (hit: SemanticHit) => {
    await openHit({
      entityType: hit.entityType as SearchHit["entityType"],
      entityId: hit.entityId,
      title: hit.title,
      snippet: "",
      updatedAt: "",
    });
  };

  const runPaletteItem = async (item: PaletteItem) => {
    if (item.kind === "command") {
      try {
        await item.run();
      } catch (err) {
        setError(err instanceof Error ? err.message : "命令执行失败");
      }
      return;
    }
    await openHit(item.hit);
  };

  const completeSelectedTask = async () => {
    const item = paletteItems[selectedIndex];
    if (!item || item.kind !== "hit" || item.hit.entityType !== "task") return;
    await ipc.taskComplete(item.hit.entityId);
    await ipc.windowHideQuick();
  };

  const postponeSelectedTask = async () => {
    const item = paletteItems[selectedIndex];
    if (!item || item.kind !== "hit" || item.hit.entityType !== "task") return;
    await ipc.taskPostpone(item.hit.entityId, 1);
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
        const parsed = await ipc.nlParseCapture(value);
        const finalTitle = parsed.title.trim() || value;
        const finalDue = dueDate || parsed.dueDate || null;
        const finalDueTime = dueTime || parsed.dueTime || null;
        const finalPriority =
          priority !== "none"
            ? priority
            : parsed.priority !== "none"
              ? parsed.priority
              : undefined;
        const finalRecurrence = recurrence ?? parsed.recurrence ?? null;
        const finalTags = mergeTagNames(parsed.tagNames, tagText);
        if (finalRecurrence) {
          await ipc.taskCreateRecurring(
            {
              title: finalTitle,
              dueDate: finalDue,
              dueTime: finalDueTime,
              priority: finalPriority,
              tagNames: finalTags.length ? finalTags : undefined,
            },
            finalRecurrence,
          );
        } else {
          await ipc.taskCreate({
            title: finalTitle,
            dueDate: finalDue,
            dueTime: finalDueTime,
            priority: finalPriority,
            tagNames: finalTags.length ? finalTags : undefined,
          });
        }
      } else if (captureType === "reminder") {
        const parsed = await ipc.nlParseCapture(value);
        const finalTitle = parsed.title.trim() || value;
        const finalFireAt =
          fireAt || buildFireAtFromParsed(parsed.dueDate, parsed.dueTime);
        if (!finalFireAt) throw new Error("请选择提醒时间");
        const normalized =
          finalFireAt.length === 16 ? `${finalFireAt}:00` : finalFireAt;
        await ipc.reminderCreate({
          title: finalTitle,
          fireAt: normalized.replace(" ", "T"),
          timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          recurrence: recurrence ?? parsed.recurrence ?? null,
        });
      } else if (captureType === "memory") {
        await ipc.memoryCreate({
          title: value,
          body: body || undefined,
        });
      } else {
        await ipc.smokeNoteCreate(value);
      }
      setTitle("");
      setBody("");
      setDueDate("");
      setDueTime("");
      setTagText("");
      setFireAt("");
      setPriority("none");
      setRecurrence(null);
      setAmbiguous([]);
      setParsedHint(null);
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
              {(["task", "reminder", "memory", "note"] as const).map((type) => (
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
                  {type === "task"
                    ? "任务"
                    : type === "reminder"
                      ? "提醒"
                      : type === "memory"
                        ? "记忆"
                        : "随手记"}
                </button>
              ))}
            </div>
            <Input
              ref={inputRef}
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder={
                captureType === "task"
                  ? "如：明天下午三点回复客户…"
                  : captureType === "reminder"
                    ? "如：明天下午三点 Standup…"
                    : captureType === "memory"
                      ? "快速记录记忆标题…"
                      : "随手记…"
              }
              onKeyDown={(event) => {
                handleCaptureFieldShortcut(event);
                if (event.key === "Tab" && !event.shiftKey && captureType === "task") {
                  event.preventDefault();
                  focusCaptureField(1);
                }
                if (event.key === "Escape") {
                  if (title || body) {
                    setTitle("");
                    setBody("");
                  } else void ipc.windowHideQuick();
                }
                if (
                  (event.metaKey || event.ctrlKey) &&
                  event.key === "Enter" &&
                  captureType !== "note"
                ) {
                  event.preventDefault();
                  void submit();
                }
                if (event.key === "Enter" && captureType === "note") {
                  // 随手记：单次回车即提交（⌘/Ctrl+Enter 也走这里，避免重复提交）。
                  event.preventDefault();
                  void submit();
                }
                if (event.key === "Enter" && snippetHit) {
                  event.preventDefault();
                  expandSnippet();
                }
              }}
            />
            {snippetHit ? (
              <div className="overflow-hidden rounded-[var(--radius-panel)] border border-border">
                <button
                  type="button"
                  className="flex w-full items-center gap-2 border-b border-border px-3 py-2 text-left hover:bg-row-hover"
                  onClick={expandSnippet}
                >
                  <span className="rounded bg-row-hover px-1.5 text-[10px] text-muted">
                    片段
                  </span>
                  <span className="truncate text-[13px] font-medium">
                    ↩ 展开「{snippetHit.title}」
                  </span>
                </button>
              </div>
            ) : null}
            {captureType === "task" ? (
              <div className="grid grid-cols-2 gap-2">
                <label className="space-y-1 text-[11px] text-muted">
                  截止日期 <span className="text-muted/70">⌘2</span>
                  <Input
                    ref={dueDateRef}
                    type="date"
                    value={dueDate}
                    onChange={(e) => setDueDate(e.target.value)}
                    onKeyDown={handleCaptureFieldShortcut}
                    className={
                      ambiguous.includes("dueDate")
                        ? "ring-2 ring-amber-400/60"
                        : undefined
                    }
                  />
                </label>
                <label className="space-y-1 text-[11px] text-muted">
                  时间 <span className="text-muted/70">⌘3</span>
                  <Input
                    ref={dueTimeRef}
                    type="time"
                    value={dueTime}
                    onChange={(e) => setDueTime(e.target.value)}
                    onKeyDown={handleCaptureFieldShortcut}
                    className={
                      ambiguous.includes("dueTime")
                        ? "ring-2 ring-amber-400/60"
                        : undefined
                    }
                  />
                </label>
                <label className="col-span-2 space-y-1 text-[11px] text-muted">
                  标签 <span className="text-muted/70">⌘4 · #工作</span>
                  <Input
                    ref={tagsRef}
                    value={tagText}
                    onChange={(e) => setTagText(e.target.value)}
                    onKeyDown={handleCaptureFieldShortcut}
                    placeholder="逗号分隔，或输入 #标签"
                  />
                </label>
                <label className="col-span-2 space-y-1 text-[11px] text-muted">
                  优先级 <span className="text-muted/70">⌘5 · p1</span>
                  <select
                    ref={priorityRef}
                    className="h-8 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[13px] text-foreground"
                    value={priority}
                    onChange={(e) => setPriority(e.target.value as TaskPriority)}
                    onKeyDown={handleCaptureFieldShortcut}
                  >
                    <option value="none">无</option>
                    <option value="low">低</option>
                    <option value="medium">中</option>
                    <option value="high">高</option>
                  </select>
                </label>
              </div>
            ) : null}
            {(captureType === "task" || captureType === "reminder") && parsedHint ? (
              <p className="text-[11px] text-muted">{parsedHint}</p>
            ) : null}
            {captureType === "task" || captureType === "reminder" ? (
              <RecurrencePicker
                value={recurrence}
                onChange={setRecurrence}
                compact
              />
            ) : null}
            {captureType === "reminder" ? (
              <div className="space-y-2">
                <label className="space-y-1 text-[11px] text-muted">
                  提醒时间
                  <Input
                    type="datetime-local"
                    value={fireAt}
                    onChange={(e) => setFireAt(e.target.value)}
                    className={
                      ambiguous.includes("dueDate") || ambiguous.includes("dueTime")
                        ? "ring-2 ring-amber-400/60"
                        : undefined
                    }
                  />
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
              <span>
                {captureType === "task"
                  ? "⌘1–5 字段 · ⌘Enter 创建"
                  : "⌘/Ctrl + Enter 创建"}
                {" · Esc 取消"}
              </span>
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
                    : captureType === "memory"
                      ? "创建记忆"
                      : "创建随手记"}
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
              placeholder="搜索内容或输入命令…"
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  if (searchText) setSearchText("");
                  else void ipc.windowHideQuick();
                }
                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setSelectedIndex((i) =>
                    Math.min(i + 1, Math.max(paletteItems.length - 1, 0)),
                  );
                }
                if (e.key === "ArrowUp") {
                  e.preventDefault();
                  setSelectedIndex((i) => Math.max(i - 1, 0));
                }
                if (e.key === "Enter" && paletteItems[selectedIndex]) {
                  e.preventDefault();
                  if ((e.metaKey || e.ctrlKey) && e.shiftKey) {
                    void postponeSelectedTask();
                  } else if (e.metaKey || e.ctrlKey) {
                    void completeSelectedTask();
                  } else {
                    void runPaletteItem(paletteItems[selectedIndex]);
                  }
                }
              }}
            />
            <div className="min-h-0 flex-1 overflow-auto rounded-[var(--radius-panel)] border border-border">
              {searchText.trim() && searchQuery.isLoading ? (
                <div className="p-4 text-[12px] text-muted">搜索中…</div>
              ) : paletteItems.length === 0 ? (
                <div className="p-4 text-center text-[12px] text-muted">无结果</div>
              ) : (
                <ul>
                  {commandPaletteItems.length > 0 ? (
                    <>
                      <li className="sticky top-0 z-[1] bg-surface px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted">
                        命令
                      </li>
                      {commandPaletteItems.map((item) =>
                        renderPaletteRow(item, paletteItems.indexOf(item)),
                      )}
                    </>
                  ) : null}
                  {hitPaletteItems.length > 0 ? (
                    <>
                      <li
                        className={cn(
                          "sticky top-0 z-[1] bg-surface px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted",
                          commandPaletteItems.length > 0 &&
                            "border-t border-border",
                        )}
                      >
                        内容
                      </li>
                      {hitPaletteItems.map((item) =>
                        renderPaletteRow(item, paletteItems.indexOf(item)),
                      )}
                    </>
                  ) : null}
                </ul>
              )}
              {semanticHits.length > 0 ? (
                <div className="border-t border-border">
                  <div className="bg-surface px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted">
                    语义匹配
                  </div>
                  <ul>
                    {semanticHits.map((hit) => (
                      <li key={`${hit.entityType}-${hit.entityId}`}>
                        <button
                          type="button"
                          className="flex w-full items-center gap-2 border-b border-border px-3 py-2 text-left text-[13px] hover:bg-row-hover"
                          onClick={() => void openSemanticHit(hit)}
                        >
                          <span className="rounded border border-border px-1 text-[10px] text-muted">
                            语义
                          </span>
                          <span className="min-w-0 flex-1 truncate">{hit.title}</span>
                          <span className="text-[10px] text-muted">
                            {(hit.score * 100).toFixed(0)}%
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
            </div>
            <div className="text-[11px] text-muted">
              Enter 执行 · ⌘/Ctrl+Enter 完成任务 · ⌘/Ctrl+Shift+Enter 延期
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
                  暂无记录。复制文本或图片后会出现在这里。
                </div>
              ) : (
                <ul>
                  {clipItems.map((item, index) => (
                    <li key={item.id}>
                      <button
                        type="button"
                        className={cn(
                          "flex w-full items-center gap-2 border-b border-border px-3 py-2 text-left hover:bg-row-hover",
                          index === clipIndex && "bg-row-active",
                        )}
                        onClick={() => void reuseClip(item.id)}
                        onMouseEnter={() => setClipIndex(index)}
                      >
                        {item.kind === "image" && item.thumbBase64 ? (
                          <img
                            src={`data:image/png;base64,${item.thumbBase64}`}
                            alt=""
                            className="size-9 shrink-0 rounded border border-border object-cover"
                          />
                        ) : null}
                        <div className="min-w-0 flex-1">
                          <div className="truncate text-[13px] font-medium">
                            {item.kind === "image"
                              ? item.content.replace(/\s+/g, " ").trim().slice(0, 100) ||
                                "图片"
                              : item.content.replace(/\s+/g, " ").trim().slice(0, 100)}
                          </div>
                          <div className="text-[11px] text-muted">
                            {item.favorite ? "★ " : ""}
                            {item.kind === "image" ? "图片 · " : ""}
                            {item.createdAt}
                            {item.useCount > 0 ? ` · ${item.useCount} 次` : ""}
                          </div>
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
