import { useEffect, useMemo, useRef, useState } from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  pointerWithin,
  useDroppable,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import type { DragEndEvent, DragStartEvent } from "@dnd-kit/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { BoardCardBody, BoardCard } from "@/features/board/BoardCard";
import {
  buildGroupByListId,
  defaultListIdForScope,
  sameListReorderIds,
  splitBoardColumns,
} from "@/features/board/splitBoardColumns";
import {
  buildTodayScopes,
  sameScope,
} from "@/features/tasks/listGroupTree";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { SplitTaskLayout } from "@/features/tasks/TaskLayout";
import {
  ipc,
  type ListGroupScope,
  type Task,
  type TaskBoardStage,
} from "@/ipc/client";
import { useRecentActions } from "@/stores/recent-actions";
import { cn } from "@/lib/cn";
import { localTodayString } from "@/lib/defer";

const BOARD_GROUP_FILTER_KEY = "board.groupFilter";

type BoardColumnId = "col:todo" | "col:doing" | "col:done";

type BoardColumnKey = "todo" | "doing" | "done";

type BoardCacheSnapshot = { todoSnapshot: unknown; doneSnapshot: unknown };

function parseStoredScope(raw: string | null): ListGroupScope | null {
  if (!raw) return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return null;
    if ("kind" in parsed && parsed.kind === "ungrouped") {
      return { kind: "ungrouped" };
    }
    if (
      "kind" in parsed &&
      parsed.kind === "group" &&
      "groupId" in parsed &&
      typeof parsed.groupId === "string"
    ) {
      return { kind: "group", groupId: parsed.groupId };
    }
  } catch {
    /* ignore */
  }
  return null;
}

function readBoardGroupFilter(): ListGroupScope | null {
  try {
    return parseStoredScope(localStorage.getItem(BOARD_GROUP_FILTER_KEY));
  } catch {
    /* ignore */
  }
  return null;
}

function persistBoardGroupFilter(scope: ListGroupScope | null) {
  try {
    if (scope) localStorage.setItem(BOARD_GROUP_FILTER_KEY, JSON.stringify(scope));
    else localStorage.removeItem(BOARD_GROUP_FILTER_KEY);
  } catch {
    /* ignore */
  }
}

/** URL `?group=` 深链取值：`ungrouped` / 分组 id；缺失时回退 localStorage 偏好。 */
function parseGroupParam(
  groupParam: string | null,
): ListGroupScope | null {
  if (groupParam === "ungrouped") return { kind: "ungrouped" };
  if (groupParam) return { kind: "group", groupId: groupParam };
  return readBoardGroupFilter();
}

function BoardColumnView({
  columnId,
  title,
  hint,
  count,
  footer,
  children,
}: {
  columnId: BoardColumnId;
  title: string;
  hint?: string;
  count: number;
  footer?: React.ReactNode;
  children: React.ReactNode;
}) {
  const { setNodeRef, isOver } = useDroppable({ id: columnId });
  return (
    <section
      ref={setNodeRef}
      className={cn(
        "flex min-w-[272px] w-[272px] flex-col rounded-[var(--radius-panel)] border border-border bg-surface",
        isOver && "border-accent",
      )}
    >
      <header className="flex shrink-0 items-center gap-2 px-3 py-2">
        <h2 className="text-[12px] font-medium">{title}</h2>
        <span className="rounded-full bg-row-active px-1.5 text-[11px] text-muted">
          {count}
        </span>
        {hint ? <span className="text-[10px] text-muted">{hint}</span> : null}
      </header>
      <div className="min-h-0 flex-1 space-y-2 overflow-y-auto px-2 pb-2">
        {children}
      </div>
      {footer ? (
        <div className="shrink-0 border-t border-border px-2 py-2">{footer}</div>
      ) : null}
    </section>
  );
}

/** 列内新建：折叠为「＋ 新建任务」，展开为行内标题输入（Enter 创建 / Esc 关闭）。 */
function BoardQuickAdd({
  open,
  busy,
  value,
  onOpen,
  onChange,
  onSubmit,
  onDismiss,
}: {
  open: boolean;
  busy: boolean;
  value: string;
  onOpen: () => void;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onDismiss: () => void;
}) {
  if (!open) {
    return (
      <button
        type="button"
        onClick={onOpen}
        className="w-full rounded-[var(--radius-control)] border border-dashed border-border px-2 py-1.5 text-left text-[11px] text-muted transition-colors hover:border-accent hover:text-foreground"
      >
        ＋ 新建任务
      </button>
    );
  }
  return (
    <input
      autoFocus
      value={value}
      disabled={busy}
      placeholder="标题后回车创建，Esc 关闭"
      onChange={(e) => onChange(e.target.value)}
      onBlur={() => {
        if (!value.trim()) onDismiss();
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter" && value.trim()) onSubmit();
        if (e.key === "Escape") onDismiss();
      }}
      className="w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 py-1.5 text-[12px] outline-none focus:border-accent"
    />
  );
}

export function BoardPage() {
  useDomainInvalidation();
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [focusTitleId, setFocusTitleId] = useState<string | null>(null);
  const [activeDragId, setActiveDragId] = useState<string | null>(null);
  const [composerColumn, setComposerColumn] = useState<"todo" | "doing" | null>(
    null,
  );
  const [composerTitle, setComposerTitle] = useState("");
  const [activeDragTask, setActiveDragTask] = useState<Task | null>(null);
  const suppressClickUntilRef = useRef(0);
  const today = localTodayString();

  // 分组过滤：URL `?group=` 单向同步（解析即状态）；页内切换走 replace，
  // URL 无参数时回退 localStorage 偏好（board.groupFilter，try/catch 持久化）。
  const groupParam = searchParams.get("group");
  const groupScope = useMemo(() => parseGroupParam(groupParam), [groupParam]);
  useEffect(() => {
    // 深链进入时把选择写入偏好，回到 /board（无参数）时仍可还原。
    if (groupParam) persistBoardGroupFilter(parseGroupParam(groupParam));
  }, [groupParam]);

  const applyGroupScope = (scope: ListGroupScope | null) => {
    persistBoardGroupFilter(scope);
    if (scope) {
      setSearchParams(
        { group: scope.kind === "ungrouped" ? "ungrouped" : scope.groupId },
        { replace: true },
      );
    } else {
      setSearchParams({}, { replace: true });
    }
  };

  const overviewQuery = useQuery({
    queryKey: ["task-list-overview"],
    queryFn: () => ipc.taskListOverview(),
    refetchInterval: 15_000,
  });

  const listGroupId =
    groupScope?.kind === "group" ? groupScope.groupId : undefined;

  const todoQueryKey = ["tasks", "board", "todo", groupScope];
  const doneQueryKey = ["tasks", "board", "done", groupScope, today];

  const todoQuery = useQuery({
    queryKey: todoQueryKey,
    queryFn: () => ipc.taskQuery({ status: "todo", listGroupId }),
    refetchInterval: 15_000,
  });
  const doneQuery = useQuery({
    queryKey: doneQueryKey,
    queryFn: () =>
      ipc.taskQuery({ status: "completed", completedSince: today, listGroupId }),
    refetchInterval: 15_000,
  });

  const groupByListId = useMemo(
    () =>
      overviewQuery.data
        ? buildGroupByListId(overviewQuery.data)
        : new Map<string, string | null>(),
    [overviewQuery.data],
  );

  const columns = useMemo(
    () =>
      splitBoardColumns(
        todoQuery.data?.items ?? [],
        doneQuery.data?.items ?? [],
        groupScope,
        groupByListId,
      ),
    [todoQuery.data, doneQuery.data, groupScope, groupByListId],
  );

  const allBoardTasks = useMemo(
    () => [...columns.todo, ...columns.doing, ...columns.done],
    [columns],
  );
  const selectedTask =
    allBoardTasks.find((task) => task.id === selectedId) ?? null;

  const scopes = useMemo(
    () => (overviewQuery.data ? buildTodayScopes(overviewQuery.data) : []),
    [overviewQuery.data],
  );

  // 失效面收窄到看板自身查询 + 侧栏徽标 overview。打全 ["tasks"] 前缀会连带
  // 每张卡片的 SubtaskBadge 查询（["tasks","board","subtasks",id]）——一次落卡
  // 触发 ~20 个 IPC 重查 + 全列重渲染，表现为落卡卡顿。
  const invalidateBoard = () => {
    void queryClient.invalidateQueries({ queryKey: ["tasks", "board"] });
    void queryClient.invalidateQueries({ queryKey: ["task-list-overview"] });
  };

  const patchBoardCaches = (taskId: string, patch: Partial<Task>) => {
    const patchIn = (old: { items: Task[] } | undefined) =>
      old
        ? {
            ...old,
            items: old.items.map((t) =>
              t.id === taskId ? { ...t, ...patch } : t,
            ),
          }
        : old;
    queryClient.setQueryData(todoQueryKey, patchIn);
    queryClient.setQueryData(doneQueryKey, patchIn);
  };

  // 乐观更新基建：cancel 进行中请求 → 快照两路缓存；失败时原样恢复。
  const snapshotBoardCaches = async (): Promise<BoardCacheSnapshot> => {
    await queryClient.cancelQueries({ queryKey: ["tasks", "board"] });
    return {
      todoSnapshot: queryClient.getQueryData(todoQueryKey),
      doneSnapshot: queryClient.getQueryData(doneQueryKey),
    };
  };

  const restoreBoardCaches = (context: BoardCacheSnapshot | undefined) => {
    if (!context) return;
    queryClient.setQueryData(todoQueryKey, context.todoSnapshot);
    queryClient.setQueryData(doneQueryKey, context.doneSnapshot);
  };

  const stageMutation = useMutation<
    Task,
    Error,
    { id: string; stage: TaskBoardStage | null },
    BoardCacheSnapshot
  >({
    mutationFn: ({ id, stage }) => ipc.taskSetBoardStage(id, stage),
    onMutate: async (vars) => {
      const snapshot = await snapshotBoardCaches();
      patchBoardCaches(vars.id, { boardStage: vars.stage });
      return snapshot;
    },
    onError: (_error, _vars, context) => {
      restoreBoardCaches(context);
    },
    onSettled: invalidateBoard,
  });

  const completeMutation = useMutation<Task, Error, Task, BoardCacheSnapshot>({
    mutationFn: (task) => ipc.taskComplete(task.id),
    onMutate: async (task) => {
      const snapshot = await snapshotBoardCaches();
      patchBoardCaches(task.id, {
        status: "completed",
        boardStage: null,
        completedAt: new Date().toISOString(),
      });
      return snapshot;
    },
    onError: (_error, _task, context) => {
      restoreBoardCaches(context);
    },
    onSuccess: (_data, task) => {
      invalidateBoard();
      useRecentActions.getState().push({
        label: "完成",
        undo: async () => {
          await ipc.taskUncomplete(task.id);
          invalidateBoard();
        },
      });
    },
  });

  const uncompleteMutation = useMutation<Task, Error, Task, BoardCacheSnapshot>({
    mutationFn: (task) => ipc.taskUncomplete(task.id),
    onMutate: async (task) => {
      const snapshot = await snapshotBoardCaches();
      patchBoardCaches(task.id, {
        status: "todo",
        boardStage: null,
        completedAt: null,
      });
      return snapshot;
    },
    onError: (_error, _task, context) => {
      restoreBoardCaches(context);
    },
    onSuccess: (_data, task) => {
      invalidateBoard();
      useRecentActions.getState().push({
        label: "取消完成",
        undo: async () => {
          await ipc.taskComplete(task.id);
          invalidateBoard();
        },
      });
    },
  });

  // 列内新建：标题即建，清单取当前作用域默认（分组 → 该组首个清单，否则收件箱）；
  // 进行中列创建后立刻置 doing。成功清空输入并保持 composer 开启，便于连续添加；
  // 失败保留已输标题供重试。
  const createMutation = useMutation<
    Task,
    Error,
    { title: string; column: "todo" | "doing" }
  >({
    mutationFn: async ({ title, column }) => {
      const created = await ipc.taskCreate({
        title,
        listId:
          overviewQuery.data
            ? (defaultListIdForScope(groupScope, overviewQuery.data) ??
              undefined)
            : undefined,
      });
      if (column === "doing") {
        return ipc.taskSetBoardStage(created.id, "doing");
      }
      return created;
    },
    onSuccess: () => {
      setComposerTitle("");
      invalidateBoard();
    },
  });

  // 同清单列内排序：乐观重排缓存数组（split 按输入顺序分流，数组序即列序），
  // 持久化交给 task_reorder——后端按清单分组重写 sort_order，其他清单不动。
  const reorderMutation = useMutation<
    void,
    Error,
    { activeId: string; overId: string; orderedIds: string[] },
    BoardCacheSnapshot
  >({
    mutationFn: ({ orderedIds }) => ipc.taskReorder(orderedIds),
    onMutate: async (vars) => {
      const snapshot = await snapshotBoardCaches();
      const reorderIn = (old: { items: Task[] } | undefined) => {
        if (!old) return old;
        const from = old.items.findIndex((t) => t.id === vars.activeId);
        const to = old.items.findIndex((t) => t.id === vars.overId);
        if (from === -1 || to === -1) return old;
        const items = arrayMove(old.items, from, to);
        const movedListId = items[to]?.listId;
        let seq = 0;
        for (const t of items) {
          if (t.listId === movedListId && vars.orderedIds.includes(t.id)) {
            t.sortOrder = seq++;
          }
        }
        return { ...old, items };
      };
      queryClient.setQueryData(todoQueryKey, reorderIn);
      queryClient.setQueryData(doneQueryKey, reorderIn);
      return snapshot;
    },
    onError: (_error, _vars, context) => {
      restoreBoardCaches(context);
    },
    onSettled: invalidateBoard,
  });

  const submitComposer = (column: "todo" | "doing") => {
    const title = composerTitle.trim();
    if (!title || createMutation.isPending) return;
    createMutation.mutate({ title, column });
  };

  const dismissComposer = () => {
    setComposerColumn(null);
    setComposerTitle("");
  };

  const applyDefer = async (task: Task, availableAt: string | null) => {
    const prev = task.availableAt;
    await ipc.taskSetDefer(task.id, availableAt);
    invalidateBoard();
    useRecentActions.getState().push({
      label: availableAt ? `推迟显示至 ${availableAt}` : "取消推迟显示",
      undo: async () => {
        await ipc.taskSetDefer(task.id, prev);
        invalidateBoard();
      },
    });
  };

  const applyMarkWaiting = (task: Task) => {
    // 与任务页一致：标记等待在详情面板里补充 waiting_for。
    setSelectedId(task.id);
    setFocusTitleId(task.id);
  };

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
  );

  const findTask = (id: string) =>
    allBoardTasks.find((task) => task.id === id) ?? null;

  const handleDragStart = (event: DragStartEvent) => {
    const dragged = findTask(String(event.active.id));
    setActiveDragId(String(event.active.id));
    setActiveDragTask(dragged);
  };

  const columnOf = (taskId: string): BoardColumnKey | null => {
    if (columns.todo.some((t) => t.id === taskId)) return "todo";
    if (columns.doing.some((t) => t.id === taskId)) return "doing";
    if (columns.done.some((t) => t.id === taskId)) return "done";
    return null;
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const dragged = findTask(String(event.active.id));
    setActiveDragId(null);
    setActiveDragTask(null);
    suppressClickUntilRef.current = Date.now() + 300;
    if (!dragged || !event.over) return;
    const overId = String(event.over.id);
    const source = columnOf(dragged.id);
    // over 可能是卡片（useSortable）或列 droppable（空白区/空列）。
    const target: BoardColumnKey | null =
      columnOf(overId) ??
      (overId === "col:todo" || overId === "col:doing" || overId === "col:done"
        ? (overId.slice(4) as BoardColumnKey)
        : null);
    if (!source || !target) return;

    // 同列：仅同清单段支持手动排序（sort_order 是清单内维度）；
    // 完成列按完成时间排序，不支持手动重排。
    if (target === source) {
      if (source === "done") return;
      if (overId === dragged.id || overId.startsWith("col:")) return;
      const orderedIds = sameListReorderIds(
        columns[source],
        dragged.id,
        overId,
      );
      if (orderedIds) {
        reorderMutation.mutate({
          activeId: dragged.id,
          overId,
          orderedIds,
        });
      }
      return;
    }

    if (target === "doing") {
      if (dragged.status === "todo" && dragged.boardStage !== "doing") {
        stageMutation.mutate({ id: dragged.id, stage: "doing" });
      }
      return;
    }
    if (target === "todo") {
      if (dragged.status === "completed") {
        uncompleteMutation.mutate(dragged);
      } else if (dragged.boardStage === "doing") {
        stageMutation.mutate({ id: dragged.id, stage: null });
      }
      return;
    }
    if (target === "done" && dragged.status === "todo") {
      completeMutation.mutate(dragged);
    }
  };

  const renderCards = (tasks: Task[]) => {
    if (tasks.length === 0) {
      return (
        <p className="px-2 py-4 text-center text-[11px] text-muted">
          拖拽任务到此列
        </p>
      );
    }
    return tasks.map((task) => (
      <BoardCard
        key={task.id}
        task={task}
        selected={task.id === selectedId}
        isDragging={task.id === activeDragId}
        onSelect={(selected) => {
          setSelectedId(selected.id);
          setFocusTitleId(null);
        }}
        onSetDefer={applyDefer}
        onMarkWaiting={applyMarkWaiting}
        suppressClickUntilRef={suppressClickUntilRef}
      />
    ));
  };

  const totalCount =
    columns.todo.length + columns.doing.length + columns.done.length;

  return (
    <SplitTaskLayout
      title="看板"
      description={`待办 / 进行中 / 今日完成 · 共 ${totalCount} 项`}
      list={
        <div className="flex h-full min-h-0 flex-col">
          <div className="flex flex-wrap items-center gap-1 border-b border-border px-3 py-2">
            {scopes.map((item) => (
              <button
                key={item.label}
                type="button"
                className={cn(
                  "rounded-full px-2.5 py-0.5 text-[11px]",
                  sameScope(groupScope, item.scope)
                    ? "bg-row-active text-foreground"
                    : "text-muted hover:bg-row-hover hover:text-foreground",
                )}
                onClick={() => applyGroupScope(item.scope)}
              >
                {item.label}
              </button>
            ))}
          </div>
          <DndContext
            sensors={sensors}
            collisionDetection={pointerWithin}
            onDragStart={handleDragStart}
            onDragEnd={handleDragEnd}
            onDragCancel={() => {
              setActiveDragId(null);
              setActiveDragTask(null);
            }}
          >
            <div className="flex min-h-0 flex-1 gap-3 overflow-x-auto p-3">
              <BoardColumnView
                columnId="col:todo"
                title="待办"
                count={columns.todo.length}
                footer={
                  <BoardQuickAdd
                    open={composerColumn === "todo"}
                    busy={createMutation.isPending}
                    value={composerTitle}
                    onOpen={() => setComposerColumn("todo")}
                    onChange={setComposerTitle}
                    onSubmit={() => submitComposer("todo")}
                    onDismiss={dismissComposer}
                  />
                }
              >
                <SortableContext
                  items={columns.todo.map((t) => t.id)}
                  strategy={verticalListSortingStrategy}
                >
                  {renderCards(columns.todo)}
                </SortableContext>
              </BoardColumnView>
              <BoardColumnView
                columnId="col:doing"
                title="进行中"
                count={columns.doing.length}
                footer={
                  <BoardQuickAdd
                    open={composerColumn === "doing"}
                    busy={createMutation.isPending}
                    value={composerTitle}
                    onOpen={() => setComposerColumn("doing")}
                    onChange={setComposerTitle}
                    onSubmit={() => submitComposer("doing")}
                    onDismiss={dismissComposer}
                  />
                }
              >
                <SortableContext
                  items={columns.doing.map((t) => t.id)}
                  strategy={verticalListSortingStrategy}
                >
                  {renderCards(columns.doing)}
                </SortableContext>
              </BoardColumnView>
              <BoardColumnView
                columnId="col:done"
                title="已完成"
                hint="今日"
                count={columns.done.length}
              >
                <SortableContext
                  items={columns.done.map((t) => t.id)}
                  strategy={verticalListSortingStrategy}
                >
                  {renderCards(columns.done)}
                </SortableContext>
              </BoardColumnView>
            </div>
            <DragOverlay
              dropAnimation={{
                duration: 180,
                easing: "cubic-bezier(0.18, 0.67, 0.6, 0.22)",
              }}
            >
              {activeDragTask ? (
                <div className="w-[240px] rotate-2 cursor-grabbing rounded-[var(--radius-control)] border border-accent/50 bg-surface p-2 shadow-xl ring-2 ring-accent/40">
                  <BoardCardBody task={activeDragTask} />
                </div>
              ) : null}
            </DragOverlay>
          </DndContext>
        </div>
      }
      detail={
        <TaskDetailPanel
          task={selectedTask}
          onDeleted={() => setSelectedId(null)}
          focusTitleId={focusTitleId}
        />
      }
    />
  );
}
