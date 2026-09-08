import { useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  DndContext,
  PointerSensor,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { ChevronRight, Folder, FolderOpen, ListTodo, Plus } from "lucide-react";
import { cn } from "@/lib/cn";
import {
  ipc,
  type ListDeleteDisposition,
  type ListGroupDeleteUndo,
  type TaskListSummary,
} from "@/ipc/client";
import { buildSidebarRows, type SidebarTaskRow } from "./listGroupTree";
import { ListDeleteDialog } from "./ListDeleteDialog";
import { useRecentActions } from "@/stores/recent-actions";

const COLLAPSE_KEY = "sidebar.groupCollapsed";

type SidebarGroupRowData = Extract<SidebarTaskRow, { kind: "group" }>;

function readCollapsed(): Set<string> {
  try {
    const raw = localStorage.getItem(COLLAPSE_KEY);
    if (!raw) return new Set();
    const parsed: unknown = JSON.parse(raw);
    if (Array.isArray(parsed)) {
      return new Set(parsed.filter((x): x is string => typeof x === "string"));
    }
  } catch {
    /* ignore */
  }
  return new Set();
}

function writeCollapsed(ids: Set<string>) {
  try {
    localStorage.setItem(COLLAPSE_KEY, JSON.stringify([...ids]));
  } catch {
    /* ignore */
  }
}

type MenuState = {
  x: number;
  y: number;
  target:
    | { kind: "area" }
    | { kind: "group"; id: string; name: string; index: number }
    | { kind: "list"; list: TaskListSummary };
  pickingGroupFor?: boolean;
};

export function TaskSidebarTree() {
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  const [collapsed, setCollapsed] = useState<Set<string>>(() => readCollapsed());
  const [renaming, setRenaming] = useState<{
    kind: "group" | "list";
    id: string;
    name: string;
  } | null>(null);
  const [draft, setDraft] = useState<null | "group" | "list">(null);
  const [draftName, setDraftName] = useState("");
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<TaskListSummary | null>(null);
  const [dragging, setDragging] = useState(false);
  const suppressClickUntilRef = useRef(0);

  const overviewQuery = useQuery({
    queryKey: ["task-list-overview"],
    queryFn: () => ipc.taskListOverview(),
    refetchInterval: 15_000,
  });

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["task-list-overview"] });
    void queryClient.invalidateQueries({ queryKey: ["task-lists"] });
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  };

  const groupRenameMutation = useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      ipc.taskListGroupRename(id, name),
    onSuccess: invalidate,
  });
  const groupDeleteMutation = useMutation({
    mutationFn: (id: string) => ipc.taskListGroupDelete(id),
    onSuccess: (undo: ListGroupDeleteUndo) => {
      invalidate();
      useRecentActions.getState().push({
        label: `解散分组「${undo.group.name}」`,
        undo: async () => {
          await ipc.taskListGroupUndoDelete(undo);
          invalidate();
        },
      });
    },
  });
  const groupReorderMutation = useMutation({
    mutationFn: (orderedIds: string[]) => ipc.taskListGroupReorder(orderedIds),
    onSuccess: invalidate,
  });
  const listRenameMutation = useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      ipc.taskListUpdate(id, name),
    onSuccess: invalidate,
  });
  const listGroupingMutation = useMutation({
    mutationFn: (input: {
      id: string;
      name: string;
      groupId?: string;
      clearGroup?: boolean;
    }) =>
      ipc.taskListUpdate(input.id, input.name, {
        groupId: input.groupId,
        clearGroup: input.clearGroup,
      }),
    onSuccess: invalidate,
  });
  const listDeleteMutation = useMutation({
    mutationFn: ({
      id,
      disposition,
    }: {
      id: string;
      disposition: ListDeleteDisposition;
    }) => ipc.taskListDelete(id, disposition),
    onSuccess: (result) => {
      invalidate();
      setDeleteTarget(null);
      useRecentActions.getState().push({
        label: `删除清单「${result.listName}」`,
        undo: async () => {
          await ipc.taskListUndoDelete(result);
          invalidate();
        },
      });
    },
  });
  const createGroupMutation = useMutation({
    mutationFn: (name: string) => ipc.taskListGroupCreate(name),
    onSuccess: invalidate,
  });
  const createListMutation = useMutation({
    mutationFn: (name: string) => ipc.taskListCreate(name),
    onSuccess: invalidate,
  });

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [menu]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
  );

  const rows = useMemo(
    () => (overviewQuery.data ? buildSidebarRows(overviewQuery.data) : []),
    [overviewQuery.data],
  );
  const groups = useMemo(
    () =>
      rows.filter(
        (r): r is SidebarGroupRowData => r.kind === "group",
      ),
    [rows],
  );
  const allLists = useMemo(
    () => rows.flatMap((r) => (r.kind === "group" ? r.lists : [r])),
    [rows],
  );

  const beginDeleteList = async (list: TaskListSummary) => {
    const todoCount = await ipc.taskListTodoCount(list.id);
    if (todoCount > 0) {
      setDeleteTarget(list);
      return;
    }
    listDeleteMutation.mutate({ id: list.id, disposition: "moveToInbox" });
  };

  const toggleCollapse = (groupId: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(groupId)) next.delete(groupId);
      else next.add(groupId);
      writeCollapsed(next);
      return next;
    });
  };

  const isActiveList = (listId: string) => location.pathname === `/tasks/${listId}`;
  const isActiveGroup = (groupId: string) =>
    location.pathname === `/tasks/group/${groupId}`;

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over) return;
    const activeId = String(active.id);
    const overId = String(over.id);
    if (activeId === overId) return;
    suppressClickUntilRef.current = Date.now() + 300;

    if (activeId.startsWith("group:")) {
      if (!overId.startsWith("group:")) return;
      const ids = groups.map((g) => g.id);
      const from = ids.indexOf(activeId.slice("group:".length));
      const to = ids.indexOf(overId.slice("group:".length));
      if (from < 0 || to < 0) return;
      const next = [...ids];
      const [moved] = next.splice(from, 1);
      next.splice(to, 0, moved);
      if (next.join("|") === ids.join("|")) return;
      groupReorderMutation.mutate(next);
      return;
    }

    if (activeId.startsWith("list:")) {
      const listId = activeId.slice("list:".length);
      const list = allLists.find((l) => l.id === listId);
      if (!list) return;
      if (overId.startsWith("group:")) {
        const gid = overId.slice("group:".length);
        if (list.groupId === gid) return;
        listGroupingMutation.mutate({ id: list.id, name: list.name, groupId: gid });
        return;
      }
      if (overId === "ungrouped") {
        if (list.groupId == null) return;
        listGroupingMutation.mutate({ id: list.id, name: list.name, clearGroup: true });
      }
    }
  };

  const submitDraft = () => {
    const name = draftName.trim();
    const kind = draft;
    setDraft(null);
    setDraftName("");
    if (!name || !kind) return;
    if (kind === "group") createGroupMutation.mutate(name);
    else createListMutation.mutate(name);
  };

  const submitRename = () => {
    if (!renaming) return;
    const current = renaming;
    setRenaming(null);
    const name = current.name.trim();
    if (!name) return;
    if (current.kind === "group") groupRenameMutation.mutate({ id: current.id, name });
    else listRenameMutation.mutate({ id: current.id, name });
  };

  const openRename = (kind: "group" | "list", id: string, name: string) => {
    setMenu(null);
    setRenaming({ kind, id, name });
  };

  const moveGroup = (index: number, delta: -1 | 1) => {
    const target = index + delta;
    if (target < 0 || target >= groups.length) return;
    const ids = groups.map((g) => g.id);
    const next = [...ids];
    const [moved] = next.splice(index, 1);
    next.splice(target, 0, moved);
    groupReorderMutation.mutate(next);
  };

  return (
    <div className="mt-1 flex flex-col">
      {/* 任务区标题：➕ 新建分组/清单 */}
      <div className="flex h-7 items-center gap-1 pl-2 pr-1">
        <span className="flex-1 text-[10px] font-medium uppercase tracking-wide text-muted">
          任务
        </span>
        <button
          type="button"
          aria-label="新建分组或清单"
          className="flex h-5 w-5 items-center justify-center rounded text-muted hover:bg-row-hover hover:text-foreground"
          onClick={(e) => {
            e.preventDefault();
            setMenu({ x: e.clientX, y: e.clientY + 4, target: { kind: "area" } });
          }}
          onContextMenu={(e) => {
            e.preventDefault();
            setMenu({ x: e.clientX, y: e.clientY, target: { kind: "area" } });
          }}
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* 新建行内输入 */}
      {draft ? (
        <div className="px-2 pb-1">
          <input
            autoFocus
            className="h-6 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-1.5 text-[12px]"
            placeholder={draft === "group" ? "分组名称…" : "清单名称…"}
            value={draftName}
            onChange={(e) => setDraftName(e.target.value)}
            onBlur={submitDraft}
            onKeyDown={(e) => {
              if (e.key === "Enter") submitDraft();
              if (e.key === "Escape") {
                setDraft(null);
                setDraftName("");
              }
            }}
          />
        </div>
      ) : null}

      <DndContext
        sensors={sensors}
        onDragStart={() => setDragging(true)}
        onDragEnd={(event) => {
          setDragging(false);
          handleDragEnd(event);
        }}
        onDragCancel={() => setDragging(false)}
      >
        <div className="flex flex-col pb-1">
          {rows.map((row, index) =>
            row.kind === "group" ? (
              <div key={row.id}>
                <GroupRow
                  row={row}
                  collapsed={collapsed.has(row.id)}
                  active={isActiveGroup(row.id)}
                  renamingValue={
                    renaming?.kind === "group" && renaming.id === row.id
                      ? renaming.name
                      : null
                  }
                  onRenameChange={(name) =>
                    setRenaming(
                      name === null ? null : { kind: "group", id: row.id, name },
                    )
                  }
                  onSubmitRename={submitRename}
                  onToggleCollapse={() => toggleCollapse(row.id)}
                  onNavigate={() => {
                    if (Date.now() < suppressClickUntilRef.current) return;
                    navigate(`/tasks/group/${row.id}`);
                  }}
                  onContextMenu={(x, y) =>
                    setMenu({
                      x,
                      y,
                      target: { kind: "group", id: row.id, name: row.name, index },
                    })
                  }
                />
                {!collapsed
                  ? row.lists.map((list) => (
                      <ListRow
                        key={list.id}
                        list={list}
                        depth={1}
                        active={isActiveList(list.id)}
                        renamingValue={
                          renaming?.kind === "list" && renaming.id === list.id
                            ? renaming.name
                            : null
                        }
                        onRenameChange={(name) =>
                          setRenaming(
                            name === null
                              ? null
                              : { kind: "list", id: list.id, name },
                          )
                        }
                        onSubmitRename={submitRename}
                        onNavigate={() => {
                          if (Date.now() < suppressClickUntilRef.current) return;
                          navigate(`/tasks/${list.id}`);
                        }}
                        onContextMenu={(x, y) =>
                          setMenu({
                            x,
                            y,
                            target: {
                              kind: "list",
                              list: {
                                id: list.id,
                                name: list.name,
                                kind: "custom",
                                groupId: list.groupId,
                                openCount: list.openCount,
                              },
                            },
                          })
                        }
                      />
                    ))
                  : null}
              </div>
            ) : (
              <ListRow
                key={row.id}
                list={{
                  id: row.id,
                  name: row.name,
                  groupId: row.groupId,
                  openCount: row.openCount,
                }}
                depth={0}
                active={isActiveList(row.id)}
                renamingValue={
                  renaming?.kind === "list" && renaming.id === row.id
                    ? renaming.name
                    : null
                }
                onRenameChange={(name) =>
                  setRenaming(
                    name === null ? null : { kind: "list", id: row.id, name },
                  )
                }
                onSubmitRename={submitRename}
                onNavigate={() => {
                  if (Date.now() < suppressClickUntilRef.current) return;
                  navigate(`/tasks/${row.id}`);
                }}
                onContextMenu={(x, y) =>
                  setMenu({
                    x,
                    y,
                    target: {
                      kind: "list",
                      list: {
                        id: row.id,
                        name: row.name,
                        kind: "custom",
                        groupId: row.groupId,
                        openCount: row.openCount,
                      },
                    },
                  })
                }
              />
            ),
          )}
          {/* 拖拽期间显示「未分组」落点：拖出分组时用 */}
          {dragging ? <UngroupedDropZone /> : null}
        </div>
      </DndContext>

      {menu ? (
        <div
          className="fixed z-50 min-w-[11rem] rounded-[var(--radius-control)] border border-border bg-surface py-1 shadow-lg"
          style={{ left: menu.x, top: menu.y }}
          onClick={(e) => e.stopPropagation()}
        >
          {menu.target.kind === "area" ? (
            <>
              <MenuButton
                label="新建分组"
                onClick={() => {
                  setMenu(null);
                  setDraft("group");
                }}
              />
              <MenuButton
                label="新建清单"
                onClick={() => {
                  setMenu(null);
                  setDraft("list");
                }}
              />
            </>
          ) : null}

          {menu.target.kind === "group" ? (
            <>
              <MenuButton
                label="重命名"
                onClick={() =>
                  openRename("group", menu.target.kind === "group" ? menu.target.id : "", menu.target.kind === "group" ? menu.target.name : "")
                }
              />
              <MenuButton
                label="上移"
                disabled={menu.target.index <= 0}
                onClick={() => {
                  const t = menu.target;
                  setMenu(null);
                  if (t.kind === "group") moveGroup(t.index, -1);
                }}
              />
              <MenuButton
                label="下移"
                disabled={menu.target.index >= groups.length - 1}
                onClick={() => {
                  const t = menu.target;
                  setMenu(null);
                  if (t.kind === "group") moveGroup(t.index, 1);
                }}
              />
              <MenuButton
                label="删除分组（清单回未分组）"
                danger
                onClick={() => {
                  const t = menu.target;
                  setMenu(null);
                  if (t.kind === "group") groupDeleteMutation.mutate(t.id);
                }}
              />
            </>
          ) : null}

          {menu.target.kind === "list" ? (
            menu.pickingGroupFor ? (
              <>
                <div className="px-3 py-1 text-[10px] text-muted">移动到分组…</div>
                {groups.map((g) => (
                  <MenuButton
                    key={g.id}
                    label={g.name}
                    onClick={() => {
                      const list = menu.target.kind === "list" ? menu.target.list : null;
                      setMenu(null);
                      if (!list || list.groupId === g.id) return;
                      listGroupingMutation.mutate({
                        id: list.id,
                        name: list.name,
                        groupId: g.id,
                      });
                    }}
                  />
                ))}
                <MenuButton
                  label="移出分组（未分组）"
                  onClick={() => {
                    const list = menu.target.kind === "list" ? menu.target.list : null;
                    setMenu(null);
                    if (!list || list.groupId == null) return;
                    listGroupingMutation.mutate({
                      id: list.id,
                      name: list.name,
                      clearGroup: true,
                    });
                  }}
                />
              </>
            ) : (
              <>
                <MenuButton
                  label="重命名"
                  onClick={() => {
                    const t = menu.target;
                    if (t.kind === "list") openRename("list", t.list.id, t.list.name);
                  }}
                />
                <MenuButton
                  label="移动到分组…"
                  onClick={() => setMenu({ ...menu, pickingGroupFor: true })}
                />
                <MenuButton
                  label="删除…"
                  danger
                  onClick={() => {
                    const t = menu.target;
                    setMenu(null);
                    if (t.kind === "list") void beginDeleteList(t.list);
                  }}
                />
              </>
            )
          ) : null}
        </div>
      ) : null}

      <ListDeleteDialog
        target={deleteTarget}
        pending={listDeleteMutation.isPending}
        onConfirm={(disposition) => {
          if (!deleteTarget) return;
          listDeleteMutation.mutate({ id: deleteTarget.id, disposition });
        }}
        onClose={() => setDeleteTarget(null)}
      />
    </div>
  );
}

function MenuButton({
  label,
  onClick,
  danger,
  disabled,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      className={cn(
        "block w-full px-3 py-1.5 text-left text-[12px] hover:bg-surface-raised disabled:opacity-40",
        danger && "text-destructive",
      )}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

function UngroupedDropZone() {
  const { setNodeRef, isOver } = useDroppable({ id: "ungrouped" });
  return (
    <div
      ref={setNodeRef}
      className={cn(
        "mx-2 mt-0.5 flex h-6 items-center justify-center rounded-[var(--radius-control)] border border-dashed px-2 text-[10px] text-muted",
        isOver ? "border-accent bg-row-active" : "border-border",
      )}
    >
      拖到此处移出分组
    </div>
  );
}

function GroupRow({
  row,
  collapsed,
  active,
  renamingValue,
  onRenameChange,
  onSubmitRename,
  onToggleCollapse,
  onNavigate,
  onContextMenu,
}: {
  row: SidebarGroupRowData;
  collapsed: boolean;
  active: boolean;
  renamingValue: string | null;
  onRenameChange: (name: string | null) => void;
  onSubmitRename: () => void;
  onToggleCollapse: () => void;
  onNavigate: () => void;
  onContextMenu: (x: number, y: number) => void;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    isDragging,
  } = useDraggable({ id: `group:${row.id}` });
  const { setNodeRef: setDropRef, isOver } = useDroppable({ id: `group:${row.id}` });

  return (
    <div
      ref={(node) => {
        setNodeRef(node);
        setDropRef(node);
      }}
      className={cn(
        "flex h-7 cursor-grab items-center gap-1 rounded-[var(--radius-control)] px-2 text-[13px]",
        "hover:bg-row-hover",
        active && "bg-row-active text-foreground",
        isDragging && "opacity-40",
        isOver && "ring-1 ring-accent",
      )}
      {...attributes}
      {...listeners}
      onClick={onNavigate}
      onContextMenu={(e) => {
        e.preventDefault();
        onContextMenu(e.clientX, e.clientY);
      }}
    >
      <button
        type="button"
        className="flex h-4 w-4 shrink-0 items-center justify-center text-muted hover:text-foreground"
        onClick={(e) => {
          e.stopPropagation();
          onToggleCollapse();
        }}
      >
        <ChevronRight
          className={cn("h-3 w-3 transition-transform", !collapsed && "rotate-90")}
        />
      </button>
      {collapsed ? (
        <Folder className="h-3.5 w-3.5 shrink-0 text-muted" />
      ) : (
        <FolderOpen className="h-3.5 w-3.5 shrink-0 text-muted" />
      )}
      {renamingValue !== null ? (
        <input
          autoFocus
          className="h-5 w-full min-w-0 rounded border border-border bg-surface-raised px-1 text-[12px]"
          value={renamingValue}
          onChange={(e) => onRenameChange(e.target.value)}
          onBlur={onSubmitRename}
          onKeyDown={(e) => {
            if (e.key === "Enter") onSubmitRename();
            if (e.key === "Escape") onRenameChange(null);
          }}
          onClick={(e) => e.stopPropagation()}
        />
      ) : (
        <span className="min-w-0 flex-1 truncate">{row.name}</span>
      )}
      {row.openCount > 0 ? (
        <span className="min-w-4 rounded px-1 text-center text-[11px] text-muted">
          {row.openCount}
        </span>
      ) : null}
    </div>
  );
}

function ListRow({
  list,
  depth,
  active,
  renamingValue,
  onRenameChange,
  onSubmitRename,
  onNavigate,
  onContextMenu,
}: {
  list: {
    id: string;
    name: string;
    groupId: string | null;
    openCount: number;
  };
  depth: number;
  active: boolean;
  renamingValue: string | null;
  onRenameChange: (name: string | null) => void;
  onSubmitRename: () => void;
  onNavigate: () => void;
  onContextMenu: (x: number, y: number) => void;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `list:${list.id}`,
  });
  return (
    <div
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      className={cn(
        "flex h-7 cursor-grab items-center gap-1 rounded-[var(--radius-control)] pr-2 text-[12px] text-muted",
        "hover:bg-row-hover hover:text-foreground",
        depth > 0 ? "pl-7" : "pl-2",
        active && "bg-row-active text-foreground",
        isDragging && "opacity-40",
      )}
      onClick={onNavigate}
      onContextMenu={(e) => {
        e.preventDefault();
        onContextMenu(e.clientX, e.clientY);
      }}
    >
      <ListTodo className="h-3.5 w-3.5 shrink-0" />
      {renamingValue !== null ? (
        <input
          autoFocus
          className="h-5 w-full min-w-0 rounded border border-border bg-surface-raised px-1 text-[12px]"
          value={renamingValue}
          onChange={(e) => onRenameChange(e.target.value)}
          onBlur={onSubmitRename}
          onKeyDown={(e) => {
            if (e.key === "Enter") onSubmitRename();
            if (e.key === "Escape") onRenameChange(null);
          }}
          onClick={(e) => e.stopPropagation()}
        />
      ) : (
        <span className="min-w-0 flex-1 truncate">{list.name}</span>
      )}
      {list.openCount > 0 ? (
        <span className="min-w-4 rounded px-1 text-center text-[11px] text-muted">
          {list.openCount}
        </span>
      ) : null}
    </div>
  );
}
