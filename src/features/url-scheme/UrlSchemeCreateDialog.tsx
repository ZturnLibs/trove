import { useState } from "react";
import { Button } from "@/design-system/primitives/Button";
import { ipc } from "@/ipc/client";

export type UrlSchemePendingCreate = {
  action: "createPreview";
  kind: "task" | "reminder" | "memory";
  title: string;
  notes?: string;
  dueDate?: string;
  fireAt?: string;
};

const kindLabel: Record<UrlSchemePendingCreate["kind"], string> = {
  task: "任务",
  reminder: "提醒",
  memory: "记忆",
};

export function UrlSchemeCreateDialog({
  pending,
  onClose,
}: {
  pending: UrlSchemePendingCreate | null;
  onClose: () => void;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!pending) return null;

  async function confirm() {
    setSaving(true);
    setError(null);
    try {
      if (pending!.kind === "task") {
        await ipc.taskCreate({
          title: pending!.title,
          notes: pending!.notes,
          dueDate: pending!.dueDate ?? null,
        });
      } else if (pending!.kind === "reminder") {
        if (!pending!.fireAt) {
          setError("提醒缺少触发时间");
          return;
        }
        await ipc.reminderCreate({
          title: pending!.title,
          notes: pending!.notes,
          fireAt: pending!.fireAt,
        });
      } else {
        await ipc.memoryCreate({
          title: pending!.title,
          body: pending!.notes,
        });
      }
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建失败");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <button
        type="button"
        aria-label="取消创建"
        className="absolute inset-0 cursor-default bg-black/40"
        onClick={onClose}
      />
      <div
        className="relative w-full max-w-md rounded-[var(--radius-panel)] border border-border bg-surface p-4 shadow-xl"
        role="dialog"
        aria-modal="true"
        aria-label="确认创建"
      >
        <h2 className="text-[14px] font-semibold">通过链接创建{kindLabel[pending.kind]}</h2>
        <p className="mt-1 text-[11px] text-muted">请确认后再写入 Trove</p>

        <dl className="mt-4 space-y-2 text-[12px]">
          <div>
            <dt className="text-muted">标题</dt>
            <dd className="mt-0.5 font-medium">{pending.title}</dd>
          </div>
          {pending.notes ? (
            <div>
              <dt className="text-muted">备注</dt>
              <dd className="mt-0.5 whitespace-pre-wrap">{pending.notes}</dd>
            </div>
          ) : null}
          {pending.dueDate ? (
            <div>
              <dt className="text-muted">截止日期</dt>
              <dd className="mt-0.5">{pending.dueDate}</dd>
            </div>
          ) : null}
          {pending.fireAt ? (
            <div>
              <dt className="text-muted">提醒时间</dt>
              <dd className="mt-0.5">{pending.fireAt}</dd>
            </div>
          ) : null}
        </dl>

        {error ? <p className="mt-3 text-[11px] text-destructive">{error}</p> : null}

        <div className="mt-4 flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={saving}>
            取消
          </Button>
          <Button size="sm" onClick={() => void confirm()} disabled={saving}>
            {saving ? "创建中…" : "确认创建"}
          </Button>
        </div>
      </div>
    </div>
  );
}
