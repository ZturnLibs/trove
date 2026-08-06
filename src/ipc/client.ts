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
  autoBackupOnLaunch: boolean;
  backupRetentionCount: number;
  onboardingCompleted: boolean;
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
  ocr: CapabilityStatus;
};

export type AppHealth = {
  ok: boolean;
  appVersion: string;
  database: DbHealth;
  capabilities: PlatformCapabilities;
  backup: BackupStatus;
};

export type BackupInfo = {
  fileName: string;
  path: string;
  sizeBytes: number;
  createdAt: string;
  reason: string;
};

export type BackupStatus = {
  directory: string;
  count: number;
  latest: BackupInfo | null;
  lastError: string | null;
};

export type ImportResult = {
  tables: number;
  rows: number;
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
  dueFrom?: string;
  dueTo?: string;
  dueNull?: boolean;
  completedSince?: string;
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

export type UpdateReminderInput = {
  id: string;
  title: string;
  notes: string;
  fireAt: string;
  recurrence?: RecurrenceRule | null;
  enabled: boolean;
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
  quickInsert: boolean;
  triggerWord: string | null;
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
  quickInsert?: boolean;
  triggerWord?: string | null;
  tagNames?: string[];
};

export type UpdateMemoryInput = {
  id: string;
  title: string;
  body: string;
  pinned: boolean;
  archived: boolean;
  quickInsert: boolean;
  triggerWord: string | null;
  tagNames: string[];
};

export type MemoryQuery = {
  pinnedOnly?: boolean;
  includeArchived?: boolean;
  tagId?: string;
  quickInsertOnly?: boolean;
  search?: string;
};

export type SmartListKind =
  | "tomorrow"
  | "next7Days"
  | "overdue"
  | "highPriority"
  | "noDue"
  | "recentCompleted";

export type ParsedCapture = {
  title: string;
  dueDate: string | null;
  dueTime: string | null;
  priority: TaskPriority;
  recurrence: RecurrenceRule | null;
  ambiguousFields: string[];
  raw: string;
};

export type TemplateKind = "task" | "reminder" | "memory";

export type ItemTemplate = {
  id: string;
  kind: TemplateKind;
  name: string;
  payload: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type TemplatePreview = {
  kind: TemplateKind;
  title: string;
  body: string;
  dueDate: string | null;
  dueTime: string | null;
  fireAt: string | null;
  priority: TaskPriority | null;
  recurrence: RecurrenceRule | null;
  tagNames: string[];
};

export type SavedView = {
  id: string;
  name: string;
  filter: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
  revision: number;
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

export type ClipboardKind = "text" | "image";

export type ClipboardItem = {
  id: string;
  kind: ClipboardKind;
  content: string;
  contentHash: string;
  assetId: string | null;
  sourceApp: string | null;
  favorite: boolean;
  useCount: number;
  lastUsedAt: string | null;
  width: number | null;
  height: number | null;
  thumbBase64: string | null;
  ocrText: string | null;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type ClipboardQuery = {
  favoritesOnly?: boolean;
  search?: string;
  limit?: number;
  kind?: ClipboardKind;
};

export type ConvertMemoryToTaskResult = {
  memory: Memory;
  taskId: string;
};

export type LinkEntityType =
  | "task"
  | "reminder"
  | "memory"
  | "clipboard"
  | "asset";

export type EntityLink = {
  id: string;
  sourceType: string;
  sourceId: string;
  targetType: string;
  targetId: string;
  linkKind: string;
  createdAt: string;
};

export type LinkInput = {
  sourceType: LinkEntityType;
  sourceId: string;
  targetType: LinkEntityType;
  targetId: string;
  linkKind: "attachment" | "converted_to";
};

export type LinkedAsset = {
  linkId: string;
  assetId: string;
  contentHash: string;
  byteSize: number;
  width: number | null;
  height: number | null;
  thumbBase64: string | null;
  createdAt: string;
};

export const ipc = {
  appHealth: () => invoke<AppHealth>("app_health"),
  settingsGet: () => invoke<AppSettings>("settings_get"),
  settingsSave: (settings: AppSettings) =>
    invoke<AppSettings>("settings_save", { settings }),
  settingsResetShortcuts: () =>
    invoke<AppSettings>("settings_reset_shortcuts"),
  shortcutsApply: () =>
    invoke<{ ok: boolean; errors: string[] }>("shortcuts_apply"),
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
  taskUnarchive: (id: string) => invoke<Task>("task_unarchive", { id }),
  taskDelete: (id: string) => invoke<void>("task_delete", { id }),
  taskSkip: (id: string) => invoke<Task>("task_skip", { id }),
  taskReorder: (orderedIds: string[]) =>
    invoke<void>("task_reorder", { orderedIds }),
  taskListTags: () => invoke<Tag[]>("task_list_tags"),
  taskCounts: () => invoke<TaskCounts>("task_counts"),
  taskSmartList: (kind: SmartListKind) =>
    invoke<Task[]>("task_smart_list", { kind }),
  taskPostpone: (id: string, days = 1) =>
    invoke<Task>("task_postpone", { id, days }),
  nlParseCapture: (text: string) =>
    invoke<ParsedCapture>("nl_parse_capture", { text }),
  templateList: () => invoke<ItemTemplate[]>("template_list"),
  templateCreate: (input: {
    kind: TemplateKind;
    name: string;
    payload: Record<string, unknown>;
  }) => invoke<ItemTemplate>("template_create", { input }),
  templateDelete: (id: string) => invoke<void>("template_delete", { id }),
  templatePreview: (id: string) =>
    invoke<TemplatePreview>("template_preview", { id }),
  templateApply: (id: string) =>
    invoke<{ kind: string; id: string }>("template_apply", { id }),
  savedViewCreate: (input: {
    name: string;
    filter: Record<string, unknown>;
  }) => invoke<SavedView>("saved_view_create", { input }),
  savedViewList: () => invoke<SavedView[]>("saved_view_list"),
  savedViewDelete: (id: string) => invoke<void>("saved_view_delete", { id }),
  reminderCreate: (input: CreateReminderInput) =>
    invoke<Reminder>("reminder_create", { input }),
  reminderUpdate: (input: UpdateReminderInput) =>
    invoke<Reminder>("reminder_update", { input }),
  reminderListAll: () => invoke<Reminder[]>("reminder_list_all"),
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
  entityLinkCreate: (input: LinkInput) =>
    invoke<EntityLink>("entity_link_create", { input }),
  entityLinkRemove: (id: string) =>
    invoke<void>("entity_link_remove", { id }),
  entityLinkList: (entityType: string, entityId: string) =>
    invoke<EntityLink[]>("entity_link_list", { entityType, entityId }),
  entityLinkAssets: (entityType: string, entityId: string) =>
    invoke<LinkedAsset[]>("entity_link_assets", { entityType, entityId }),
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
  assetReadThumb: (id: string) =>
    invoke<string | null>("asset_read_thumb", { id }),
  clipboardDelete: (id: string) => invoke<void>("clipboard_delete", { id }),
  clipboardClearNonFavorites: () =>
    invoke<number>("clipboard_clear_non_favorites"),
  clipboardConvertToTask: (id: string) =>
    invoke<string>("clipboard_convert_to_task", { id }),
  clipboardConvertToMemory: (id: string) =>
    invoke<string>("clipboard_convert_to_memory", { id }),
  clipboardSetCaptureEnabled: (enabled: boolean) =>
    invoke<AppSettings>("clipboard_set_capture_enabled", { enabled }),
  backupCreate: () => invoke<BackupInfo>("backup_create"),
  backupList: () => invoke<BackupInfo[]>("backup_list"),
  backupStatus: () => invoke<BackupStatus>("backup_status"),
  backupRestore: (fileName: string) =>
    invoke<void>("backup_restore", { fileName }),
  dataExport: () => invoke<string>("data_export"),
  dataImport: (json: string) => invoke<ImportResult>("data_import", { json }),
  windowShowMain: () => invoke<void>("window_show_main"),
  windowShowQuick: (mode?: "capture" | "search" | "clip") =>
    invoke<void>("window_show_quick", { mode }),
  windowHideQuick: () => invoke<void>("window_hide_quick"),
  appQuit: () => invoke<void>("app_quit"),
};
