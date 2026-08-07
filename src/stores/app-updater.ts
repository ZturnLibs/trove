import { create } from "zustand";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdaterPhase =
  | "idle"
  | "checking"
  | "upToDate"
  | "available"
  | "downloading"
  | "installing"
  | "error";

const LAST_CHECKED_KEY = "trove.updater.lastCheckedAt";
const AUTO_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function isUpdaterEnabled(): boolean {
  return isTauriRuntime() && !import.meta.env.DEV;
}

function readLastCheckedAt(): string | null {
  try {
    return localStorage.getItem(LAST_CHECKED_KEY);
  } catch {
    return null;
  }
}

function writeLastCheckedAt(iso: string) {
  try {
    localStorage.setItem(LAST_CHECKED_KEY, iso);
  } catch {
    // ignore quota / private mode
  }
}

function shouldAutoCheck(lastCheckedAt: string | null): boolean {
  if (!lastCheckedAt) return true;
  const last = Date.parse(lastCheckedAt);
  if (Number.isNaN(last)) return true;
  return Date.now() - last >= AUTO_CHECK_INTERVAL_MS;
}

type AppUpdaterState = {
  phase: UpdaterPhase;
  availableVersion: string | null;
  releaseNotes: string | null;
  progress: number | null;
  error: string | null;
  lastCheckedAt: string | null;
  dismissedVersion: string | null;
  pendingUpdate: Update | null;
  checkForUpdates: (options?: { force?: boolean }) => Promise<void>;
  installUpdate: () => Promise<void>;
  dismissUpdate: () => void;
  clearError: () => void;
};

export const useAppUpdater = create<AppUpdaterState>((set, get) => ({
  phase: "idle",
  availableVersion: null,
  releaseNotes: null,
  progress: null,
  error: null,
  lastCheckedAt: readLastCheckedAt(),
  dismissedVersion: null,
  pendingUpdate: null,

  checkForUpdates: async (options) => {
    if (!isUpdaterEnabled()) return;

    const { phase } = get();
    if (phase === "checking" || phase === "downloading" || phase === "installing") {
      return;
    }

    const lastCheckedAt = readLastCheckedAt();
    if (!options?.force && !shouldAutoCheck(lastCheckedAt)) {
      return;
    }

    set({ phase: "checking", error: null, progress: null });

    try {
      const update = await check();
      const checkedAt = new Date().toISOString();
      writeLastCheckedAt(checkedAt);

      if (!update) {
        set({
          phase: "upToDate",
          availableVersion: null,
          releaseNotes: null,
          pendingUpdate: null,
          lastCheckedAt: checkedAt,
        });
        return;
      }

      set({
        phase: "available",
        availableVersion: update.version,
        releaseNotes: update.body ?? null,
        pendingUpdate: update,
        lastCheckedAt: checkedAt,
      });
    } catch (err) {
      set({
        phase: "error",
        error: err instanceof Error ? err.message : "检查更新失败",
        pendingUpdate: null,
      });
    }
  },

  installUpdate: async () => {
    const { pendingUpdate, phase } = get();
    if (!isUpdaterEnabled() || !pendingUpdate) return;
    if (phase === "downloading" || phase === "installing") return;

    set({ phase: "downloading", error: null, progress: 0 });

    try {
      let downloaded = 0;
      let contentLength = 0;

      await pendingUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            set({ progress: contentLength > 0 ? 0 : null });
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              set({
                progress: Math.min(
                  100,
                  Math.round((downloaded / contentLength) * 100),
                ),
              });
            }
            break;
          case "Finished":
            set({ phase: "installing", progress: 100 });
            break;
        }
      });

      await relaunch();
    } catch (err) {
      set({
        phase: "error",
        error: err instanceof Error ? err.message : "安装更新失败",
        progress: null,
      });
    }
  },

  dismissUpdate: () => {
    const { availableVersion } = get();
    set({
      dismissedVersion: availableVersion,
      phase: "idle",
    });
  },

  clearError: () => {
    set({ phase: "idle", error: null, progress: null });
  },
}));

export function isAppUpdaterSupported(): boolean {
  return isUpdaterEnabled();
}
