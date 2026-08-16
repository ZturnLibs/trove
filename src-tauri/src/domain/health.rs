use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthBackupSummary {
    pub directory: String,
    pub count: usize,
    pub latest_created_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthDashboardSnapshot {
    pub backup: HealthBackupSummary,
    pub backup_total_bytes: u64,
    pub storage: StorageBreakdown,
    pub storage_gc: StorageGcPreview,
    pub reminders_7d: ReminderOutcomeStats,
    pub reminders_30d: ReminderOutcomeStats,
    pub tasks: TaskHealthStats,
    pub clipboard: ClipboardHealthStats,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBreakdown {
    pub database_bytes: u64,
    pub wal_bytes: u64,
    pub assets_bytes: i64,
    pub thumb_bytes: u64,
    pub assets_root: String,
    /// 统计口径说明（非评价性结论）。
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageGcPreview {
    pub candidate_count: usize,
    pub candidate_bytes: i64,
    pub retention_days: u32,
    /// 与 assets.collect_garbage 相同规则：无引用且超过保留期的孤儿资源。
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderOutcomeStats {
    /// 按时完成（actioned 且未贪睡）。
    pub on_time: i64,
    /// 贪睡中或贪睡后完成。
    pub snoozed: i64,
    /// 推断错过。
    pub missed: i64,
    /// 窗口内仍逾期未完成。
    pub pending_overdue: i64,
}

impl ReminderOutcomeStats {
    pub fn resolved_total(&self) -> i64 {
        self.on_time + self.snoozed + self.missed + self.pending_overdue
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHealthStats {
    pub inbox_count: i64,
    pub inbox_oldest_days: Option<i64>,
    pub stale_active_count: i64,
    pub completion_trend: Vec<DailyCompletionCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyCompletionCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardHealthStats {
    pub total_count: i64,
    pub favorite_count: i64,
    pub max_items: u32,
    pub retention_days: u32,
    pub remaining_slots: i64,
}
