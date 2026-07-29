import { NavLink, Outlet } from "react-router-dom";
import {
  ClipboardList,
  Inbox,
  ListTodo,
  NotebookPen,
  Settings,
  SunMedium,
} from "lucide-react";
import { cn } from "@/lib/cn";

const navItems = [
  { to: "/today", label: "今日", icon: SunMedium },
  { to: "/inbox", label: "收件箱", icon: Inbox },
  { to: "/tasks", label: "任务", icon: ListTodo },
  { to: "/memory", label: "记忆", icon: NotebookPen },
  { to: "/clipboard", label: "剪切板", icon: ClipboardList },
] as const;

export function MainShell() {
  return (
    <div className="flex h-full bg-surface text-foreground">
      <aside className="flex w-[200px] shrink-0 flex-col border-r border-border bg-sidebar">
        <div className="flex h-11 items-center px-3 text-[15px] font-semibold tracking-tight">
          工作台
        </div>
        <nav className="flex flex-1 flex-col gap-0.5 px-2 py-1">
          {navItems.map(({ to, label, icon: Icon }) => (
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
              {label}
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
        <Outlet />
      </main>
    </div>
  );
}
