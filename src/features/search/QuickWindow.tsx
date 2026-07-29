import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
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
  const [value, setValue] = useState("");
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
  }, [mode]);

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
      </div>
      <div className="flex flex-1 flex-col gap-3 p-3">
        <Input
          ref={inputRef}
          value={value}
          onChange={(event) => setValue(event.target.value)}
          placeholder={
            mode === "capture"
              ? "快速记录任务（阶段 1 接入）…"
              : mode === "search"
                ? "搜索任务、提醒、记忆…"
                : "搜索剪切板历史…"
          }
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              setValue("");
            }
          }}
        />
        <div
          className={cn(
            "flex flex-1 items-center justify-center rounded-[var(--radius-panel)] border border-dashed border-border text-[12px] text-muted",
          )}
        >
          {mode === "capture" && "全局快捷键唤起 · 默认创建任务"}
          {mode === "search" && "统一搜索将在阶段 3 接入"}
          {mode === "clip" && "剪切板浮层将在阶段 4 接入"}
        </div>
      </div>
    </div>
  );
}
