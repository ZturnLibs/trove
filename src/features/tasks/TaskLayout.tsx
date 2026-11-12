import { useRef, useState, type ReactNode } from "react";
import { Button } from "@/design-system/primitives/Button";

const PANE_WIDTH_KEY = "trove.detailPaneWidth";
const PANE_WIDTH_MIN = 280;
const PANE_WIDTH_MAX = 640;

function clampPaneWidth(value: number): number {
  const viewportCap = Math.max(PANE_WIDTH_MIN, window.innerWidth - 80);
  return Math.min(PANE_WIDTH_MAX, viewportCap, Math.max(PANE_WIDTH_MIN, value));
}

/** 悬浮详情栏：左缘可拖拽调宽，宽度持久化到 localStorage。 */
export function ResizableDetailPane({ children }: { children: ReactNode }) {
  const [width, setWidth] = useState<number>(() => {
    const raw = Number(localStorage.getItem(PANE_WIDTH_KEY));
    return Number.isFinite(raw) && raw >= PANE_WIDTH_MIN && raw <= PANE_WIDTH_MAX
      ? raw
      : 360;
  });
  const draggingRef = useRef(false);

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    draggingRef.current = true;
    document.body.style.userSelect = "none";
  };
  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!draggingRef.current) return;
    setWidth(clampPaneWidth(window.innerWidth - event.clientX));
  };
  const onPointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!draggingRef.current) return;
    draggingRef.current = false;
    event.currentTarget.releasePointerCapture(event.pointerId);
    document.body.style.userSelect = "";
    setWidth((current) => {
      localStorage.setItem(PANE_WIDTH_KEY, String(current));
      return current;
    });
  };

  return (
    <aside
      className="absolute inset-y-0 right-0 z-10 flex overflow-hidden border-l border-border bg-surface-raised shadow-xl"
      style={{ width }}
    >
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label="拖拽调整详情栏宽度"
        title="拖拽调整宽度"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        className="group absolute inset-y-0 left-0 z-20 w-1.5 cursor-col-resize hover:bg-accent/40"
      >
        <span className="absolute inset-y-0 left-0 w-0.5 bg-border group-hover:bg-accent/60" />
      </div>
      <div className="min-w-0 flex-1 overflow-hidden">{children}</div>
    </aside>
  );
}

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
      <div className="relative flex min-h-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col bg-surface">
          <section className="min-h-0 flex-1 overflow-auto">{list}</section>
          {footer ? (
            <div className="shrink-0 border-t border-border">{footer}</div>
          ) : null}
        </div>
        {/* 详情悬浮于内容区之上，不占用列表宽度；未选中任务时不渲染。 */}
        {detail ? <ResizableDetailPane>{detail}</ResizableDetailPane> : null}
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
