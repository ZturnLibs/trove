import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent, DragStartEvent } from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { SortableTaskRow, TaskRow } from "@/design-system/patterns/TaskRow";
import { EmptyState } from "@/components/PageScaffold";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";
import {
  ipc,
  type DeleteListResult,
  type ListDeleteDisposition,
  type SavedView,
  type SmartListKind,
  type Task,
  type TaskList,
  type TaskPriority,
  type TaskStatus,
} from "@/ipc/client";
import {
  NewTaskButton,
  SplitTaskLayout,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { useTaskRename } from "@/features/tasks/useTaskRename";
import { useFocusSession } from "@/stores/focus-session";
import { useRecentActions } from "@/stores/recent-actions";
import {
  PagedListFooter,
  usePagedQuery,
} from "@/features/shared/usePagedQuery";

const smartLists: { id: SmartListKind | "none"; label: string }[] = [
  { id: "none", label: "清单视图" },
  { id: "tomorrow", label: "明天" },
  { id: "next7Days", label: "未来七天" },
  { id: "overdue", label: "逾期" },
  { id: "highPriority", label: "高优先级" },
  { id: "noDue", label: "无日期" },
  { id: "recentCompleted", label: "最近完成" },
  { id: "deferred", label: "已推迟" },
  { id: "waitingFollowUp", label: "等待跟进" },
];

export function TasksPage() {
  useDomainInvalidation();
  const rename = useTaskRename();
  const queryClient = useQueryClient();
  const startFocus = useFocusSession((s) => s.start);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [listId, setListId] = useState<string>("all");
  const [status, setStatus] = useState<TaskStatus | "active">("active");
  const [priority, setPriority] = useState<TaskPriority | "all">("all");
  const [tagId, setTagId] = useState<string | null>(null);
  const [smart, setSmart] = useState<SmartListKind | "none">("none");
  const [showDeferred, setShowDeferred] = useState(false);
  const [showWaiting, setShowWaiting] = useState(false);
  const [search, setSearch] = useState("");
  const [newListName, setNewListName] = useState("");
  const [createdId, setCreatedId] = useState<string | null>(null);
  const [listMenu, setListMenu] = useState<{
    list: TaskList;
    x: number;
    y: number;
  } | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<{
    list: TaskList;
    todoCount: number;
  } | null>(null);
  const [showViewInput, setShowViewInput] = useState(false);
  const [viewName, setViewName] = useState("");
  const [selectedViewId, setSelectedViewId] = useState<string>("");
  const [activeDragId, setActiveDragId] = useState<string | null>(null);
  // Browsers synthesize a click on the drop target after a drag ends; suppress
  // clicks inside this window so reordering never accidentally selects a task.
  const suppressClickUntilRef = useRef(0);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
  );

  const listsQuery = useQuery({
    queryKey: ["task-lists"],
    queryFn: () => ipc.taskListLists(),
  });

  const tagsQuery = useQuery({
    queryKey: ["task-tags"],
    queryFn: () => ipc.taskListTags(),
  });

  const savedViewsQuery = useQuery({
    queryKey: ["saved-views"],
    queryFn: () => ipc.savedViewList(),
  });

  const saveViewMutation = useMutation({
    mutationFn: () =>
      ipc.savedViewCreate({
        name: viewName.trim(),
        filter: { listId, status, priority, tagId, smart, showDeferred, showWaiting },
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["saved-views"] });
      setViewName("");
      setShowViewInput(false);
    },
  });

  const deleteViewMutation = useMutation({
    mutationFn: (id: string) => ipc.savedViewDelete(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["saved-views"] });
      setSelectedViewId("");
    },
  });

  const applySavedView = (view: SavedView) => {
    const f = view.filter;
    setListId(typeof f.listId === "string" ? f.listId : "all");
    const nextStatus = f.status;
    setStatus(
      nextStatus === "todo" ||
        nextStatus === "completed" ||
        nextStatus === "archived"
        ? (nextStatus as TaskStatus)
        : "active",
    );
    const nextPriority = f.priority;
    setPriority(
      nextPriority === "high" ||
        nextPriority === "medium" ||
        nextPriority === "low" ||
        nextPriority === "none"
        ? (nextPriority as TaskPriority)
        : "all",
    );
    setTagId(typeof f.tagId === "string" ? f.tagId : null);
    const nextSmart = f.smart;
    setSmart(
      smartLists.some((s) => s.id === nextSmart)
        ? (nextSmart as SmartListKind | "none")
        : "none",
    );
    setShowDeferred(f.showDeferred === true);
    setShowWaiting(f.showWaiting === true);
  };

  const fetchTasks = useCallback(
    (offset: number, limit: number) =>
      smart === "none"
        ? ipc.taskQuery({
            listId: listId === "all" ? undefined : listId,
            status:
              showDeferred || showWaiting
                ? "todo"
                : status === "active"
                  ? undefined
                  : status,
            includeArchived: status === "archived",
            priority: priority === "all" ? undefined : priority,
            tagId: tagId ?? undefined,
            search: search.trim() || undefined,
            deferredOnly: showDeferred || undefined,
            workflowState: showWaiting ? "waiting" : undefined,
            limit,
            offset,
          })
        : ipc.taskSmartList(smart, limit, offset),
    [listId, priority, search, showDeferred, showWaiting, smart, status, tagId],
  );

  const taskListQueryKey = [
    "tasks",
    "list",
    listId,
    status,
    priority,
    smart,
    tagId,
    search,
    showDeferred,
    showWaiting,
  ];

  const {
    items: tasks,
    total: taskTotal,
    hasMore: tasksHasMore,
    loading: tasksLoading,
    loadingMore: tasksLoadingMore,
    loadMore: loadMoreTasks,
  } = usePagedQuery(taskListQueryKey, fetchTasks);

  const createMutation = useMutation({
    mutationFn: () =>
      ipc.taskCreate({
        title: "新任务",
        listId: listId === "all" ? undefined : listId,
      }),
    onSuccess: (task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setSelectedId(task.id);
      setCreatedId(task.id);
    },
  });

  const createListMutation = useMutation({
    mutationFn: (name: string) => ipc.taskListCreate(name),
    onSuccess: (list) => {
      void queryClient.invalidateQueries({ queryKey: ["task-lists"] });
      setListId(list.id);
      setNewListName("");
    },
  });

  const updateListMutation = useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      ipc.taskListUpdate(id, name),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["task-lists"] });
    },
  });

  const deleteListMutation = useMutation({
    mutationFn: ({
      id,
      disposition,
    }: {
      id: string;
      disposition: ListDeleteDisposition;
    }) => ipc.taskListDelete(id, disposition),
    onSuccess: (result: DeleteListResult) => {
      void queryClient.invalidateQueries({ queryKey: ["task-lists"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      if (listId === result.listId) setListId("all");
      setDeleteTarget(null);
      useRecentActions.getState().push({
        label: `删除清单「${result.listName}」`,
        undo: async () => {
          await ipc.taskListUndoDelete(result);
          void queryClient.invalidateQueries({ queryKey: ["task-lists"] });
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
    },
  });

  useEffect(() => {
    if (!listMenu) return;
    const close = () => setListMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [listMenu]);

  const beginDeleteList = async (list: TaskList) => {
    setListMenu(null);
    const todoCount = await ipc.taskListTodoCount(list.id);
    if (todoCount > 0) {
      setDeleteTarget({ list, todoCount });
      return;
    }
    deleteListMutation.mutate({ id: list.id, disposition: "moveToInbox" });
  };

  const beginRenameList = (list: TaskList) => {
    setListMenu(null);
    const next = window.prompt("重命名清单", list.name);
    if (!next?.trim() || next.trim() === list.name) return;
    updateListMutation.mutate({ id: list.id, name: next.trim() });
  };

  const customLists = useMemo(
    () => (listsQuery.data ?? []).filter((list) => list.kind === "custom"),
    [listsQuery.data],
  );

  const hasActiveFilters =
    smart !== "none" ||
    showDeferred ||
    showWaiting ||
    status !== "active" ||
    priority !== "all" ||
    tagId !== null ||
    search.trim().length > 0;

  const applyDefer = useCallback(
    async (task: Task, availableAt: string | null) => {
      const prev = task.availableAt;
      await ipc.taskSetDefer(task.id, availableAt);
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      useRecentActions.getState().push({
        label: availableAt
          ? `推迟显示至 ${availableAt}`
          : "取消推迟显示",
        undo: async () => {
          await ipc.taskSetDefer(task.id, prev);
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
    },
    [queryClient],
  );

  const applyMarkWaiting = useCallback((task: Task) => {
    setSelectedId(task.id);
    setCreatedId(task.id);
  }, []);

  const toggleMutation = useMutation({
    mutationFn: async (task: Task) => {
      if (task.status === "completed") await ipc.taskUncomplete(task.id);
      else await ipc.taskComplete(task.id);
    },
    onSuccess: (_data, task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      const wasCompleted = task.status === "completed";
      useRecentActions.getState().push({
        label: wasCompleted ? "取消完成" : "完成",
        undo: async () => {
          if (wasCompleted) await ipc.taskComplete(task.id);
          else await ipc.taskUncomplete(task.id);
        },
      });
    },
  });

  const postponeMutation = useMutation({
    mutationFn: (id: string) => ipc.taskPostpone(id, 1),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const reorderMutation = useMutation({
    mutationFn: (orderedIds: string[]) => ipc.taskReorder(orderedIds),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const moveSelected = (direction: "up" | "down") => {
    if (!selectedId || tasks.length === 0) return;
    const list = [...tasks];
    const index = list.findIndex((t) => t.id === selectedId);
    if (index < 0) return;
    const target = direction === "up" ? index - 1 : index + 1;
    if (target < 0 || target >= list.length) return;
    const [item] = list.splice(index, 1);
    list.splice(target, 0, item);
    reorderMutation.mutate(list.map((t) => t.id));
  };

  const handleSelect = (id: string) => {
    if (Date.now() < suppressClickUntilRef.current) return;
    setSelectedId(id);
  };

  const handleDragStart = (event: DragStartEvent) => {
    setActiveDragId(String(event.active.id));
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveDragId(null);
    suppressClickUntilRef.current = Date.now() + 300;
    if (!over || active.id === over.id) return;
    const oldIndex = tasks.findIndex((t) => t.id === active.id);
    const newIndex = tasks.findIndex((t) => t.id === over.id);
    if (oldIndex < 0 || newIndex < 0) return;
    const orderedIds = arrayMove(
      tasks.map((t) => t.id),
      oldIndex,
      newIndex,
    );
    if (orderedIds.join("|") === tasks.map((t) => t.id).join("|")) return;
    queryClient.setQueryData<Task[]>(taskListQueryKey, (old) => {
      if (!old) return old;
      const order = new Map(orderedIds.map((id, i) => [id, i]));
      return [...old].sort(
        (a, b) =>
          (order.get(a.id) ?? Number.MAX_SAFE_INTEGER) -
          (order.get(b.id) ?? Number.MAX_SAFE_INTEGER),
      );
    });
    reorderMutation.mutate(orderedIds);
  };

  const handleDragCancel = () => {
    setActiveDragId(null);
    suppressClickUntilRef.current = Date.now() + 300;
  };

  const activeDragTask = useMemo(
    () => tasks.find((t) => t.id === activeDragId) ?? null,
    [tasks, activeDragId],
  );

  const selected = useMemo(
    () => tasks.find((t) => t.id === selectedId) ?? null,
    [tasks, selectedId],
  );

  const listName = showDeferred
    ? "已推迟"
    : showWaiting
      ? "等待中"
      : smart !== "none"
      ? (smartLists.find((s) => s.id === smart)?.label ?? "智能列表")
      : listId === "all"
        ? "全部"
        : (listsQuery.data?.find((l) => l.id === listId)?.name ?? "任务");

  return (
    <>
    <SplitTaskLayout
      title={listName}
      description={
        showDeferred
          ? "推迟显示中的任务 · 到日期后会回到活跃列表"
          : showWaiting
            ? "等待外部依赖的任务 · 跟进日到期会出现在今日页"
            : smart === "none"
            ? "按清单管理任务"
            : "智能列表 · 条件视图，非数据副本"
      }
      actions={
        <>
          <select
            className="h-7 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
            value={smart}
            onChange={(e) => {
              const next = e.target.value as SmartListKind | "none";
              setSmart(next);
              if (next !== "none") {
                setShowDeferred(false);
                setShowWaiting(false);
              }
            }}
          >
            {smartLists.map((item) => (
              <option key={item.id} value={item.id}>
                {item.label}
              </option>
            ))}
          </select>
          {smart === "none" ? (
            <>
              <Input
                className="h-7 w-36"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="搜索任务…"
              />
              <select
                className="h-7 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
                value={listId}
                onChange={(e) => setListId(e.target.value)}
              >
                <option value="all">全部</option>
                {(listsQuery.data ?? []).map((list) => (
                  <option key={list.id} value={list.id}>
                    {list.name}
                  </option>
                ))}
              </select>
              <select
                className="h-7 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
                value={status}
                onChange={(e) =>
                  setStatus(e.target.value as TaskStatus | "active")
                }
              >
                <option value="active">未归档</option>
                <option value="todo">待办</option>
                <option value="completed">已完成</option>
                <option value="archived">已归档</option>
              </select>
              <select
                className="h-7 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
                value={priority}
                onChange={(e) =>
                  setPriority(e.target.value as TaskPriority | "all")
                }
              >
                <option value="all">全部优先级</option>
                <option value="high">高</option>
                <option value="medium">中</option>
                <option value="low">低</option>
                <option value="none">无</option>
              </select>
              <select
                className="h-7 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
                value={tagId ?? "all"}
                onChange={(e) =>
                  setTagId(e.target.value === "all" ? null : e.target.value)
                }
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
                variant={showDeferred ? "secondary" : "ghost"}
                onClick={() => {
                  setShowDeferred((v) => !v);
                  if (!showDeferred) {
                    setSmart("none");
                    setShowWaiting(false);
                  }
                }}
              >
                已推迟
              </Button>
              <Button
                size="sm"
                variant={showWaiting ? "secondary" : "ghost"}
                onClick={() => {
                  setShowWaiting((v) => !v);
                  if (!showWaiting) {
                    setSmart("none");
                    setShowDeferred(false);
                  }
                }}
              >
                等待中
              </Button>
            </>
          ) : null}
          {(savedViewsQuery.data ?? []).length > 0 ? (
            <>
              <select
                className="h-7 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
                value={selectedViewId}
                onChange={(e) => {
                  const id = e.target.value;
                  setSelectedViewId(id);
                  const view = savedViewsQuery.data?.find((v) => v.id === id);
                  if (view) applySavedView(view);
                }}
              >
                <option value="">无</option>
                {(savedViewsQuery.data ?? []).map((view) => (
                  <option key={view.id} value={view.id}>
                    {view.name}
                  </option>
                ))}
              </select>
              {selectedViewId ? (
                <ConfirmButton
                  size="sm"
                  variant="secondary"
                  confirmLabel="确认删除"
                  confirmVariant="danger"
                  onConfirm={() => deleteViewMutation.mutate(selectedViewId)}
                  resetKey={selectedViewId}
                >
                  删除视图
                </ConfirmButton>
              ) : null}
            </>
          ) : null}
          {showViewInput ? (
            <>
              <Input
                value={viewName}
                onChange={(e) => setViewName(e.target.value)}
                placeholder="视图名称…"
                className="h-7 w-32"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && viewName.trim()) {
                    saveViewMutation.mutate();
                  } else if (e.key === "Escape") {
                    setShowViewInput(false);
                  }
                }}
              />
              <Button
                size="sm"
                variant="secondary"
                disabled={!viewName.trim() || saveViewMutation.isPending}
                onClick={() => saveViewMutation.mutate()}
              >
                保存
              </Button>
            </>
          ) : (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setShowViewInput(true)}
            >
              保存视图
            </Button>
          )}
          {selectedId ? (
            <>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => postponeMutation.mutate(selectedId)}
              >
                延期明天
              </Button>
              {smart === "none" ? (
                <>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => moveSelected("up")}
                  >
                    上移
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => moveSelected("down")}
                  >
                    下移
                  </Button>
                </>
              ) : null}
            </>
          ) : null}
          <NewTaskButton onClick={() => createMutation.mutate()} />
        </>
      }
      list={
        <div>
          {smart === "none" ? (
            <div className="border-b border-border">
              <div className="flex gap-2 border-b border-border p-2">
                <Input
                  value={newListName}
                  onChange={(e) => setNewListName(e.target.value)}
                  placeholder="新建清单…"
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && newListName.trim()) {
                      createListMutation.mutate(newListName.trim());
                    }
                  }}
                />
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={!newListName.trim() || createListMutation.isPending}
                  onClick={() => createListMutation.mutate(newListName.trim())}
                >
                  添加清单
                </Button>
              </div>
              {customLists.length > 0 ? (
                <div className="flex flex-wrap gap-1.5 px-2 py-2">
                  {customLists.map((list) => (
                    <button
                      key={list.id}
                      type="button"
                      className={`rounded-[var(--radius-control)] border px-2 py-1 text-[11px] ${
                        listId === list.id
                          ? "border-foreground bg-surface-raised text-foreground"
                          : "border-border text-muted hover:text-foreground"
                      }`}
                      onClick={() => setListId(list.id)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setListMenu({ list, x: e.clientX, y: e.clientY });
                      }}
                    >
                      {list.name}
                    </button>
                  ))}
                  <span className="self-center text-[10px] text-muted">
                    右键清单可重命名或删除
                  </span>
                </div>
              ) : null}
            </div>
          ) : null}
          {tasksLoading ? (
            <div className="p-4 text-[12px] text-muted">加载中…</div>
          ) : tasks.length === 0 ? (
            <EmptyState
              title={
                showDeferred
                  ? "没有推迟的任务"
                  : showWaiting
                    ? "没有等待中的任务"
                    : hasActiveFilters
                    ? "没有匹配的任务"
                    : listId === "all"
                      ? "还没有任务"
                      : "这个清单还是空的"
              }
              body={
                showDeferred
                  ? "推迟显示可以让任务暂时让路，到日期会自动回来。"
                  : showWaiting
                    ? "标记等待后任务会离开活跃列表，跟进日到期时出现在今日页。"
                    : hasActiveFilters
                    ? "调整筛选条件，或新建任务。"
                    : "把收件箱里的任务移过来，或直接新建。"
              }
              primaryAction={{
                label: "新建任务",
                onClick: () => createMutation.mutate(),
              }}
              secondaryAction={
                hasActiveFilters
                  ? {
                      label: "清除筛选",
                      onClick: () => {
                        setSmart("none");
                        setShowDeferred(false);
                        setShowWaiting(false);
                        setStatus("active");
                        setPriority("all");
                        setTagId(null);
                        setSearch("");
                      },
                    }
                  : undefined
              }
            />
          ) : (
            <>
              {smart === "none" ? (
                <DndContext
                  sensors={sensors}
                  collisionDetection={closestCenter}
                  onDragStart={handleDragStart}
                  onDragEnd={handleDragEnd}
                  onDragCancel={handleDragCancel}
                >
                  <SortableContext
                    items={tasks.map((t) => t.id)}
                    strategy={verticalListSortingStrategy}
                  >
                    {tasks.map((task) => (
                      <SortableTaskRow
                        key={task.id}
                        task={task}
                        selected={selectedId === task.id}
                        onSelect={() => handleSelect(task.id)}
                        onToggleComplete={() => toggleMutation.mutate(task)}
                        onRename={rename}
                        onSetDefer={applyDefer}
                        onMarkWaiting={applyMarkWaiting}
                        showDeferLabel={showDeferred}
                        showWaitingLabel={showWaiting}
                      />
                    ))}
                  </SortableContext>
                  <DragOverlay>
                    {activeDragTask ? (
                      <div className="rounded-md bg-surface shadow-lg ring-1 ring-border">
                        <TaskRow
                          task={activeDragTask}
                          onSelect={() => {}}
                          onToggleComplete={() => {}}
                        />
                      </div>
                    ) : null}
                  </DragOverlay>
                </DndContext>
              ) : (
                tasks.map((task) => (
                  <TaskRow
                    key={task.id}
                    task={task}
                    selected={selectedId === task.id}
                    onSelect={() => handleSelect(task.id)}
                    onToggleComplete={() => toggleMutation.mutate(task)}
                    onRename={rename}
                    onSetDefer={applyDefer}
                    onMarkWaiting={applyMarkWaiting}
                    showDeferLabel={smart === "deferred"}
                    showWaitingLabel={smart === "waitingFollowUp"}
                  />
                ))
              )}
              <PagedListFooter
                shown={tasks.length}
                total={taskTotal}
                hasMore={tasksHasMore}
                loadingMore={tasksLoadingMore}
                onLoadMore={loadMoreTasks}
              />
            </>
          )}
        </div>
      }
      detail={
        <TaskDetailPanel
          task={selected}
          onDeleted={() => setSelectedId(null)}
          focusTitleId={createdId}
          onStartFocus={
            selected?.status === "todo"
              ? () => void startFocus(selected.id)
              : undefined
          }
        />
      }
    />
    {listMenu ? (
      <div
        className="fixed z-50 min-w-[8rem] rounded-[var(--radius-control)] border border-border bg-surface py-1 shadow-lg"
        style={{ left: listMenu.x, top: listMenu.y }}
        onClick={(e) => e.stopPropagation()}
      >
        <button
          type="button"
          className="block w-full px-3 py-1.5 text-left text-[12px] hover:bg-surface-raised"
          onClick={() => beginRenameList(listMenu.list)}
        >
          重命名
        </button>
        <button
          type="button"
          className="block w-full px-3 py-1.5 text-left text-[12px] text-destructive hover:bg-surface-raised"
          onClick={() => void beginDeleteList(listMenu.list)}
        >
          删除…
        </button>
      </div>
    ) : null}
    {deleteTarget ? (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
        <div
          className="w-full max-w-sm rounded-[var(--radius-panel)] border border-border bg-surface p-4 shadow-lg"
          onClick={(e) => e.stopPropagation()}
        >
          <h3 className="text-[13px] font-medium text-foreground">
            删除清单「{deleteTarget.list.name}」
          </h3>
          <p className="mt-2 text-[12px] text-muted">
            清单内还有 {deleteTarget.todoCount} 个未完成任务，请选择处理方式：
          </p>
          <div className="mt-4 flex flex-col gap-2">
            <Button
              size="sm"
              variant="secondary"
              disabled={deleteListMutation.isPending}
              onClick={() =>
                deleteListMutation.mutate({
                  id: deleteTarget.list.id,
                  disposition: "moveToInbox",
                })
              }
            >
              移动到收件箱
            </Button>
            <Button
              size="sm"
              variant="secondary"
              disabled={deleteListMutation.isPending}
              onClick={() =>
                deleteListMutation.mutate({
                  id: deleteTarget.list.id,
                  disposition: "archiveTasks",
                })
              }
            >
              归档未完成任务
            </Button>
            <Button
              size="sm"
              variant="danger"
              disabled={deleteListMutation.isPending}
              onClick={() =>
                deleteListMutation.mutate({
                  id: deleteTarget.list.id,
                  disposition: "forceDelete",
                })
              }
            >
              强制删除（含任务）
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => setDeleteTarget(null)}
            >
              取消
            </Button>
          </div>
        </div>
      </div>
    ) : null}
  </>
  );
}
