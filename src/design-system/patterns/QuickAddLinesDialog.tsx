import { useEffect } from "react";
import { Button } from "@/design-system/primitives/Button";

/**
 * 多行快速添加确认对话框：输入含多行内容时，询问创建为一条任务还是逐行多条。
 * Esc = 取消；预览区最多展示 5 行，其余以计数提示。
 */
export function QuickAddLinesDialog({
  lines,
  busy = false,
  onCreateSingle,
  onCreateMultiple,
  onCancel,
}: {
  lines: string[];
  busy?: boolean;
  onCreateSingle: () => void;
  onCreateMultiple: () => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-6"
      onClick={onCancel}
    >
      <div
        className="w-[420px] rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4 shadow-2xl"
        onClick={(event) => event.stopPropagation()}
      >
        <h2 className="text-[14px] font-semibold text-foreground">多行输入</h2>
        <p className="mt-1 text-[12px] text-muted">
          输入包含 {lines.length} 行内容，创建为一条任务还是多条任务？
        </p>
        <ul className="mt-2 max-h-28 space-y-1 overflow-auto rounded-[var(--radius-control)] border border-border bg-surface p-2 text-[12px] text-foreground">
          {lines.slice(0, 5).map((line, index) => (
            <li key={index} className="truncate">
              · {line}
            </li>
          ))}
          {lines.length > 5 ? (
            <li className="text-muted">…共 {lines.length} 行</li>
          ) : null}
        </ul>
        <div className="mt-3 flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onCancel} disabled={busy}>
            取消
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={onCreateSingle}
            disabled={busy}
          >
            创建 1 条任务
          </Button>
          <Button size="sm" onClick={onCreateMultiple} disabled={busy}>
            创建 {lines.length} 条任务
          </Button>
        </div>
      </div>
    </div>
  );
}
