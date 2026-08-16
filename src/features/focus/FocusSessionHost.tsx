import { useEffect } from "react";
import { PermissionBanner } from "@/components/PermissionBanner";
import { FocusOverlay } from "@/features/focus/FocusOverlay";
import { useFocusSession } from "@/stores/focus-session";

export function FocusSessionHost() {
  const abandonedNotice = useFocusSession((s) => s.abandonedNotice);
  const dismissAbandonedNotice = useFocusSession((s) => s.dismissAbandonedNotice);
  const recoverStaleSession = useFocusSession((s) => s.recoverStaleSession);

  useEffect(() => {
    void recoverStaleSession();
  }, [recoverStaleSession]);

  return (
    <>
      {abandonedNotice ? (
        <div className="pointer-events-none fixed inset-x-0 top-0 z-[55] flex justify-center p-2">
          <div className="pointer-events-auto w-full max-w-xl">
            <PermissionBanner
              kind="info"
              title="专注会话已结束"
              body={abandonedNotice}
              onDismiss={dismissAbandonedNotice}
            />
          </div>
        </div>
      ) : null}
      <FocusOverlay />
    </>
  );
}
