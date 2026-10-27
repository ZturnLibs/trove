import { useEffect, useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useQuery } from "@tanstack/react-query";
import { TaskWorkflowContextMenu } from "@/design-system/patterns/TaskRow";
import { ipc, type Task } from "@/ipc/client";
import { cn } from "@/lib/cn";
import { localTodayString } from "@/lib/defer";

const priorityDot: Record<Task["priority"], string | null> = {
  none: null,
  low: "bg-muted",
  medium: "bg-accent",
  high: "bg-danger",
};

const priorityLabel: Record<Task["priority"], string> = {
  none: "无优先级",
  low: "低优先级",
  medium: "中优先级",
  high: "高优先级",
};

/** 子任务计数徽标（已完成/全部），无子任务时不渲染。 */
function SubtaskBadge({ taskId }: { taskId: string }) {
  const query = useQuery({
    queryKey: ["tasks", "board", "subtasks", taskId],
    queryFn: () => ipc.taskQuery({ parentId: taskId, limit: 200 }),
  });
  const total = query.data?.total ?? 0;
  if (total === 0) return null;
  const done = (query.data?.items ?? []).filter(
    (t) => t.status === "completed",
  ).length;
  return (
    <span
      className="shrink-0 rounded border border-border px-1 text-[10px] text-muted"
      title={`${done}/${total} 子任务已完成`}
    >
      {done}/{total}
    </span>
  );
}

/**
 * 看板卡片（精简于 TaskRow）：标题、清单名、到期日（逾期红）、优先级点、
 * 子任务计数徽标。点击打开详情面板；右键复用任务工作流菜单；
 * 拖拽由页内 DndContext 统一分发，这里只提供 draggable 手柄。
 */
export function BoardCard({
  task,
  selected,
  isDragging,
  onRename,
  onSelect,
  onSetDefer,
  onMarkWaiting,
  suppressClickUntilRef,
}: {
  task: Task;
  selected?: boolean;
  isDragging?: boolean;
  /** 双击标题行内改名（乐观）。 */
  onRename?: (task: Task, title: string) => void | Promise<void>;
  onSelect: (task: Task) => void;
  onSetDefer?: (task: Task, availableAt: string | null) => void | Promise<void>;
  onMarkWaiting?: (task: Task) => void | Promise<void>;
  suppressClickUntilRef: { current: number };
}) {
  // useSortable = 拖拽手柄 + 落点目标：同列拖动时兄弟卡片通过 transform 让位。
  // 自身处于拖拽态时不应用 transform（浮层负责跟随），保留半透明占位。
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    isDragging: dndDragging,
  } = useSortable({ id: task.id });
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(
    null,
  );
  const dragging = isDragging || dndDragging;

  // 菜单关闭时机与 TaskRow 一致：outside click / 滚动 / Esc。
  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
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
  }, [contextMenu]);

  return (
    <div
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      style={{
        transform:
          !dragging && transform ? CSS.Translate.toString(transform) : undefined,
        transition: "transform 180ms ease",
      }}
      role="button"
      tabIndex={0}
      onClick={() => {
        if (Date.now() < suppressClickUntilRef.current) return;
        onSelect(task);
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onSelect(task);
        }
      }}
      onContextMenu={(event) => {
        if (task.status === "completed" || (!onSetDefer && !onMarkWaiting)) {
          return;
        }
        event.preventDefault();
        setContextMenu({ x: event.clientX, y: event.clientY });
      }}
      className={cn(
        "group touch-none rounded-[var(--radius-control)] border border-border bg-surface p-2 text-left shadow-sm hover:border-accent/40 hover:bg-row-hover",
        dragging ? "cursor-grabbing opacity-40" : "cursor-grab",
        selected && "border-accent bg-row-active",
        task.status === "completed" && "opacity-70",
      )}
    >
      <BoardCardBody task={task} onRename={onRename} />
      {contextMenu ? (
        <TaskWorkflowContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          task={task}
          isWaiting={false}
          onSetDefer={onSetDefer}
          onMarkWaiting={onMarkWaiting}
          onClose={() => setContextMenu(null)}
        />
      ) : null}
    </div>
  );
}

/** 卡片内容。传 onRename 时标题支持双击行内编辑（与 TaskRow 同模式）。 */
export function BoardCardBody({
  task,
  onRename,
}: {
  task: Task;
  onRename?: (task: Task, title: string) => void | Promise<void>;
}) {
  const [editing, setEditing] = useState(false);
  const overdue =
    task.status !== "completed" &&
    task.dueDate !== null &&
    task.dueDate < localTodayString();
  const dot = priorityDot[task.priority];

  // 提交：trim 后与现值不同才落库；Esc 直接退出（值未变，blur 提交为 no-op）。
  const commit = (value: string) => {
    const trimmed = value.trim();
    if (trimmed && trimmed !== task.title) {
      void onRename?.(task, trimmed);
    }
    setEditing(false);
  };

  return (
    <div className="flex items-start gap-1.5">
      {dot ? (
        <span
          aria-hidden
          title={priorityLabel[task.priority]}
          className={cn("mt-1.5 h-2 w-2 shrink-0 rounded-full", dot)}
        />
      ) : null}
      <div className="min-w-0 flex-1">
        {editing && onRename ? (
          <input
            autoFocus
            defaultValue={task.title}
            onClick={(event) => event.stopPropagation()}
            onDoubleClick={(event) => event.stopPropagation()}
            onPointerDown={(event) => event.stopPropagation()}
            onBlur={(event) => commit(event.target.value)}
            onKeyDown={(event) => {
              // 按键留在输入框内（卡片层 Enter/Space 是选中、指针监听是拖拽）。
              event.stopPropagation();
              if (event.key === "Enter") commit(event.currentTarget.value);
              if (event.key === "Escape") setEditing(false);
            }}
            className="w-full rounded-[var(--radius-control)] border border-accent/60 bg-surface-raised px-1 py-0.5 text-[13px] outline-none"
          />
        ) : (
          <div
            className={cn(
              "truncate text-[13px]",
              task.status === "completed" && "text-muted line-through",
              overdue && "text-danger",
            )}
            onDoubleClick={(event) => {
              if (!onRename) return;
              event.stopPropagation();
              setEditing(true);
            }}
          >
            {task.title}
          </div>
        )}
        <div className="mt-1 flex items-center gap-1.5 text-[11px] text-muted">
          <span className="min-w-0 truncate">{task.listName}</span>
          {task.dueDate ? (
            <span className={cn("shrink-0", overdue && "text-danger")}>
              {task.dueDate}
              {task.dueTime ? ` ${task.dueTime}` : ""}
            </span>
          ) : null}
          {task.seriesId ? <span className="shrink-0">重复</span> : null}
          <SubtaskBadge taskId={task.id} />
        </div>
      </div>
    </div>
  );
}
