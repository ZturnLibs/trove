import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { TaskRow } from "@/design-system/patterns/TaskRow";
import { EmptyState } from "@/components/PageScaffold";
import { Button } from "@/design-system/primitives/Button";
import { ipc } from "@/ipc/client";
import {
  NewTaskButton,
  SplitTaskLayout,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";

export function InboxPage() {
  useDomainInvalidation();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const inboxQuery = useQuery({
    queryKey: ["tasks", "inbox"],
    queryFn: () =>
      ipc.taskQuery({ inboxOnly: true, status: "todo" }),
  });

  const createMutation = useMutation({
    mutationFn: () => ipc.taskCreate({ title: "新任务" }),
    onSuccess: (task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setSelectedId(task.id);
    },
  });

  const toggleMutation = useMutation({
    mutationFn: async (id: string) => {
      const task = inboxQuery.data?.find((t) => t.id === id);
      if (!task) return;
      if (task.status === "completed") await ipc.taskUncomplete(id);
      else await ipc.taskComplete(id);
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const moveUp = useMutation({
    mutationFn: async (id: string) => {
      const list = [...(inboxQuery.data ?? [])];
      const index = list.findIndex((t) => t.id === id);
      if (index <= 0) return;
      const [item] = list.splice(index, 1);
      list.splice(index - 1, 0, item);
      await ipc.taskReorder(list.map((t) => t.id));
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const moveDown = useMutation({
    mutationFn: async (id: string) => {
      const list = [...(inboxQuery.data ?? [])];
      const index = list.findIndex((t) => t.id === id);
      if (index < 0 || index >= list.length - 1) return;
      const [item] = list.splice(index, 1);
      list.splice(index + 1, 0, item);
      await ipc.taskReorder(list.map((t) => t.id));
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const selected = useMemo(
    () => inboxQuery.data?.find((t) => t.id === selectedId) ?? null,
    [inboxQuery.data, selectedId],
  );

  return (
    <SplitTaskLayout
      title="收件箱"
      description={`${inboxQuery.data?.length ?? 0} 项待整理`}
      actions={
        <>
          {selectedId ? (
            <>
              <Button size="sm" variant="secondary" onClick={() => moveUp.mutate(selectedId)}>
                上移
              </Button>
              <Button size="sm" variant="secondary" onClick={() => moveDown.mutate(selectedId)}>
                下移
              </Button>
            </>
          ) : null}
          <NewTaskButton onClick={() => createMutation.mutate()} />
        </>
      }
      list={
        inboxQuery.isLoading ? (
          <div className="p-4 text-[12px] text-muted">加载中…</div>
        ) : (inboxQuery.data?.length ?? 0) === 0 ? (
          <EmptyState
            title="收件箱为空"
            body="新任务默认进入这里。可用全局快捷键快速捕获。"
          />
        ) : (
          <div>
            {inboxQuery.data?.map((task) => (
              <TaskRow
                key={task.id}
                task={task}
                selected={selectedId === task.id}
                onSelect={() => setSelectedId(task.id)}
                onToggleComplete={() => toggleMutation.mutate(task.id)}
              />
            ))}
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
