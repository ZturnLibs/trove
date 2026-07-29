use crate::app_state::AppState;
use crate::application::smoke_notes::SmokeNote;
use crate::domain::{AppError, EntityId};
use crate::infrastructure::db::DbHealth;
use crate::infrastructure::settings::AppSettings;
use crate::platform::{detect_capabilities, PlatformCapabilities};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppHealth {
    pub ok: bool,
    pub app_version: String,
    pub database: DbHealth,
    pub capabilities: PlatformCapabilities,
}

#[tauri::command]
pub fn app_health(state: State<'_, AppState>) -> Result<AppHealth, AppError> {
    let database = state
        .db
        .health_check()
        .map_err(|e| AppError::retryable("db_unavailable", e.to_string()))?;

    Ok(AppHealth {
        ok: true,
        app_version: env!("CARGO_PKG_VERSION").into(),
        database,
        capabilities: detect_capabilities(),
    })
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    state.settings.get().map_err(Into::into)
}

#[tauri::command]
pub fn settings_save(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    state.settings.save(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn smoke_note_create(
    state: State<'_, AppState>,
    body: String,
) -> Result<SmokeNote, AppError> {
    tracing::info!(command = "smoke_note_create", "invoked");
    state.smoke_notes.create(body).map_err(Into::into)
}

#[tauri::command]
pub fn smoke_note_list(state: State<'_, AppState>) -> Result<Vec<SmokeNote>, AppError> {
    state.smoke_notes.list_active().map_err(Into::into)
}

#[tauri::command]
pub fn smoke_note_delete(state: State<'_, AppState>, id: EntityId) -> Result<(), AppError> {
    state.smoke_notes.soft_delete(id).map_err(Into::into)
}

#[tauri::command]
pub fn window_show_main(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        window
            .show()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
        window
            .set_focus()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_show_quick(
    app: tauri::AppHandle,
    mode: Option<String>,
) -> Result<(), AppError> {
    use tauri::Emitter;
    use tauri::Manager;

    if let Some(window) = app.get_webview_window("quick") {
        let mode = mode.unwrap_or_else(|| "capture".into());
        let _ = window.emit("quick://set-mode", mode);
        window
            .show()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
        window
            .set_focus()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn app_quit(app: tauri::AppHandle) {
    app.exit(0);
}
