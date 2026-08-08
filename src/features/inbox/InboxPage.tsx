import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { TaskRow } from "@/design-system/patterns/TaskRow";
import { EmptyState } from "@/components/PageScaffold";
import { Button } from "@/design-system/primitives/Button";
import { ipc } from "@/ipc/client";
import { formatShortcutLabel } from "@/lib/shortcuts";
import {
  NewTaskButton,
  SplitTaskLayout,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { useTaskRename } from "@/features/tasks/useTaskRename";

export function InboxPage() {
  useDomainInvalidation();
  const rename = useTaskRename();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createdId, setCreatedId] = useState<string | null>(null);

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });

  const inboxQuery = useQuery({
    queryKey: ["tasks", "inbox"],
    queryFn: () =>
      ipc.taskQuery({ inboxOnly: true, status: "todo" }),
  });

  const inboxTasks = inboxQuery.data?.items ?? [];

  const createMutation = useMutation({
    mutationFn: () => ipc.taskCreate({ title: "新任务" }),
    onSuccess: (task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setSelectedId(task.id);
      setCreatedId(task.id);
    },
  });

  const toggleMutation = useMutation({
    mutationFn: async (id: string) => {
      const task = inboxTasks.find((t) => t.id === id);
      if (!task) return;
      if (task.status === "completed") await ipc.taskUncomplete(id);
      else await ipc.taskComplete(id);
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const moveUp = useMutation({
    mutationFn: async (id: string) => {
      const list = [...inboxTasks];
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
      const list = [...inboxTasks];
      const index = list.findIndex((t) => t.id === id);
      if (index < 0 || index >= list.length - 1) return;
      const [item] = list.splice(index, 1);
      list.splice(index + 1, 0, item);
      await ipc.taskReorder(list.map((t) => t.id));
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const selected = useMemo(
    () => inboxTasks.find((t) => t.id === selectedId) ?? null,
    [inboxTasks, selectedId],
  );

  return (
    <SplitTaskLayout
      title="收件箱"
      description={`${inboxQuery.data?.total ?? inboxTasks.length} 项待整理`}
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
        ) : inboxTasks.length === 0 ? (
          <EmptyState
            title="收件箱为空"
            body="新任务会先出现在这里。可用全局快捷键随时捕获。"
            primaryAction={{
              label: "新建任务",
              onClick: () => createMutation.mutate(),
            }}
            hint={
              settingsQuery.data?.shortcuts.quickCapture
                ? `快速记录：${formatShortcutLabel(settingsQuery.data.shortcuts.quickCapture)}`
                : undefined
            }
          />
        ) : (
          <div>
            {inboxTasks.map((task) => (
              <TaskRow
                key={task.id}
                task={task}
                selected={selectedId === task.id}
                onSelect={() => setSelectedId(task.id)}
                onToggleComplete={() => toggleMutation.mutate(task.id)}
                onRename={rename}
              />
            ))}
          </div>
        )
      }
      detail={
        <TaskDetailPanel
          task={selected}
          onDeleted={() => setSelectedId(null)}
          focusTitleId={createdId}
        />
      }
    />
  );
}
