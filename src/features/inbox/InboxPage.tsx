import { useMemo, useRef, useState } from "react";
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
  const [activeDragId, setActiveDragId] = useState<string | null>(null);
  // Browsers synthesize a click on the drop target after a drag ends; suppress
  // clicks inside this window so reordering never accidentally selects a task.
  const suppressClickUntilRef = useRef(0);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
  );

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

  const reorderMutation = useMutation({
    mutationFn: (orderedIds: string[]) => ipc.taskReorder(orderedIds),
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
    const oldIndex = inboxTasks.findIndex((t) => t.id === active.id);
    const newIndex = inboxTasks.findIndex((t) => t.id === over.id);
    if (oldIndex < 0 || newIndex < 0) return;
    const orderedIds = arrayMove(
      inboxTasks.map((t) => t.id),
      oldIndex,
      newIndex,
    );
    if (orderedIds.join("|") === inboxTasks.map((t) => t.id).join("|")) return;
    reorderMutation.mutate(orderedIds);
  };

  const handleDragCancel = () => {
    setActiveDragId(null);
    suppressClickUntilRef.current = Date.now() + 300;
  };

  const activeDragTask = useMemo(
    () => inboxTasks.find((t) => t.id === activeDragId) ?? null,
    [inboxTasks, activeDragId],
  );

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
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragStart={handleDragStart}
              onDragEnd={handleDragEnd}
              onDragCancel={handleDragCancel}
            >
              <SortableContext
                items={inboxTasks.map((t) => t.id)}
                strategy={verticalListSortingStrategy}
              >
                {inboxTasks.map((task) => (
                  <SortableTaskRow
                    key={task.id}
                    task={task}
                    selected={selectedId === task.id}
                    onSelect={() => handleSelect(task.id)}
                    onToggleComplete={() => toggleMutation.mutate(task.id)}
                    onRename={rename}
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
