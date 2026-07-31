import type { ReactNode } from "react";
import { Button } from "@/design-system/primitives/Button";

export function PageScaffold({
  title,
  description,
  actions,
  children,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
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
      <div className="min-h-0 flex-1 overflow-auto">{children}</div>
    </div>
  );
}

export type EmptyStateAction = {
  label: string;
  onClick: () => void;
  variant?: "default" | "secondary" | "ghost";
};

export function EmptyState({
  title,
  body,
  primaryAction,
  secondaryAction,
  hint,
}: {
  title: string;
  body: string;
  primaryAction?: EmptyStateAction;
  secondaryAction?: EmptyStateAction;
  hint?: string;
}) {
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="max-w-sm text-center">
        <h2 className="text-[14px] font-medium">{title}</h2>
        <p className="mt-1 text-[12px] text-muted">{body}</p>
        {primaryAction || secondaryAction ? (
          <div className="mt-4 flex items-center justify-center gap-2">
            {secondaryAction ? (
              <Button
                size="sm"
                variant={secondaryAction.variant ?? "secondary"}
                onClick={secondaryAction.onClick}
              >
                {secondaryAction.label}
              </Button>
            ) : null}
            {primaryAction ? (
              <Button
                size="sm"
                variant={primaryAction.variant ?? "default"}
                onClick={primaryAction.onClick}
              >
                {primaryAction.label}
              </Button>
            ) : null}
          </div>
        ) : null}
        {hint ? (
          <p className="mt-3 text-[11px] text-muted">{hint}</p>
        ) : null}
      </div>
    </div>
  );
}
