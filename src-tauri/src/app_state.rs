use crate::application::automation::AutomationService;
use crate::application::backup::BackupService;
use crate::application::clipboard::ClipboardService;
use crate::application::daily_wrap::DailyWrapService;
use crate::application::data_port::DataPortService;
use crate::application::file_refs::FileReferenceService;
use crate::application::focus::FocusService;
use crate::application::health_dashboard::HealthDashboardService;
use crate::application::links::EntityLinkService;
use crate::application::memories::MemoryService;
use crate::application::reminders::ReminderService;
use crate::application::saved_views::SavedViewService;
use crate::application::search::SearchService;
use crate::application::smoke_notes::SmokeNoteService;
use crate::application::tasks::TaskService;
use crate::application::templates::TemplateService;
use crate::application::weekly_review::WeeklyReviewService;
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
    pub templates: Arc<TemplateService>,
    pub links: Arc<EntityLinkService>,
    pub saved_views: Arc<SavedViewService>,
    pub focus: Arc<FocusService>,
    pub daily_wrap: Arc<DailyWrapService>,
    pub weekly_review: Arc<WeeklyReviewService>,
    pub health_dashboard: Arc<HealthDashboardService>,
    pub file_refs: Arc<FileReferenceService>,
    pub automation: Arc<AutomationService>,
}

impl AppState {
    pub fn new(db: Database, backup_dir: PathBuf, assets_root: PathBuf) -> Result<Self, String> {
        let db = Arc::new(db);
        let tasks = TaskService::new(db.as_ref().clone());
        tasks
            .ensure_seed_data()
            .map_err(|e| format!("seed task data: {e}"))?;
        let reminders = ReminderService::new(db.as_ref().clone());
        let search = SearchService::new(db.as_ref().clone());
        let memories = MemoryService::new(db.as_ref().clone());
        let health_dashboard =
            HealthDashboardService::new(db.as_ref().clone(), assets_root.clone());
        let clipboard = ClipboardService::new(db.as_ref().clone(), assets_root);
        let backups = BackupService::new(db.as_ref().clone(), backup_dir);
        let data_port = DataPortService::new(db.as_ref().clone());
        let templates = TemplateService::new(db.as_ref().clone());
        let links = EntityLinkService::new(db.as_ref().clone());
        let saved_views = SavedViewService::new(db.as_ref().clone());
        let focus = FocusService::new(db.as_ref().clone());
        let daily_wrap = DailyWrapService::new(db.as_ref().clone());
        let weekly_review = WeeklyReviewService::new(db.as_ref().clone());
        let file_refs = FileReferenceService::new(db.as_ref().clone());
        let automation = AutomationService::new(db.as_ref().clone());
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
            templates: Arc::new(templates),
            links: Arc::new(links),
            saved_views: Arc::new(saved_views),
            focus: Arc::new(focus),
            daily_wrap: Arc::new(daily_wrap),
            weekly_review: Arc::new(weekly_review),
            health_dashboard: Arc::new(health_dashboard),
            file_refs: Arc::new(file_refs),
            automation: Arc::new(automation),
            db,
        };
        if let Err(err) = state.search.rebuild_all() {
            tracing::warn!(error = %err, "search index rebuild failed");
        }
        let _ = state.clipboard.enforce_limits();
        Ok(state)
    }
}
