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
  screenshotRegion: string;
};

export type AppSettings = {
  theme: ThemePreference;
  launchAtLogin: boolean;
  shortcuts: ShortcutSettings;
  clipboardCaptureEnabled: boolean;
  clipboardRetentionDays: number;
  clipboardMaxItems: number;
  clipboardExcludedApps: string[];
  clipboardSmartActionsEnabled: boolean;
  todaySmartSortEnabled: boolean;
  autoBackupOnLaunch: boolean;
  autoCheckUpdates: boolean;
  backupRetentionCount: number;
  onboardingCompleted: boolean;
  lastFocusCarryDismissedDate?: string | null;
  automationEnabled: boolean;
  ai: AIConfig;
};

export type AIMode = "off" | "ollama" | "custom";

export type AIFeature = "extract" | "related" | "summary" | "suggest" | "split";

export type AIFeatureToggles = {
  extract: boolean;
  related: boolean;
  summary: boolean;
  suggest: boolean;
  split: boolean;
};

export type AIConfig = {
  mode: AIMode;
  ollamaUrl: string;
  ollamaModel: string;
  customEndpoint: string;
  customModel: string;
  features: AIFeatureToggles;
};

export type ProbeReport = {
  mode: AIMode;
  reachable: boolean;
  model: string | null;
  latencyMs: number | null;
  hint: string | null;
};

export type SuggestedItem = {
  title: string;
  detail: string | null;
  dueDate: string | null;
  dueTime: string | null;
  ambiguous: boolean;
  sourceExcerpt: string;
};

export type AISuggestionRecord = {
  id: string;
  featureType: string;
  sourceEntityType: string;
  sourceEntityId: string;
  payload: { items: SuggestedItem[]; summary: string | null };
  sources: { entityType: string; entityId: string; textOffset: number; excerpt: string }[];
  status: "pending" | "accepted" | "rejected" | "dismissed";
  provider: string;
  model: string;
  createdAt: string;
  decidedAt: string | null;
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

export type CsvFieldMapping = {
  title?: string | null;
  notes?: string | null;
  status?: string | null;
  priority?: string | null;
  list?: string | null;
  dueDate?: string | null;
  dueTime?: string | null;
  tags?: string | null;
};

export type CsvRowIssue = {
  row: number;
  title?: string | null;
  message: string;
};

export type CsvSampleRow = {
  title: string;
  list?: string | null;
  dueDate?: string | null;
  priority: string;
  duplicate: boolean;
};

export type CsvPreview = {
  headers: string[];
  mapping: CsvFieldMapping;
  rowCount: number;
  validCount: number;
  duplicateCount: number;
  errorCount: number;
  errors: CsvRowIssue[];
  duplicates: CsvRowIssue[];
  unmappedLists: string[];
  sample: CsvSampleRow[];
};

export type CsvImportResult = {
  batchId: string;
  created: number;
  skipped: number;
};

export type ImportBatch = {
  id: string;
  source: string;
  created: number;
  skipped: number;
  status: string;
  createdAt: string;
  undoneAt?: string | null;
};

export type CsvUndoResult = {
  deleted: number;
  kept: number;
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

export type ListDeleteDisposition =
  | "moveToInbox"
  | "archiveTasks"
  | "forceDelete";

export type DeleteListResult = {
  listId: string;
  listName: string;
  disposition: ListDeleteDisposition;
  taskIds: string[];
  archivedTaskIds: string[];
};

export type Tag = {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type TaskWorkflowState = "active" | "waiting";

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
  workflowState: TaskWorkflowState;
  availableAt: string | null;
  waitingFor: string | null;
  followUpDate: string | null;
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
  search?: string;
  workflowState?: TaskWorkflowState;
  deferredOnly?: boolean;
  waitingFollowUpDue?: boolean;
  limit?: number;
  offset?: number;
};

export type PagedResult<T> = {
  items: T[];
  total: number;
  hasMore: boolean;
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
  focus: Task[];
  waitingFollowUp: Task[];
  focusCarrySuggestions: Task[];
  remindersToday: TodayReminderItem[];
  today: string;
};

export type TodaySortSuggestion = {
  taskId: string;
  rank: number;
  reason: string;
};

export type TodaySortSuggestions = {
  enabled: boolean;
  suggestions: TodaySortSuggestion[];
};

export type FocusOutcome = "inProgress" | "completed" | "keptTodo" | "abandoned";

export type FocusSession = {
  id: string;
  taskId: string;
  startedAt: string;
  endedAt: string | null;
  plannedMinutes: number | null;
  outcome: FocusOutcome;
  progressNote: string | null;
  createdAt: string;
  updatedAt: string;
};

export type DailyWrapRun = {
  id: string;
  wrapDate: string;
  startedAt: string;
  completedAt: string | null;
  stepsCompleted: number;
  summary: Record<string, unknown> | null;
  createdAt: string;
};

export type DailyWrapSnapshot = {
  wrapDate: string;
  unfinishedFocus: Task[];
  tomorrowDue: Task[];
  inboxUnprocessed: Task[];
  completedTodayCount: number;
  remindersTodayCount: number;
};

export type DailyWrapCompleteInput = {
  stepsCompleted: number;
  summary?: Record<string, unknown> | null;
};

export type ReviewSession = {
  id: string;
  reviewType: "weekly";
  startedAt: string;
  completedAt: string | null;
  summary: Record<string, unknown> | null;
  createdAt: string;
};

export type WeeklyReviewSnapshot = {
  inboxUnprocessed: Task[];
  inboxCount: number;
  overdue: Task[];
  overdueCount: number;
  waitingFollowUp: Task[];
  waitingFollowUpCount: number;
  staleActive: Task[];
  staleActiveCount: number;
  completedLast7Days: Task[];
  completedLast7DaysCount: number;
  upcomingRecurringReminders: Reminder[];
  upcomingRecurringCount: number;
  largeClipboardItems: ClipboardItem[];
  largeClipboardCount: number;
};

export type ReviewCompleteInput = {
  summary?: Record<string, unknown> | null;
};

export type HealthBackupSummary = {
  directory: string;
  count: number;
  latestCreatedAt: string | null;
  lastError: string | null;
};

export type StorageBreakdown = {
  databaseBytes: number;
  walBytes: number;
  assetsBytes: number;
  thumbBytes: number;
  assetsRoot: string;
  note: string;
};

export type StorageGcPreview = {
  candidateCount: number;
  candidateBytes: number;
  retentionDays: number;
  note: string;
};

export type AssetsGcSummary = {
  removed: number;
  freedBytes: number;
};

export type FileReference = {
  id: string;
  displayName: string;
  pathHint: string;
  mimeType: string | null;
  byteSize: number | null;
  accessible: boolean;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type LinkedFileReference = {
  linkId: string;
  file: FileReference;
};

export type ReminderOutcomeStats = {
  onTime: number;
  snoozed: number;
  missed: number;
  pendingOverdue: number;
};

export type DailyCompletionCount = {
  date: string;
  count: number;
};

export type TaskHealthStats = {
  inboxCount: number;
  inboxOldestDays: number | null;
  staleActiveCount: number;
  completionTrend: DailyCompletionCount[];
};

export type ClipboardHealthStats = {
  totalCount: number;
  favoriteCount: number;
  maxItems: number;
  retentionDays: number;
  remainingSlots: number;
};

export type HealthDashboardSnapshot = {
  backup: HealthBackupSummary;
  backupTotalBytes: number;
  storage: StorageBreakdown;
  storageGc: StorageGcPreview;
  reminders7d: ReminderOutcomeStats;
  reminders30d: ReminderOutcomeStats;
  tasks: TaskHealthStats;
  clipboard: ClipboardHealthStats;
  generatedAt: string;
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
  sensitive: boolean;
  mentionUseCount: number;
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
  sensitive: boolean;
  tagNames: string[];
};

export type MemoryQuery = {
  pinnedOnly?: boolean;
  includeArchived?: boolean;
  tagId?: string;
  quickInsertOnly?: boolean;
  search?: string;
  limit?: number;
  offset?: number;
};

export type MemorySummary = {
  id: string;
  title: string;
};

export type WikilinkPendingReason = "missing" | "ambiguous";

export type WikilinkPending = {
  title: string;
  reason: WikilinkPendingReason;
  candidates: MemorySummary[];
};

export type WikilinkResolutionAction = "link" | "create" | "skip";

export type WikilinkResolution = {
  title: string;
  action: WikilinkResolutionAction;
  targetId?: string | null;
};

export type WikilinkSyncResult = {
  memory: Memory;
  linkedIds: string[];
  pending: WikilinkPending[];
};

export type MemoryBacklink = {
  memoryId: string;
  title: string;
};

export type RelatedMemoryHit = {
  memoryId: string;
  title: string;
  score: number;
  reasons: string[];
};

export type SmartListKind =
  | "tomorrow"
  | "next7Days"
  | "overdue"
  | "highPriority"
  | "noDue"
  | "recentCompleted"
  | "deferred"
  | "waitingFollowUp";

export type ParsedCapture = {
  title: string;
  dueDate: string | null;
  dueTime: string | null;
  priority: TaskPriority;
  recurrence: RecurrenceRule | null;
  ambiguousFields: string[];
  tagNames: string[];
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

export type ClipboardKindHint =
  | "plain"
  | "url"
  | "email"
  | "phone"
  | "date"
  | "code"
  | "error";

export type ClipboardItem = {
  id: string;
  kind: ClipboardKind;
  kindHint: ClipboardKindHint;
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
  offset?: number;
  kind?: ClipboardKind;
  kindHint?: ClipboardKindHint;
  sourceApp?: string;
  dateFrom?: string;
  dateTo?: string;
};

export type ClipboardTaskDraftInput = {
  title?: string | null;
  notes?: string | null;
  dueDate?: string | null;
  dueTime?: string | null;
  priority?: TaskPriority | null;
};

export type SimilarTaskHit = {
  taskId: string;
  title: string;
  score: number;
};

export type ClipboardSmartContext = {
  kindHint: ClipboardKindHint;
  taskDraft: ParsedCapture | null;
  similarTasks: SimilarTaskHit[];
  linkedTaskId: string | null;
  linkedMemoryId: string | null;
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

export type AutomationEntityType = "task" | "reminder" | "memory" | "clipboard";

export type AutomationEventKind =
  | "taskCreated"
  | "reminderCreated"
  | "memoryCreated"
  | "clipboardFavorited"
  | "reminderFired"
  | "taskMovedToList"
  | "taskTagAdded";

export type AutomationTrigger =
  | { kind: "taskCreated" }
  | { kind: "reminderCreated" }
  | { kind: "memoryCreated" }
  | { kind: "clipboardFavorited" }
  | { kind: "reminderFired" }
  | { kind: "taskMovedToList"; listId?: string | null }
  | { kind: "taskTagAdded"; tagName?: string | null };

export type AutomationCondition =
  | { kind: "titleContains"; text: string; caseInsensitive?: boolean }
  | { kind: "bodyContains"; text: string; caseInsensitive?: boolean }
  | { kind: "entityType"; entityType: AutomationEntityType }
  | { kind: "listId"; listId: string }
  | { kind: "hasTag"; tagName: string }
  | { kind: "priority"; priority: TaskPriority }
  | { kind: "sourceApp"; app: string }
  | { kind: "weekday"; days: number[] }
  | { kind: "timeRange"; start: string; end: string };

export type AutomationAction =
  | { kind: "setPriority"; priority: TaskPriority }
  | { kind: "moveToList"; listId: string }
  | { kind: "addTag"; tagName: string }
  | { kind: "pinMemory" }
  | { kind: "notify"; title: string; body: string };

export type AutomationRuleDefinition = {
  trigger: AutomationTrigger;
  conditions: AutomationCondition[];
  actions: AutomationAction[];
};

export type AutomationRule = {
  id: string;
  name: string;
  enabled: boolean;
  definition: AutomationRuleDefinition;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type AutomationRunStatus = "success" | "skipped" | "failed" | "dryRun";

export type AutomationRun = {
  id: string;
  ruleId: string;
  ruleName: string;
  entityType: AutomationEntityType;
  entityId: string;
  status: AutomationRunStatus;
  actionsApplied: AutomationAction[];
  errorSummary?: string | null;
  dryRun: boolean;
  createdAt: string;
};

export type AutomationDryRunResult = {
  ruleId: string;
  ruleName: string;
  matched: boolean;
  actions: AutomationAction[];
  skipReason?: string | null;
};

export type AutomationEvent = {
  kind: AutomationEventKind;
  entityType: AutomationEntityType;
  entityId: string;
  title: string;
  body: string;
  listId?: string | null;
  tagNames: string[];
  priority?: TaskPriority | null;
  sourceApp?: string | null;
  addedTag?: string | null;
  targetListId?: string | null;
};

export type CreateAutomationRuleInput = {
  name: string;
  definition: AutomationRuleDefinition;
};

export type UpdateAutomationRuleInput = {
  id: string;
  name: string;
  enabled: boolean;
  definition: AutomationRuleDefinition;
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
  taskListUpdate: (id: string, name: string) =>
    invoke<TaskList>("task_list_update", { id, name }),
  taskListTodoCount: (id: string) => invoke<number>("task_list_todo_count", { id }),
  taskListDelete: (id: string, disposition: ListDeleteDisposition) =>
    invoke<DeleteListResult>("task_list_delete", { id, disposition }),
  taskListUndoDelete: (result: DeleteListResult) =>
    invoke<TaskList>("task_list_undo_delete", { result }),
  taskCreate: (input: CreateTaskInput) => invoke<Task>("task_create", { input }),
  taskCreateRecurring: (input: CreateTaskInput, recurrence: RecurrenceRule) =>
    invoke<Task>("task_create_recurring", { input, recurrence }),
  taskUpdate: (input: UpdateTaskInput) => invoke<Task>("task_update", { input }),
  taskGet: (id: string) => invoke<Task>("task_get", { id }),
  taskQuery: (query: TaskQuery = {}) =>
    invoke<PagedResult<Task>>("task_query", { query }),
  taskToday: () => invoke<TodayTasks>("task_today"),
  todaySortSuggestions: () =>
    invoke<TodaySortSuggestions>("today_sort_suggestions"),
  todaySetSmartSortEnabled: (enabled: boolean) =>
    invoke<AppSettings>("today_set_smart_sort_enabled", { enabled }),
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
  taskSmartList: (kind: SmartListKind, limit?: number, offset?: number) =>
    invoke<PagedResult<Task>>("task_smart_list", { kind, limit, offset }),
  taskPostpone: (id: string, days = 1) =>
    invoke<Task>("task_postpone", { id, days }),
  taskSetDefer: (id: string, availableAt: string | null) =>
    invoke<Task>("task_set_defer", { id, availableAt }),
  taskSetWaiting: (
    id: string,
    waitingFor: string | null,
    followUpDate: string | null,
  ) => invoke<Task>("task_set_waiting", { id, waitingFor, followUpDate }),
  taskClearWaiting: (id: string) => invoke<Task>("task_clear_waiting", { id }),
  dailyFocusAdd: (taskId: string, focusDate?: string | null) =>
    invoke<Task>("daily_focus_add", { taskId, focusDate }),
  dailyFocusRemove: (taskId: string, focusDate?: string | null) =>
    invoke<Task>("daily_focus_remove", { taskId, focusDate }),
  dailyFocusReorder: (taskIds: string[], focusDate?: string | null) =>
    invoke<void>("daily_focus_reorder", { taskIds, focusDate }),
  dailyFocusCarry: (fromDate: string, toDate: string) =>
    invoke<Task[]>("daily_focus_carry", { fromDate, toDate }),
  focusStart: (taskId: string, plannedMinutes?: number | null) =>
    invoke<FocusSession>("focus_start", { taskId, plannedMinutes }),
  focusEnd: (
    sessionId: string,
    outcome: Exclude<FocusOutcome, "inProgress">,
    progressNote?: string | null,
  ) => invoke<FocusSession>("focus_end", { sessionId, outcome, progressNote }),
  focusActive: () => invoke<FocusSession | null>("focus_active"),
  focusList: (taskId?: string, limit?: number) =>
    invoke<FocusSession[]>("focus_list", { taskId, limit }),
  dailyWrapSnapshot: (wrapDate?: string | null) =>
    invoke<DailyWrapSnapshot>("daily_wrap_snapshot", { wrapDate }),
  dailyWrapStart: (wrapDate?: string | null) =>
    invoke<DailyWrapRun>("daily_wrap_start", { wrapDate }),
  dailyWrapComplete: (runId: string, input: DailyWrapCompleteInput) =>
    invoke<DailyWrapRun>("daily_wrap_complete", { runId, input }),
  dailyWrapCompletedForDate: (wrapDate?: string | null) =>
    invoke<DailyWrapRun | null>("daily_wrap_completed_for_date", { wrapDate }),
  weeklyReviewSnapshot: () =>
    invoke<WeeklyReviewSnapshot>("weekly_review_snapshot"),
  weeklyReviewStart: () => invoke<ReviewSession>("weekly_review_start"),
  weeklyReviewComplete: (sessionId: string, input: ReviewCompleteInput) =>
    invoke<ReviewSession>("weekly_review_complete", { sessionId, input }),
  weeklyReviewLastCompleted: () =>
    invoke<ReviewSession | null>("weekly_review_last_completed"),
  healthDashboardSnapshot: () =>
    invoke<HealthDashboardSnapshot>("health_dashboard_snapshot"),
  storageRunAssetsGc: () => invoke<AssetsGcSummary>("storage_run_assets_gc"),
  captureRegionScreenshot: () =>
    invoke<ClipboardItem | null>("capture_region_screenshot"),
  fileRefPickAndAttach: (sourceType: "task" | "memory", sourceId: string) =>
    invoke<LinkedFileReference | null>("file_ref_pick_and_attach", {
      sourceType,
      sourceId,
    }),
  fileRefListForEntity: (sourceType: "task" | "memory", sourceId: string) =>
    invoke<LinkedFileReference[]>("file_ref_list_for_entity", {
      sourceType,
      sourceId,
    }),
  fileRefOpen: (id: string) => invoke<void>("file_ref_open", { id }),
  fileRefReveal: (id: string) => invoke<void>("file_ref_reveal", { id }),
  fileRefRelink: (id: string) =>
    invoke<FileReference | null>("file_ref_relink", { id }),
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
    invoke<PagedResult<Memory>>("memory_query", { query }),
  memoryDelete: (id: string) => invoke<void>("memory_delete", { id }),
  memoryConvertToTask: (id: string) =>
    invoke<ConvertMemoryToTaskResult>("memory_convert_to_task", { id }),
  memoryWikilinkPending: (id: string) =>
    invoke<WikilinkPending[]>("memory_wikilink_pending", { id }),
  memoryResolveWikilinks: (id: string, resolutions: WikilinkResolution[]) =>
    invoke<WikilinkSyncResult>("memory_resolve_wikilinks", { id, resolutions }),
  memoryBacklinks: (id: string) =>
    invoke<MemoryBacklink[]>("memory_backlinks", { id }),
  memoryRelated: (id: string) =>
    invoke<RelatedMemoryHit[]>("memory_related", { id }),
  memoryLinkMention: (sourceId: string, targetId: string) =>
    invoke<void>("memory_link_mention", { sourceId, targetId }),
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
    invoke<PagedResult<ClipboardItem>>("clipboard_query", { query }),
  clipboardListSourceApps: () => invoke<string[]>("clipboard_list_source_apps"),
  clipboardGet: (id: string) => invoke<ClipboardItem>("clipboard_get", { id }),
  clipboardSetFavorite: (id: string, favorite: boolean) =>
    invoke<ClipboardItem>("clipboard_set_favorite", { id, favorite }),
  clipboardCopy: (id: string) => invoke<ClipboardItem>("clipboard_copy", { id }),
  assetReadThumb: (id: string) =>
    invoke<string | null>("asset_read_thumb", { id }),
  clipboardDelete: (id: string) => invoke<void>("clipboard_delete", { id }),
  clipboardClearNonFavorites: () =>
    invoke<number>("clipboard_clear_non_favorites"),
  clipboardConvertToTask: (id: string, draft?: ClipboardTaskDraftInput | null) =>
    invoke<string>("clipboard_convert_to_task", { id, draft: draft ?? null }),
  clipboardConvertToMemory: (id: string) =>
    invoke<string>("clipboard_convert_to_memory", { id }),
  clipboardSmartContext: (id: string) =>
    invoke<ClipboardSmartContext>("clipboard_smart_context", { id }),
  clipboardLinkToTask: (clipboardId: string, taskId: string) =>
    invoke<void>("clipboard_link_to_task", { clipboardId, taskId }),
  clipboardSetCaptureEnabled: (enabled: boolean) =>
    invoke<AppSettings>("clipboard_set_capture_enabled", { enabled }),
  clipboardSetSmartActionsEnabled: (enabled: boolean) =>
    invoke<AppSettings>("clipboard_set_smart_actions_enabled", { enabled }),
  backupCreate: () => invoke<BackupInfo>("backup_create"),
  backupList: () => invoke<BackupInfo[]>("backup_list"),
  backupStatus: () => invoke<BackupStatus>("backup_status"),
  backupRestore: (fileName: string) =>
    invoke<void>("backup_restore", { fileName }),
  dataExport: () => invoke<string>("data_export"),
  dataImport: (json: string) => invoke<ImportResult>("data_import", { json }),
  csvExportTasks: () => invoke<string>("csv_export_tasks"),
  csvPreviewTasks: (csv: string, mapping?: CsvFieldMapping | null) =>
    invoke<CsvPreview>("csv_preview_tasks", { csv, mapping: mapping ?? null }),
  csvImportTasks: (input: {
    csv: string;
    skipDuplicates?: boolean;
    mapping?: CsvFieldMapping | null;
  }) => invoke<CsvImportResult>("csv_import_tasks", { input }),
  csvImportBatches: () => invoke<ImportBatch[]>("csv_import_batches"),
  csvUndoImport: (id: string) => invoke<CsvUndoResult>("csv_undo_import", { id }),
  windowShowMain: () => invoke<void>("window_show_main"),
  windowShowQuick: (mode?: "capture" | "search" | "clip") =>
    invoke<void>("window_show_quick", { mode }),
  windowHideQuick: () => invoke<void>("window_hide_quick"),
  urlSchemeHandle: (url: string) => invoke<void>("url_scheme_handle", { url }),
  automationList: () => invoke<AutomationRule[]>("automation_list"),
  automationCreate: (input: CreateAutomationRuleInput) =>
    invoke<AutomationRule>("automation_create", { input }),
  automationUpdate: (input: UpdateAutomationRuleInput) =>
    invoke<AutomationRule>("automation_update", { input }),
  automationDelete: (id: string) => invoke<void>("automation_delete", { id }),
  automationSetEnabled: (id: string, enabled: boolean) =>
    invoke<AutomationRule>("automation_set_enabled", { id, enabled }),
  automationRunsList: (ruleId: string | null, limit?: number) =>
    invoke<AutomationRun[]>("automation_runs_list", {
      ruleId,
      limit: limit ?? null,
    }),
  automationDryRun: (ruleId: string, event: AutomationEvent) =>
    invoke<AutomationDryRunResult>("automation_dry_run", { ruleId, event }),
  aiProviderKeyStatus: () =>
    invoke<{ exists: boolean }>("ai_provider_key_status"),
  aiProviderKeySet: (key: string) =>
    invoke<{ exists: boolean }>("ai_provider_key_set", { key }),
  aiProviderKeyClear: () =>
    invoke<{ exists: boolean }>("ai_provider_key_clear"),
  aiProviderProbe: () => invoke<ProbeReport>("ai_provider_probe"),
  aiSuggestionList: (
    feature?: AIFeature | null,
    status?: AISuggestionRecord["status"] | null,
  ) =>
    invoke<AISuggestionRecord[]>("ai_suggestion_list", {
      feature: feature ?? null,
      status: status ?? null,
    }),
  aiSuggestionDecide: (
    id: string,
    decision: "accept" | "reject" | "dismiss",
  ) =>
    invoke<AISuggestionRecord>("ai_suggestion_decide", { id, decision }),
  aiSuggestionClear: () => invoke<number>("ai_suggestion_clear"),
  aiExtractRequest: (memoryId: string) =>
    invoke<AISuggestionRecord | null>("ai_extract_request", { memoryId }),
  aiSuggestionApply: (suggestionId: string, selectedIndices: number[]) =>
    invoke<{ tasks: Task[]; suggestion: AISuggestionRecord }>("ai_suggestion_apply", {
      input: { suggestionId, selectedIndices },
    }),
  aiWeeklySummaryRequest: () =>
    invoke<AISuggestionRecord | null>("ai_weekly_summary_request"),
  aiRelatedRequest: (taskId: string) =>
    invoke<AISuggestionRecord | null>("ai_related_request", { taskId }),
  aiRelatedConfirm: (suggestionId: string, selectedIndices: number[], taskId: string) =>
    invoke<EntityLink[]>("ai_related_confirm", {
      suggestionId,
      selectedIndices,
      taskId,
    }),
  aiRelatedRejectItem: (suggestionId: string, index: number) =>
    invoke<AISuggestionRecord>("ai_related_reject_item", { suggestionId, index }),
  aiDailySuggestRequest: () =>
    invoke<AISuggestionRecord | null>("ai_daily_suggest_request"),
  aiDailySuggestSkip: (suggestionId: string, index: number) =>
    invoke<AISuggestionRecord>("ai_daily_suggest_skip", { suggestionId, index }),
  aiDailySuggestAccept: (suggestionId: string, index: number) =>
    invoke<AISuggestionRecord>("ai_daily_suggest_accept", { suggestionId, index }),
  appQuit: () => invoke<void>("app_quit"),
};
