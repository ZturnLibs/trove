use crate::application::smoke_notes::SmokeNoteService;
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub settings: Arc<SettingsService>,
    pub smoke_notes: Arc<SmokeNoteService>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let db = Arc::new(db);
        Self {
            settings: Arc::new(SettingsService::new(db.as_ref().clone())),
            smoke_notes: Arc::new(SmokeNoteService::new(db.as_ref().clone())),
            db,
        }
    }
}
