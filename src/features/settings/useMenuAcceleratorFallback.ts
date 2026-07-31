import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { ipc } from "@/ipc/client";

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT" ||
    target.isContentEditable
  );
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Browser / Vite preview fallback when native menu accelerators are unavailable.
 * In the packaged Tauri app, the native menu bar owns these shortcuts.
 */
export function useMenuAcceleratorFallback() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  useEffect(() => {
    if (isTauriRuntime()) return;

    const invalidate = () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["memories"] });
      void queryClient.invalidateQueries({ queryKey: ["task-counts"] });
    };

    const onKeyDown = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey;
      if (!mod) return;
      if (isEditableTarget(event.target)) return;

      const pathByDigit: Record<string, string> = {
        "1": "/today",
        "2": "/inbox",
        "3": "/tasks",
        "4": "/memory",
        "5": "/clipboard",
      };

      if (!event.shiftKey && !event.altKey && pathByDigit[event.key]) {
        event.preventDefault();
        navigate(pathByDigit[event.key]);
        return;
      }

      if (event.key === "," && !event.shiftKey && !event.altKey) {
        event.preventDefault();
        navigate("/settings");
        return;
      }

      if (event.key === "n" || event.key === "N") {
        event.preventDefault();
        if (event.altKey) {
          void ipc.memoryCreate({ title: "新记忆", body: "" }).then(() => {
            invalidate();
            navigate("/memory");
          });
        } else if (event.shiftKey) {
          const today = new Date().toISOString().slice(0, 10);
          void ipc
            .reminderCreate({
              title: "新提醒",
              fireAt: `${today}T09:00:00`,
              timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
            })
            .then(() => {
              invalidate();
              navigate("/today");
            });
        } else {
          void ipc.taskCreate({ title: "新任务" }).then(() => {
            invalidate();
            navigate("/inbox");
          });
        }
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [navigate, queryClient]);
}
