import { Button } from "@/design-system/primitives/Button";
import { isAppUpdaterSupported, useAppUpdater } from "@/stores/app-updater";

export function UpdateToast() {
  const phase = useAppUpdater((s) => s.phase);
  const availableVersion = useAppUpdater((s) => s.availableVersion);
  const releaseNotes = useAppUpdater((s) => s.releaseNotes);
  const dismissedVersion = useAppUpdater((s) => s.dismissedVersion);
  const installUpdate = useAppUpdater((s) => s.installUpdate);
  const dismissUpdate = useAppUpdater((s) => s.dismissUpdate);

  if (!isAppUpdaterSupported()) return null;
  if (phase !== "available" || !availableVersion) return null;
  if (dismissedVersion === availableVersion) return null;

  return (
    <div className="pointer-events-none fixed bottom-16 left-1/2 z-50 w-max max-w-[min(90vw,420px)] -translate-x-1/2">
      <div
        aria-live="polite"
        className="pointer-events-auto rounded-[var(--radius-panel)] border border-border bg-surface-raised px-3 py-2 shadow-lg"
      >
        <p className="text-[13px] font-medium text-foreground">
          Trove {availableVersion} 可用
        </p>
        {releaseNotes ? (
          <p className="mt-1 line-clamp-2 text-[11px] text-muted">{releaseNotes}</p>
        ) : null}
        <div className="mt-2 flex justify-end gap-2">
          <Button size="sm" variant="secondary" onClick={dismissUpdate}>
            稍后
          </Button>
          <Button size="sm" onClick={() => void installUpdate()}>
            立即更新
          </Button>
        </div>
      </div>
    </div>
  );
}

export function UpdateProgressBanner() {
  const phase = useAppUpdater((s) => s.phase);
  const progress = useAppUpdater((s) => s.progress);

  if (!isAppUpdaterSupported()) return null;
  if (phase !== "downloading" && phase !== "installing") return null;

  return (
    <div
      aria-live="polite"
      className="border-b border-border bg-surface-raised px-4 py-2 text-[12px]"
    >
      <div className="flex items-center justify-between gap-3">
        <span className="text-foreground">
          {phase === "installing" ? "正在安装更新…" : "正在下载更新…"}
        </span>
        {progress !== null ? <span className="text-muted">{progress}%</span> : null}
      </div>
      {progress !== null ? (
        <div className="mt-1 h-1 overflow-hidden rounded-full bg-border">
          <div
            className="h-full bg-accent transition-[width]"
            style={{ width: `${progress}%` }}
          />
        </div>
      ) : null}
    </div>
  );
}
