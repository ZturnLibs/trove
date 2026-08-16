import type { ReactNode } from "react";
import { Button } from "@/design-system/primitives/Button";

export function SplitTaskLayout({
  title,
  description,
  actions,
  list,
  detail,
  footer,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
  list: ReactNode;
  detail: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex h-11 shrink-0 items-center justify-between border-b border-border px-4">
        <div className="min-w-0">
          <h1 className="truncate text-[14px] font-semibold">{title}</h1>
          {description ? (
            <p className="truncate text-[12px] text-muted">{description}</p>
          ) : null}
        </div>
        {actions ? <div className="flex items-center gap-2">{actions}</div> : null}
      </header>
      <div className="flex min-h-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col border-r border-border bg-surface">
          <section className="min-h-0 flex-1 overflow-auto">{list}</section>
          {footer ? (
            <div className="shrink-0 border-t border-border">{footer}</div>
          ) : null}
        </div>
        <aside className="w-[360px] shrink-0 overflow-hidden bg-surface-raised">
          {detail}
        </aside>
      </div>
    </div>
  );
}

export function NewTaskButton({ onClick }: { onClick: () => void }) {
  return (
    <Button size="sm" onClick={onClick}>
      新建
    </Button>
  );
}

export function TaskGroup({
  title,
  count,
  danger,
  collapsed,
  onToggle,
  alwaysShow,
  children,
}: {
  title: string;
  count: number;
  danger?: boolean;
  collapsed?: boolean;
  onToggle?: () => void;
  alwaysShow?: boolean;
  children: ReactNode;
}) {
  if (count === 0 && !onToggle && !alwaysShow) return null;
  return (
    <div>
      <button
        type="button"
        className={`sticky top-0 flex w-full items-center gap-2 bg-surface px-3 py-1.5 text-left text-[11px] font-medium uppercase tracking-wide ${
          danger ? "text-danger" : "text-muted"
        }`}
        onClick={onToggle}
      >
        <span>{title}</span>
        <span>({count})</span>
      </button>
      {!collapsed ? children : null}
    </div>
  );
}
