import { Button } from "@/design-system/primitives/Button";
import { cn } from "@/lib/cn";

export type PermissionBannerKind =
  | "notification"
  | "accessibility"
  | "clipboard_paused"
  | "backup_failed"
  | "shortcut_conflict"
  | "info";

const toneClass: Record<PermissionBannerKind, string> = {
  notification: "border-border bg-surface-raised text-foreground",
  accessibility: "border-border bg-surface-raised text-foreground",
  clipboard_paused: "border-border bg-surface-raised text-foreground",
  backup_failed: "border-danger/40 bg-danger/5 text-danger",
  shortcut_conflict: "border-border bg-surface-raised text-foreground",
  info: "border-border bg-surface-raised text-foreground",
};

export function PermissionBanner({
  kind = "info",
  title,
  body,
  primaryAction,
  secondaryAction,
  onDismiss,
  className,
}: {
  kind?: PermissionBannerKind;
  title: string;
  body: string;
  primaryAction?: { label: string; onClick: () => void };
  secondaryAction?: { label: string; onClick: () => void };
  onDismiss?: () => void;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex items-start justify-between gap-3 border-b px-4 py-2 text-[12px]",
        toneClass[kind],
        className,
      )}
      role="status"
    >
      <div className="min-w-0 flex-1">
        <p className="font-medium">{title}</p>
        <p className="mt-0.5 opacity-90">{body}</p>
      </div>
      <div className="flex shrink-0 items-center gap-1.5">
        {secondaryAction ? (
          <Button size="sm" variant="ghost" onClick={secondaryAction.onClick}>
            {secondaryAction.label}
          </Button>
        ) : null}
        {primaryAction ? (
          <Button
            size="sm"
            variant={kind === "backup_failed" ? "secondary" : "default"}
            onClick={primaryAction.onClick}
          >
            {primaryAction.label}
          </Button>
        ) : null}
        {onDismiss ? (
          <Button size="sm" variant="ghost" onClick={onDismiss} aria-label="关闭">
            稍后
          </Button>
        ) : null}
      </div>
    </div>
  );
}

const SESSION_PREFIX = "workbench.banner.dismissed.";

export function isBannerDismissed(id: string): boolean {
  try {
    return sessionStorage.getItem(SESSION_PREFIX + id) === "1";
  } catch {
    return false;
  }
}

export function dismissBanner(id: string) {
  try {
    sessionStorage.setItem(SESSION_PREFIX + id, "1");
  } catch {
    // ignore
  }
}
