import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import {
  ClipboardList,
  Inbox,
  ListTodo,
  NotebookPen,
  Settings,
  SunMedium,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { ipc } from "@/ipc/client";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { OnboardingOverlay } from "@/features/settings/OnboardingOverlay";

const navItems = [
  { to: "/today", label: "今日", icon: SunMedium, badge: "overdue" as const },
  { to: "/inbox", label: "收件箱", icon: Inbox, badge: "inbox" as const },
  { to: "/tasks", label: "任务", icon: ListTodo, badge: null },
  { to: "/memory", label: "记忆", icon: NotebookPen, badge: null },
  { to: "/clipboard", label: "剪切板", icon: ClipboardList, badge: null },
] as const;

export function MainShell() {
  useDomainInvalidation();
  const [backupError, setBackupError] = useState<string | null>(null);
  const countsQuery = useQuery({
    queryKey: ["task-counts"],
    queryFn: () => ipc.taskCounts(),
    refetchInterval: 15_000,
  });
  const healthQuery = useQuery({
    queryKey: ["app", "health"],
    queryFn: () => ipc.appHealth(),
    refetchInterval: 60_000,
  });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<{ message?: string }>("backup://failed", (event) => {
      setBackupError(event.payload.message ?? "自动备份失败");
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (healthQuery.data?.backup.lastError) {
      setBackupError(healthQuery.data.backup.lastError);
    }
  }, [healthQuery.data?.backup.lastError]);

  const badgeFor = (kind: "overdue" | "inbox" | null) => {
    if (!kind || !countsQuery.data) return null;
    const value = countsQuery.data[kind];
    return value > 0 ? value : null;
  };

  return (
    <div className="flex h-full bg-surface text-foreground">
      <aside className="flex w-[200px] shrink-0 flex-col border-r border-border bg-sidebar">
        <div className="flex h-11 items-center px-3 text-[15px] font-semibold tracking-tight">
          工作台
        </div>
        <nav className="flex flex-1 flex-col gap-0.5 px-2 py-1">
          {navItems.map(({ to, label, icon: Icon, badge }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                cn(
                  "flex h-8 items-center gap-2 rounded-[var(--radius-control)] px-2 text-[13px] text-muted hover:bg-row-hover hover:text-foreground",
                  isActive && "bg-row-active text-foreground",
                )
              }
            >
              <Icon className="h-4 w-4" />
              <span className="flex-1">{label}</span>
              {badgeFor(badge) ? (
                <span className="min-w-4 rounded px-1 text-center text-[11px] text-muted">
                  {badgeFor(badge)}
                </span>
              ) : null}
            </NavLink>
          ))}
          <div className="mt-auto border-t border-border pt-2">
            <NavLink
              to="/settings"
              className={({ isActive }) =>
                cn(
                  "flex h-8 items-center gap-2 rounded-[var(--radius-control)] px-2 text-[13px] text-muted hover:bg-row-hover hover:text-foreground",
                  isActive && "bg-row-active text-foreground",
                )
              }
            >
              <Settings className="h-4 w-4" />
              设置
            </NavLink>
          </div>
        </nav>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        {backupError ? (
          <div className="flex items-center justify-between gap-3 border-b border-danger/30 bg-danger/5 px-4 py-2 text-[12px] text-danger">
            <span>{backupError}</span>
            <div className="flex items-center gap-2">
              <NavLink to="/settings" className="underline">
                去设置
              </NavLink>
              <button
                type="button"
                className="underline"
                onClick={() => setBackupError(null)}
              >
                关闭
              </button>
            </div>
          </div>
        ) : null}
        <Outlet />
      </main>
      <OnboardingOverlay />
    </div>
  );
}
