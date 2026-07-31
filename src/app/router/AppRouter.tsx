import { useEffect } from "react";
import { BrowserRouter, Navigate, Route, Routes, useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MainShell } from "@/app/layouts/MainShell";
import { TodayPage } from "@/features/today/TodayPage";
import { InboxPage } from "@/features/inbox/InboxPage";
import { TasksPage } from "@/features/tasks/TasksPage";
import { MemoryPage } from "@/features/memory/MemoryPage";
import { ClipboardPage } from "@/features/clipboard/ClipboardPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { useMenuAcceleratorFallback } from "@/features/settings/useMenuAcceleratorFallback";
import { QuickWindow } from "@/features/search/QuickWindow";

function MainNavigateListener() {
  const navigate = useNavigate();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<string>("main://navigate", (event) => {
      if (event.payload) navigate(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [navigate]);

  return null;
}

function MainRoutes() {
  return (
    <BrowserRouter>
      <MainNavigateListener />
      <MainMenuAcceleratorFallback />
      <Routes>
        <Route element={<MainShell />}>
          <Route index element={<Navigate to="/today" replace />} />
          <Route path="/today" element={<TodayPage />} />
          <Route path="/inbox" element={<InboxPage />} />
          <Route path="/tasks" element={<TasksPage />} />
          <Route path="/tasks/:listId" element={<TasksPage />} />
          <Route path="/memory" element={<MemoryPage />} />
          <Route path="/clipboard" element={<ClipboardPage />} />
          <Route path="/settings/*" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

function MainMenuAcceleratorFallback() {
  useMenuAcceleratorFallback();
  return null;
}

export function AppRouter() {
  let label = "main";
  try {
    label = getCurrentWindow().label;
  } catch {
    // Browser-only Vite preview.
  }
  if (label === "quick") {
    return <QuickWindow />;
  }
  return <MainRoutes />;
}
