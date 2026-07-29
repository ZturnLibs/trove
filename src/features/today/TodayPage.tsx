import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { TaskRow } from "@/design-system/patterns/TaskRow";
import { EmptyState } from "@/components/PageScaffold";
import { ipc } from "@/ipc/client";
import {
  NewTaskButton,
  SplitTaskLayout,
  TaskGroup,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";

export function TodayPage() {
  useDomainInvalidation();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [completedCollapsed, setCompletedCollapsed] = useState(true);

  const todayQuery = useQuery({
    queryKey: ["tasks", "today"],
    queryFn: () => ipc.taskToday(),
  });

  const createMutation = useMutation({
    mutationFn: () =>
      ipc.taskCreate({
        title: "新任务",
        dueDate: todayQuery.data?.today,
      }),
    onSuccess: (task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setSelectedId(task.id);
    },
  });

  const toggleMutation = useMutation({
    mutationFn: async (id: string) => {
      const all = [
        ...(todayQuery.data?.overdue ?? []),
        ...(todayQuery.data?.dueToday ?? []),
        ...(todayQuery.data?.completedToday ?? []),
      ];
      const task = all.find((t) => t.id === id);
      if (!task) return;
      if (task.status === "completed") await ipc.taskUncomplete(id);
      else await ipc.taskComplete(id);
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const selected = useMemo(() => {
    const data = todayQuery.data;
    if (!data || !selectedId) return null;
    return (
      [...data.overdue, ...data.dueToday, ...data.completedToday].find(
        (t) => t.id === selectedId,
      ) ?? null
    );
  }, [todayQuery.data, selectedId]);

  const data = todayQuery.data;
  const empty =
    data &&
    data.overdue.length === 0 &&
    data.dueToday.length === 0 &&
    data.completedToday.length === 0;

  return (
    <SplitTaskLayout
      title="今日"
      description={data ? data.today : "加载中…"}
      actions={
        <NewTaskButton onClick={() => createMutation.mutate()} />
      }
      list={
        todayQuery.isLoading ? (
          <div className="p-4 text-[12px] text-muted">加载中…</div>
        ) : empty ? (
          <EmptyState
            title="今日还没有事项"
            body="新建带今日截止日期的任务，或用全局快捷键快速记录。"
          />
        ) : (
          <div>
            <TaskGroup title="逾期" count={data?.overdue.length ?? 0} danger>
              {data?.overdue.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  overdue
                  selected={selectedId === task.id}
                  onSelect={() => setSelectedId(task.id)}
                  onToggleComplete={() => toggleMutation.mutate(task.id)}
                />
              ))}
            </TaskGroup>
            <TaskGroup title="今日任务" count={data?.dueToday.length ?? 0}>
              {data?.dueToday.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  selected={selectedId === task.id}
                  onSelect={() => setSelectedId(task.id)}
                  onToggleComplete={() => toggleMutation.mutate(task.id)}
                />
              ))}
            </TaskGroup>
            <TaskGroup
              title="今日已完成"
              count={data?.completedToday.length ?? 0}
              collapsed={completedCollapsed}
              onToggle={() => setCompletedCollapsed((v) => !v)}
            >
              {data?.completedToday.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  selected={selectedId === task.id}
                  onSelect={() => setSelectedId(task.id)}
                  onToggleComplete={() => toggleMutation.mutate(task.id)}
                />
              ))}
            </TaskGroup>
          </div>
        )
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
