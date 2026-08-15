import { useCallback, useEffect, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Check } from "lucide-react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Input } from "@/design-system/primitives/Input";
import { ipc, type Task } from "@/ipc/client";
import { cn } from "@/lib/cn";

const priorityLabel: Record<Task["priority"], string> = {
  none: "",
  low: "低",
  medium: "中",
  high: "高",
};

function toDateString(d: Date): string {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function addDays(dateStr: string, days: number): string {
  const [y, m, d] = dateStr.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  dt.setDate(dt.getDate() + days);
  return toDateString(dt);
}

export type TaskRowProps = {
  task: Task;
  selected?: boolean;
  overdue?: boolean;
  onSelect: () => void;
  onToggleComplete: () => void;
  /** If provided, double-clicking the title edits it in place (optimistic). */
  onRename?: (task: Task, title: string) => void | Promise<void>;
  /**
   * Overrides the default due-date save (`ipc.taskUpdate` + `["tasks"]`
   * invalidation). Receives the row task and the next dueDate/dueTime.
   */
  onUpdateDue?: (
    task: Task,
    dueDate: string | null,
    dueTime: string | null,
  ) => void;
  /** The row is being dragged; dim it while dragging. */
  isDragging?: boolean;
};

/**
 * Default due-date persistence for TaskRow: full `ipc.taskUpdate` (every other
 * field preserved) followed by a `["tasks"]` invalidate so all lists refresh.
 * Callers can pass `onUpdateDue` to TaskRow to take over persistence.
 */
function useDueUpdate(
  task: Task,
  onUpdateDue?: TaskRowProps["onUpdateDue"],
) {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: (input: { dueDate: string | null; dueTime: string | null }) =>
      ipc.taskUpdate({
        id: task.id,
        title: task.title,
        notes: task.notes,
        priority: task.priority,
        listId: task.listId,
        dueDate: input.dueDate,
        dueTime: input.dueTime,
        tagNames: task.tagNames,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
  });
  return useCallback(
    (dueDate: string | null, dueTime: string | null) => {
      if (onUpdateDue) {
        onUpdateDue(task, dueDate, dueTime);
        return;
      }
      mutation.mutate({ dueDate, dueTime });
    },
    [task, onUpdateDue, mutation],
  );
}

export function TaskRow({
  task,
  selected,
  overdue,
  onSelect,
  onToggleComplete,
  onRename,
  onUpdateDue,
  isDragging,
}: TaskRowProps) {
  const done = task.status === "completed";
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(task.title);
  const inputRef = useRef<HTMLInputElement>(null);
  const [dueMenu, setDueMenu] = useState<{ x: number; y: number } | null>(null);
  const [customDate, setCustomDate] = useState("");
  const [customTime, setCustomTime] = useState("");
  const applyDue = useDueUpdate(task, onUpdateDue);

  useEffect(() => {
    if (editing) {
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    }
  }, [editing]);

  // Reset the custom inputs to the task's current values whenever the menu opens.
  useEffect(() => {
    if (dueMenu) {
      setCustomDate(task.dueDate ?? "");
      setCustomTime(task.dueTime ?? "");
    }
  }, [dueMenu, task.dueDate, task.dueTime]);

  // Close on outside click, scroll, or Esc (matches the listMenu pattern).
  useEffect(() => {
    if (!dueMenu) return;
    const close = () => setDueMenu(null);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [dueMenu]);

  // A fixed menu inside the sortable wrapper's transformed element would be
  // mispositioned relative to the viewport, so close it when a drag starts.
  useEffect(() => {
    if (isDragging) setDueMenu(null);
  }, [isDragging]);

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

  const openDueMenu = (event: { clientX: number; clientY: number }) => {
    setDueMenu({ x: event.clientX, y: event.clientY });
  };

  const saveDue = (dueDate: string | null, dueTime: string | null) => {
    setDueMenu(null);
    if (dueDate === task.dueDate && dueTime === task.dueTime) return;
    applyDue(dueDate, dueTime);
  };

  const today = toDateString(new Date());
  const quickOptions = [
    { label: "今天", value: today },
    { label: "明天", value: addDays(today, 1) },
    { label: "后天", value: addDays(today, 2) },
    { label: "+3 天", value: addDays(today, 3) },
    { label: "+7 天", value: addDays(today, 7) },
  ];

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
        "group relative flex h-9 cursor-default items-center gap-2 border-b border-border px-3 text-[13px] hover:bg-row-hover",
        selected && "bg-row-active",
        isDragging && "opacity-50",
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
            onPointerDown={(event) => event.stopPropagation()}
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
        <button
          type="button"
          title="设置截止日期"
          onClick={(event) => {
            event.stopPropagation();
            openDueMenu(event);
          }}
          onKeyDown={(event) => event.stopPropagation()}
          className={cn(
            "shrink-0 cursor-pointer text-[11px] text-muted hover:text-foreground",
            overdue && !done && "text-danger",
          )}
        >
          {task.dueDate}
          {task.dueTime ? ` ${task.dueTime}` : ""}
        </button>
      ) : (
        <button
          type="button"
          title="设置截止日期"
          onClick={(event) => {
            event.stopPropagation();
            openDueMenu(event);
          }}
          onKeyDown={(event) => event.stopPropagation()}
          className="shrink-0 cursor-pointer text-[11px] text-muted opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 hover:text-foreground"
        >
          无日期
        </button>
      )}
      {dueMenu ? (
        <div
          className="fixed z-50 min-w-[10rem] rounded-[var(--radius-control)] border border-border bg-surface py-1 shadow-lg"
          style={{ left: dueMenu.x, top: dueMenu.y }}
          onClick={(event) => event.stopPropagation()}
          onKeyDown={(event) => event.stopPropagation()}
        >
          {quickOptions.map((option) => (
            <button
              key={option.value}
              type="button"
              className={cn(
                "block w-full px-3 py-1.5 text-left text-[12px] hover:bg-surface-raised",
                option.value === task.dueDate &&
                  "bg-surface-raised text-foreground",
              )}
              onClick={(event) => {
                event.stopPropagation();
                saveDue(option.value, task.dueTime);
              }}
            >
              {option.label}
            </button>
          ))}
          <div className="flex items-center gap-1 border-t border-border px-2 py-1.5">
            <Input
              type="date"
              value={customDate}
              onChange={(event) => setCustomDate(event.target.value)}
              className="h-7 min-w-0 flex-1 px-1.5 text-[12px]"
            />
            <Input
              type="time"
              value={customTime}
              onChange={(event) => setCustomTime(event.target.value)}
              className="h-7 w-[5.5rem] shrink-0 px-1.5 text-[12px]"
            />
            <button
              type="button"
              className="shrink-0 rounded px-2 py-1 text-[12px] hover:bg-surface-raised"
              onClick={(event) => {
                event.stopPropagation();
                saveDue(customDate || null, customTime || null);
              }}
            >
              保存
            </button>
          </div>
          <button
            type="button"
            className="block w-full px-3 py-1.5 text-left text-[12px] text-destructive hover:bg-surface-raised"
            onClick={(event) => {
              event.stopPropagation();
              saveDue(null, null);
            }}
          >
            清除日期
          </button>
        </div>
      ) : null}
    </div>
  );
}

/**
 * Sortable wrapper around TaskRow for drag-to-reorder lists.
 *
 * Uses PointerSensor with a distance constraint (see the DndContext in the
 * calling page) so plain clicks, double-click rename and keyboard Enter/Space
 * selection keep working; keyboard reordering stays on the up/down buttons.
 * The editing input stops pointer propagation so text selection inside the
 * rename field never starts a drag.
 */
export function SortableTaskRow(props: TaskRowProps) {
  const {
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: props.task.id });

  return (
    <div
      ref={setNodeRef}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
      {...listeners}
    >
      <TaskRow {...props} isDragging={isDragging} />
    </div>
  );
}
