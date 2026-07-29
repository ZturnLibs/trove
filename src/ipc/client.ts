import { invoke } from "@tauri-apps/api/core";

export type AppError = {
  code: string;
  message: string;
  fieldErrors?: Record<string, string>;
  retryable: boolean;
};

export type ThemePreference = "system" | "light" | "dark";

export type ShortcutSettings = {
  quickCapture: string;
  search: string;
  clipboard: string;
  focusMain: string;
};

export type AppSettings = {
  theme: ThemePreference;
  launchAtLogin: boolean;
  shortcuts: ShortcutSettings;
  clipboardCaptureEnabled: boolean;
};

export type DbHealth = {
  path: string;
  schemaVersion: number;
  userVersion: number;
  journalMode: string;
  fts5Available: boolean;
};

export type CapabilityStatus = {
  available: boolean;
  notes: string;
};

export type PlatformCapabilities = {
  notifications: CapabilityStatus;
  globalShortcuts: CapabilityStatus;
  clipboardRead: CapabilityStatus;
  directPaste: CapabilityStatus;
  autostart: CapabilityStatus;
  tray: CapabilityStatus;
};

export type AppHealth = {
  ok: boolean;
  appVersion: string;
  database: DbHealth;
  capabilities: PlatformCapabilities;
};

export type SmokeNote = {
  id: string;
  body: string;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export const ipc = {
  appHealth: () => invoke<AppHealth>("app_health"),
  settingsGet: () => invoke<AppSettings>("settings_get"),
  settingsSave: (settings: AppSettings) =>
    invoke<AppSettings>("settings_save", { settings }),
  smokeNoteCreate: (body: string) =>
    invoke<SmokeNote>("smoke_note_create", { body }),
  smokeNoteList: () => invoke<SmokeNote[]>("smoke_note_list"),
  smokeNoteDelete: (id: string) => invoke<void>("smoke_note_delete", { id }),
  windowShowMain: () => invoke<void>("window_show_main"),
  windowShowQuick: (mode?: "capture" | "search" | "clip") =>
    invoke<void>("window_show_quick", { mode }),
  appQuit: () => invoke<void>("app_quit"),
};
