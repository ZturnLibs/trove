import { useQuery } from "@tanstack/react-query";
import { Button } from "@/design-system/primitives/Button";
import { BrandLogo } from "@/components/BrandLogo";
import { ipc } from "@/ipc/client";
import {
  isAppUpdaterSupported,
  useAppUpdater,
} from "@/stores/app-updater";

function updaterStatusLabel(
  phase: ReturnType<typeof useAppUpdater.getState>["phase"],
  availableVersion: string | null,
): string {
  switch (phase) {
    case "checking":
      return "正在检查更新…";
    case "upToDate":
      return "已是最新版本";
    case "available":
      return availableVersion ? `发现新版本 ${availableVersion}` : "发现新版本";
    case "downloading":
      return "正在下载更新…";
    case "installing":
      return "正在安装更新…";
    case "error":
      return "检查更新失败";
    default:
      return "尚未检查";
  }
}

export function AboutDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const healthQuery = useQuery({
    queryKey: ["app", "health"],
    queryFn: () => ipc.appHealth(),
  });
  const phase = useAppUpdater((s) => s.phase);
  const availableVersion = useAppUpdater((s) => s.availableVersion);
  const error = useAppUpdater((s) => s.error);
  const checkForUpdates = useAppUpdater((s) => s.checkForUpdates);
  const installUpdate = useAppUpdater((s) => s.installUpdate);

  if (!open) return null;

  const updaterSupported = isAppUpdaterSupported();
  const canInstall = phase === "available" && availableVersion;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <button
        type="button"
        aria-label="关闭关于"
        className="absolute inset-0 cursor-default bg-black/40"
        onClick={onClose}
      />
      <div
        className="relative w-72 rounded-[var(--radius-panel)] border border-border bg-surface p-6 text-center shadow-lg"
        role="dialog"
        aria-modal="true"
        aria-label="关于 Trove"
      >
        <BrandLogo className="mx-auto h-16 w-16" />
        <h2 className="mt-3 text-[16px] font-semibold">Trove</h2>
        <p className="mt-1 text-[12px] text-muted">
          版本 {healthQuery.data?.appVersion ?? "…"}
        </p>
        {updaterSupported ? (
          <p className="mt-1 text-[11px] text-muted">
            {updaterStatusLabel(phase, availableVersion)}
          </p>
        ) : null}
        {error ? (
          <p className="mt-1 text-[11px] text-danger">{error}</p>
        ) : null}
        <p className="mt-2 text-[12px] text-foreground">本地优先的个人工作台</p>
        <p className="mt-1 text-[11px] text-muted">© 2026 Trove</p>
        <div className="mt-4 flex flex-col gap-2">
          {updaterSupported ? (
            <>
              <Button
                size="sm"
                variant="secondary"
                disabled={phase === "checking" || phase === "downloading" || phase === "installing"}
                onClick={() => void checkForUpdates({ force: true })}
              >
                {phase === "checking" ? "检查中…" : "检查更新"}
              </Button>
              {canInstall ? (
                <Button size="sm" onClick={() => void installUpdate()}>
                  安装 {availableVersion}
                </Button>
              ) : null}
            </>
          ) : null}
          <Button size="sm" onClick={onClose}>
            关闭
          </Button>
        </div>
      </div>
    </div>
  );
}
