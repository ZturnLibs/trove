import { useEffect, useState } from "react";
import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";
import {
  PermissionBanner,
  dismissBanner,
  isBannerDismissed,
} from "@/components/PermissionBanner";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Shows when system notification permission is not granted. */
export function NotificationPermissionBanner() {
  const [visible, setVisible] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    if (isBannerDismissed("notification")) return;
    let cancelled = false;
    void isPermissionGranted()
      .then((granted) => {
        if (!cancelled && !granted) setVisible(true);
      })
      .catch(() => {
        // Plugin unavailable in browser preview.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!visible) return null;

  return (
    <PermissionBanner
      kind="notification"
      title="未开启通知"
      body="到时不会弹出系统通知，「今日」里仍看得到提醒。"
      primaryAction={{
        label: busy ? "请求中…" : "开启通知",
        onClick: () => {
          setBusy(true);
          void requestPermission()
            .then((result) => {
              if (result === "granted") setVisible(false);
            })
            .finally(() => setBusy(false));
        },
      }}
      onDismiss={() => {
        dismissBanner("notification");
        setVisible(false);
      }}
    />
  );
}
