import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { TaskRow } from "@/design-system/patterns/TaskRow";
import { EmptyState } from "@/components/PageScaffold";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import { ipc, type TaskPriority, type TaskStatus } from "@/ipc/client";
import {
  NewTaskButton,
  SplitTaskLayout,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";

export function TasksPage() {
  useDomainInvalidation();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [listId, setListId] = useState<string>("all");
  const [status, setStatus] = useState<TaskStatus | "active">("active");
  const [priority, setPriority] = useState<TaskPriority | "all">("all");
  const [newListName, setNewListName] = useState("");

  const listsQuery = useQuery({
    queryKey: ["task-lists"],
    queryFn: () => ipc.taskListLists(),
  });

  const tasksQuery = useQuery({
    queryKey: ["tasks", "list", listId, status, priority],
    queryFn: () =>
      ipc.taskQuery({
        listId: listId === "all" ? undefined : listId,
        status: status === "active" ? undefined : status,
        includeArchived: status === "archived",
        priority: priority === "all" ? undefined : priority,
      }),
  });

  const createMutation = useMutation({
    mutationFn: () =>
      ipc.taskCreate({
        title: "新任务",
        listId: listId === "all" ? undefined : listId,
      }),
    onSuccess: (task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setSelectedId(task.id);
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

  const toggleMutation = useMutation({
    mutationFn: async (id: string) => {
      const task = tasksQuery.data?.find((t) => t.id === id);
      if (!task) return;
      if (task.status === "completed") await ipc.taskUncomplete(id);
      else await ipc.taskComplete(id);
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const reorderMutation = useMutation({
    mutationFn: async (direction: "up" | "down") => {
      if (!selectedId || !tasksQuery.data) return;
      const list = [...tasksQuery.data];
      const index = list.findIndex((t) => t.id === selectedId);
      if (index < 0) return;
      const target = direction === "up" ? index - 1 : index + 1;
      if (target < 0 || target >= list.length) return;
      const [item] = list.splice(index, 1);
      list.splice(target, 0, item);
      await ipc.taskReorder(list.map((t) => t.id));
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const selected = useMemo(
    () => tasksQuery.data?.find((t) => t.id === selectedId) ?? null,
    [tasksQuery.data, selectedId],
  );

  const listName =
    listId === "all"
      ? "全部"
      : listsQuery.data?.find((l) => l.id === listId)?.name ?? "任务";

  return (
    <SplitTaskLayout
      title={listName}
      description="按清单管理任务"
      actions={
        <>
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
            onChange={(e) => setStatus(e.target.value as TaskStatus | "active")}
          >
            <option value="active">未归档</option>
            <option value="todo">待办</option>
            <option value="completed">已完成</option>
            <option value="archived">已归档</option>
          </select>
          <select
            className="h-7 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[12px]"
            value={priority}
            onChange={(e) => setPriority(e.target.value as TaskPriority | "all")}
          >
            <option value="all">全部优先级</option>
            <option value="high">高</option>
            <option value="medium">中</option>
            <option value="low">低</option>
            <option value="none">无</option>
          </select>
          {selectedId ? (
            <>
              <Button size="sm" variant="secondary" onClick={() => reorderMutation.mutate("up")}>
                上移
              </Button>
              <Button size="sm" variant="secondary" onClick={() => reorderMutation.mutate("down")}>
                下移
              </Button>
            </>
          ) : null}
          <NewTaskButton onClick={() => createMutation.mutate()} />
        </>
      }
      list={
        <div>
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
          {tasksQuery.isLoading ? (
            <div className="p-4 text-[12px] text-muted">加载中…</div>
          ) : (tasksQuery.data?.length ?? 0) === 0 ? (
            <EmptyState title="没有匹配的任务" body="调整筛选条件，或新建任务。" />
          ) : (
            tasksQuery.data?.map((task) => (
              <TaskRow
                key={task.id}
                task={task}
                selected={selectedId === task.id}
                onSelect={() => setSelectedId(task.id)}
                onToggleComplete={() => toggleMutation.mutate(task.id)}
              />
            ))
          )}
        </div>
      }
      detail={
        <TaskDetailPanel
          task={selected}
          onDeleted={() => setSelectedId(null)}
        />
      }
    />
  );
}
