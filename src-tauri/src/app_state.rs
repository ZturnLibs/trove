use crate::application::reminders::ReminderService;
use crate::application::smoke_notes::SmokeNoteService;
use crate::application::tasks::TaskService;
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub settings: Arc<SettingsService>,
    pub smoke_notes: Arc<SmokeNoteService>,
    pub tasks: Arc<TaskService>,
    pub reminders: Arc<ReminderService>,
}

impl AppState {
    pub fn new(db: Database) -> Result<Self, String> {
        let db = Arc::new(db);
        let tasks = TaskService::new(db.as_ref().clone());
        tasks
            .ensure_seed_data()
            .map_err(|e| format!("seed task data: {e}"))?;
        let reminders = ReminderService::new(db.as_ref().clone());
        Ok(Self {
            settings: Arc::new(SettingsService::new(db.as_ref().clone())),
            smoke_notes: Arc::new(SmokeNoteService::new(db.as_ref().clone())),
            tasks: Arc::new(tasks),
            reminders: Arc::new(reminders),
            db,
        })
    }
}
