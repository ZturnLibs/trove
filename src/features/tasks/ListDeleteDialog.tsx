import { Button } from "@/design-system/primitives/Button";
import type { ListDeleteDisposition } from "@/ipc/client";

/**
 * 清单删除三选一弹窗（任务页与侧边栏共用）。
 * 清单内还有未完成任务时，让用户选择任务去向：
 * 移回收件箱 / 归档未完成 / 彻底删除。
 */
export function ListDeleteDialog({
  target,
  pending,
  onConfirm,
  onClose,
}: {
  target: { id: string; name: string } | null;
  pending: boolean;
  onConfirm: (disposition: ListDeleteDisposition) => void;
  onClose: () => void;
}) {
  if (!target) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
      <div
        className="w-full max-w-sm rounded-[var(--radius-panel)] border border-border bg-surface p-4 shadow-lg"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-[13px] font-medium text-foreground">
          删除清单「{target.name}」
        </h3>
        <p className="mt-2 text-[12px] text-muted">
          清单内还有未完成任务，请选择处理方式：
        </p>
        <div className="mt-4 flex flex-col gap-2">
          <Button
            size="sm"
            variant="secondary"
            disabled={pending}
            onClick={() => onConfirm("moveToInbox")}
          >
            移动到收件箱
          </Button>
          <Button
            size="sm"
            variant="secondary"
            disabled={pending}
            onClick={() => onConfirm("archiveTasks")}
          >
            归档未完成任务
          </Button>
          <Button
            size="sm"
            variant="danger"
            disabled={pending}
            onClick={() => onConfirm("forceDelete")}
          >
            强制删除（含任务）
          </Button>
          <Button size="sm" variant="ghost" onClick={onClose}>
            取消
          </Button>
        </div>
      </div>
    </div>
  );
}
