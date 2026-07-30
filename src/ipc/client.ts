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
  clipboardRetentionDays: number;
  clipboardMaxItems: number;
  clipboardExcludedApps: string[];
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

export type TaskStatus = "todo" | "completed" | "archived";
export type TaskPriority = "none" | "low" | "medium" | "high";
export type ListKind = "inbox" | "custom";

export type TaskList = {
  id: string;
  name: string;
  kind: ListKind;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type Tag = {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type Task = {
  id: string;
  title: string;
  notes: string;
  status: TaskStatus;
  priority: TaskPriority;
  listId: string;
  listName: string;
  listKind: ListKind;
  dueDate: string | null;
  dueTime: string | null;
  completedAt: string | null;
  sortOrder: number;
  seriesId: string | null;
  tagIds: string[];
  tagNames: string[];
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type CreateTaskInput = {
  title: string;
  notes?: string;
  priority?: TaskPriority;
  listId?: string;
  dueDate?: string | null;
  dueTime?: string | null;
  tagNames?: string[];
};

export type UpdateTaskInput = {
  id: string;
  title: string;
  notes: string;
  priority: TaskPriority;
  listId: string;
  dueDate: string | null;
  dueTime: string | null;
  tagNames: string[];
};

export type TaskQuery = {
  listId?: string;
  inboxOnly?: boolean;
  status?: TaskStatus;
  priority?: TaskPriority;
  tagId?: string;
  includeArchived?: boolean;
};

export type RecurrenceFrequency =
  | "daily"
  | "weekdays"
  | "weekly"
  | "monthly"
  | "everyNDays"
  | "everyNWeeks";

export type RecurrenceRule = {
  version: number;
  frequency: RecurrenceFrequency;
  interval: number;
  weekdays?: number[];
  monthday?: number;
  timezone: string;
  endAt?: string | null;
};

export type Reminder = {
  id: string;
  title: string;
  notes: string;
  taskId: string | null;
  recurrence: RecurrenceRule | null;
  timezone: string;
  nextFireAt: string;
  endAt: string | null;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type ReminderOccurrence = {
  id: string;
  reminderId: string;
  scheduledAt: string;
  status:
    | "pending"
    | "scheduled"
    | "actioned"
    | "snoozed"
    | "cancelled"
    | "inferredMissed";
  needsSchedule: boolean;
  systemNotificationId: number | null;
  actionedAt: string | null;
  snoozeUntil: string | null;
  title: string;
  taskId: string | null;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type TodayReminderItem = {
  occurrence: ReminderOccurrence;
  reminder: Reminder;
};

export type TodayTasks = {
  overdue: Task[];
  dueToday: Task[];
  completedToday: Task[];
  remindersToday: TodayReminderItem[];
  today: string;
};

export type CreateReminderInput = {
  title: string;
  notes?: string;
  taskId?: string;
  fireAt: string;
  recurrence?: RecurrenceRule | null;
  timezone?: string;
  endAt?: string | null;
};

export type SnoozePreset = "minutes10" | "hour1" | "tomorrow";

export type TaskCounts = {
  inbox: number;
  overdue: number;
};

export type Memory = {
  id: string;
  title: string;
  body: string;
  pinned: boolean;
  archived: boolean;
  tagIds: string[];
  tagNames: string[];
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type CreateMemoryInput = {
  title: string;
  body?: string;
  pinned?: boolean;
  tagNames?: string[];
};

export type UpdateMemoryInput = {
  id: string;
  title: string;
  body: string;
  pinned: boolean;
  archived: boolean;
  tagNames: string[];
};

export type MemoryQuery = {
  pinnedOnly?: boolean;
  includeArchived?: boolean;
  tagId?: string;
};

export type SearchEntityType = "task" | "reminder" | "memory" | "clipboard";

export type SearchHit = {
  entityType: SearchEntityType;
  entityId: string;
  title: string;
  snippet: string;
  updatedAt: string;
};

export type SearchResults = {
  tasks: SearchHit[];
  reminders: SearchHit[];
  memories: SearchHit[];
  clipboard: SearchHit[];
};

export type ClipboardItem = {
  id: string;
  content: string;
  contentHash: string;
  sourceApp: string | null;
  favorite: boolean;
  useCount: number;
  lastUsedAt: string | null;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type ClipboardQuery = {
  favoritesOnly?: boolean;
  search?: string;
  limit?: number;
};

export type ConvertMemoryToTaskResult = {
  memory: Memory;
  taskId: string;
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
  taskListLists: () => invoke<TaskList[]>("task_list_lists"),
  taskListCreate: (name: string) => invoke<TaskList>("task_list_create", { name }),
  taskCreate: (input: CreateTaskInput) => invoke<Task>("task_create", { input }),
  taskCreateRecurring: (input: CreateTaskInput, recurrence: RecurrenceRule) =>
    invoke<Task>("task_create_recurring", { input, recurrence }),
  taskUpdate: (input: UpdateTaskInput) => invoke<Task>("task_update", { input }),
  taskGet: (id: string) => invoke<Task>("task_get", { id }),
  taskQuery: (query: TaskQuery = {}) => invoke<Task[]>("task_query", { query }),
  taskToday: () => invoke<TodayTasks>("task_today"),
  taskComplete: (id: string) => invoke<Task>("task_complete", { id }),
  taskUncomplete: (id: string) => invoke<Task>("task_uncomplete", { id }),
  taskArchive: (id: string) => invoke<Task>("task_archive", { id }),
  taskDelete: (id: string) => invoke<void>("task_delete", { id }),
  taskSkip: (id: string) => invoke<Task>("task_skip", { id }),
  taskReorder: (orderedIds: string[]) =>
    invoke<void>("task_reorder", { orderedIds }),
  taskListTags: () => invoke<Tag[]>("task_list_tags"),
  taskCounts: () => invoke<TaskCounts>("task_counts"),
  reminderCreate: (input: CreateReminderInput) =>
    invoke<Reminder>("reminder_create", { input }),
  reminderDelete: (id: string) => invoke<void>("reminder_delete", { id }),
  reminderListForTask: (taskId: string) =>
    invoke<Reminder[]>("reminder_list_for_task", { taskId }),
  reminderComplete: (occurrenceId: string) =>
    invoke<ReminderOccurrence>("reminder_complete", { occurrenceId }),
  reminderSnooze: (occurrenceId: string, preset: SnoozePreset) =>
    invoke<ReminderOccurrence>("reminder_snooze", { occurrenceId, preset }),
  memoryCreate: (input: CreateMemoryInput) =>
    invoke<Memory>("memory_create", { input }),
  memoryUpdate: (input: UpdateMemoryInput) =>
    invoke<Memory>("memory_update", { input }),
  memoryGet: (id: string) => invoke<Memory>("memory_get", { id }),
  memoryQuery: (query: MemoryQuery = {}) =>
    invoke<Memory[]>("memory_query", { query }),
  memoryDelete: (id: string) => invoke<void>("memory_delete", { id }),
  memoryConvertToTask: (id: string) =>
    invoke<ConvertMemoryToTaskResult>("memory_convert_to_task", { id }),
  searchQuery: (query: string, types?: SearchEntityType[], limit?: number) =>
    invoke<SearchResults>("search_query", {
      query: { query, types, limit },
    }),
  clipboardQuery: (query: ClipboardQuery = {}) =>
    invoke<ClipboardItem[]>("clipboard_query", { query }),
  clipboardGet: (id: string) => invoke<ClipboardItem>("clipboard_get", { id }),
  clipboardSetFavorite: (id: string, favorite: boolean) =>
    invoke<ClipboardItem>("clipboard_set_favorite", { id, favorite }),
  clipboardCopy: (id: string) => invoke<ClipboardItem>("clipboard_copy", { id }),
  clipboardDelete: (id: string) => invoke<void>("clipboard_delete", { id }),
  clipboardClearNonFavorites: () =>
    invoke<number>("clipboard_clear_non_favorites"),
  clipboardConvertToTask: (id: string) =>
    invoke<string>("clipboard_convert_to_task", { id }),
  clipboardConvertToMemory: (id: string) =>
    invoke<string>("clipboard_convert_to_memory", { id }),
  clipboardSetCaptureEnabled: (enabled: boolean) =>
    invoke<AppSettings>("clipboard_set_capture_enabled", { enabled }),
  windowShowMain: () => invoke<void>("window_show_main"),
  windowShowQuick: (mode?: "capture" | "search" | "clip") =>
    invoke<void>("window_show_quick", { mode }),
  windowHideQuick: () => invoke<void>("window_hide_quick"),
  appQuit: () => invoke<void>("app_quit"),
};
