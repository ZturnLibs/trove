import { useEffect, useRef, useState } from "react";
import { Check } from "lucide-react";
import { Input } from "@/design-system/primitives/Input";
import type { Task } from "@/ipc/client";
import { cn } from "@/lib/cn";

const priorityLabel: Record<Task["priority"], string> = {
  none: "",
  low: "低",
  medium: "中",
  high: "高",
};

export function TaskRow({
  task,
  selected,
  overdue,
  onSelect,
  onToggleComplete,
  onRename,
}: {
  task: Task;
  selected?: boolean;
  overdue?: boolean;
  onSelect: () => void;
  onToggleComplete: () => void;
  /** If provided, double-clicking the title edits it in place (optimistic). */
  onRename?: (task: Task, title: string) => void | Promise<void>;
}) {
  const done = task.status === "completed";
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(task.title);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing) {
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    }
  }, [editing]);

  const startEdit = () => {
    setDraft(task.title);
    setEditing(true);
  };

  const commit = () => {
    const trimmed = draft.trim();
    if (trimmed && trimmed !== task.title) {
      void onRename?.(task, trimmed);
    }
    setEditing(false);
  };

  const cancel = () => {
    setDraft(task.title);
    setEditing(false);
  };

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onSelect();
        }
      }}
      className={cn(
        "group flex h-9 cursor-default items-center gap-2 border-b border-border px-3 text-[13px] hover:bg-row-hover",
        selected && "bg-row-active",
      )}
    >
      <button
        type="button"
        aria-label={done ? "取消完成" : "完成"}
        className={cn(
          "flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-border",
          done && "border-accent bg-accent text-accent-fg",
        )}
        onClick={(event) => {
          event.stopPropagation();
          onToggleComplete();
        }}
      >
        {done ? <Check className="h-3 w-3" /> : null}
      </button>
      <div className="min-w-0 flex-1">
        {editing ? (
          <Input
            ref={inputRef}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onClick={(event) => event.stopPropagation()}
            onDoubleClick={(event) => event.stopPropagation()}
            onKeyDown={(event) => {
              // Keep keystrokes inside the field (the row binds Enter/Space to select).
              event.stopPropagation();
              if (event.key === "Enter") {
                event.preventDefault();
                commit();
              } else if (event.key === "Escape") {
                event.preventDefault();
                cancel();
              }
            }}
            onBlur={commit}
            className="h-7 text-[13px]"
          />
        ) : (
          <div
            onDoubleClick={(event) => {
              if (!onRename) return;
              event.stopPropagation();
              startEdit();
            }}
            className={cn(
              "truncate",
              done && "text-muted line-through",
              overdue && !done && "text-danger",
            )}
          >
            {task.title}
          </div>
        )}
      </div>
      {task.priority !== "none" ? (
        <span className="shrink-0 text-[11px] text-muted">
          {priorityLabel[task.priority]}
        </span>
      ) : null}
      {task.seriesId ? (
        <span className="shrink-0 text-[11px] text-muted">重复</span>
      ) : null}
      {task.dueDate ? (
        <span
          className={cn(
            "shrink-0 text-[11px] text-muted",
            overdue && !done && "text-danger",
          )}
        >
          {task.dueDate}
          {task.dueTime ? ` ${task.dueTime}` : ""}
        </span>
      ) : null}
    </div>
  );
}
