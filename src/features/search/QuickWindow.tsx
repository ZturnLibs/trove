import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import { ipc, type TaskPriority } from "@/ipc/client";
import { useUiStore, type QuickMode } from "@/stores/ui";
import { cn } from "@/lib/cn";

const modes: { id: QuickMode; label: string }[] = [
  { id: "capture", label: "记录" },
  { id: "search", label: "搜索" },
  { id: "clip", label: "剪切板" },
];

export function QuickWindow() {
  const mode = useUiStore((s) => s.quickMode);
  const setQuickMode = useUiStore((s) => s.setQuickMode);
  const [title, setTitle] = useState("");
  const [dueDate, setDueDate] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("none");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
  }, [mode]);

  const submit = async () => {
    if (mode !== "capture") return;
    const value = title.trim();
    if (!value) return;
    setSaving(true);
    setError(null);
    try {
      await ipc.taskCreate({
        title: value,
        dueDate: dueDate || null,
        priority: priority === "none" ? undefined : priority,
      });
      setTitle("");
      setDueDate("");
      setPriority("none");
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
            <div className="flex gap-2 text-[12px] text-muted">
              <span className="rounded-[var(--radius-control)] bg-row-active px-2 py-0.5 text-foreground">
                任务
              </span>
              <span>提醒 / 记忆 · 后续阶段</span>
            </div>
            <Input
              ref={inputRef}
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="快速记录任务…"
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  if (title) setTitle("");
                  else void ipc.windowHideQuick();
                }
                if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                  event.preventDefault();
                  void submit();
                }
              }}
            />
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
            {error ? <p className="text-[12px] text-danger">{error}</p> : null}
            <div className="mt-auto flex items-center justify-between text-[11px] text-muted">
              <span>⌘/Ctrl + Enter 创建 · Esc 取消</span>
              <Button size="sm" disabled={saving || !title.trim()} onClick={() => void submit()}>
                创建任务
              </Button>
            </div>
          </>
        ) : (
          <div
            className={cn(
              "flex flex-1 items-center justify-center rounded-[var(--radius-panel)] border border-dashed border-border text-[12px] text-muted",
            )}
          >
            {mode === "search" && "统一搜索将在阶段 3 接入"}
            {mode === "clip" && "剪切板浮层将在阶段 4 接入"}
          </div>
        )}
      </div>
    </div>
  );
}
