import type { Task } from "@/ipc/client";
import { sameListReorderIds, type BoardColumns } from "./splitBoardColumns";

export type BoardColumnKey = keyof BoardColumns;

/**
 * 看板拖拽分发的纯决策（unit-testable，沿 splitBoardColumns.ts 模式）。
 *
 * BoardPage.handleDragEnd 只负责把决策映射到 mutation；全部分支语义在此：
 * - 跨列：doing（todo 且未在 doing → stage=doing）、
 *         todo（completed → uncomplete；doing 态 → stage=null）、
 *         done（todo → complete）
 * - 同列：done 列不支持手动重排；drop-on-self 与列 droppable 落点为 no-op；
 *         其余按 sameListReorderIds 生成整段新序
 * - 找不到被拖卡 / 落点无法解析列 → none
 */

export type BoardDragDecision =
  | { kind: "none" }
  | { kind: "stage"; id: string; stage: "doing" | null }
  | { kind: "complete"; id: string }
  | { kind: "uncomplete"; id: string }
  | { kind: "reorder"; activeId: string; overId: string; orderedIds: string[] };

export function columnOfTask(
  columns: BoardColumns,
  taskId: string,
): BoardColumnKey | null {
  if (columns.todo.some((t) => t.id === taskId)) return "todo";
  if (columns.doing.some((t) => t.id === taskId)) return "doing";
  if (columns.done.some((t) => t.id === taskId)) return "done";
  return null;
}

function columnOfOverId(
  columns: BoardColumns,
  overId: string,
): BoardColumnKey | null {
  // over 可能是卡片（useSortable）或列 droppable（空白区/空列）。
  return (
    columnOfTask(columns, overId) ??
    (overId === "col:todo" || overId === "col:doing" || overId === "col:done"
      ? (overId.slice(4) as BoardColumnKey)
      : null)
  );
}

export function resolveBoardDrag(
  columns: BoardColumns,
  dragged: Task,
  overId: string,
): BoardDragDecision {
  const source = columnOfTask(columns, dragged.id);
  const target = columnOfOverId(columns, overId);
  if (!source || !target) return { kind: "none" };

  if (target === source) {
    // 同列：仅同清单段支持手动排序（sort_order 是清单内维度）；
    // 完成列按完成时间排序，不支持手动重排。
    if (source === "done") return { kind: "none" };
    if (overId === dragged.id || overId.startsWith("col:")) {
      return { kind: "none" };
    }
    const orderedIds = sameListReorderIds(
      columns[source],
      dragged.id,
      overId,
    );
    if (!orderedIds) return { kind: "none" };
    return {
      kind: "reorder",
      activeId: dragged.id,
      overId,
      orderedIds,
    };
  }

  if (target === "doing") {
    if (dragged.status === "todo" && dragged.boardStage !== "doing") {
      return { kind: "stage", id: dragged.id, stage: "doing" };
    }
    return { kind: "none" };
  }
  if (target === "todo") {
    if (dragged.status === "completed") {
      return { kind: "uncomplete", id: dragged.id };
    }
    if (dragged.boardStage === "doing") {
      return { kind: "stage", id: dragged.id, stage: null };
    }
    return { kind: "none" };
  }
  // done
  if (dragged.status === "todo") {
    return { kind: "complete", id: dragged.id };
  }
  return { kind: "none" };
}
