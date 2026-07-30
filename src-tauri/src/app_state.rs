use crate::application::backup::BackupService;
use crate::application::clipboard::ClipboardService;
use crate::application::data_port::DataPortService;
use crate::application::memories::MemoryService;
use crate::application::reminders::ReminderService;
use crate::application::search::SearchService;
use crate::application::smoke_notes::SmokeNoteService;
use crate::application::tasks::TaskService;
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub settings: Arc<SettingsService>,
    pub smoke_notes: Arc<SmokeNoteService>,
    pub tasks: Arc<TaskService>,
    pub reminders: Arc<ReminderService>,
    pub memories: Arc<MemoryService>,
    pub search: Arc<SearchService>,
    pub clipboard: Arc<ClipboardService>,
    pub backups: Arc<BackupService>,
    pub data_port: Arc<DataPortService>,
}

impl AppState {
    pub fn new(db: Database, backup_dir: PathBuf) -> Result<Self, String> {
        let db = Arc::new(db);
        let tasks = TaskService::new(db.as_ref().clone());
        tasks
            .ensure_seed_data()
            .map_err(|e| format!("seed task data: {e}"))?;
        let reminders = ReminderService::new(db.as_ref().clone());
        let search = SearchService::new(db.as_ref().clone());
        let memories = MemoryService::new(db.as_ref().clone());
        let clipboard = ClipboardService::new(db.as_ref().clone());
        let backups = BackupService::new(db.as_ref().clone(), backup_dir);
        let data_port = DataPortService::new(db.as_ref().clone());
        let state = Self {
            settings: Arc::new(SettingsService::new(db.as_ref().clone())),
            smoke_notes: Arc::new(SmokeNoteService::new(db.as_ref().clone())),
            tasks: Arc::new(tasks),
            reminders: Arc::new(reminders),
            memories: Arc::new(memories),
            search: Arc::new(search),
            clipboard: Arc::new(clipboard),
            backups: Arc::new(backups),
            data_port: Arc::new(data_port),
            db,
        };
        if let Err(err) = state.search.rebuild_all() {
            tracing::warn!(error = %err, "search index rebuild failed");
        }
        let _ = state.clipboard.enforce_limits();
        Ok(state)
    }
}
