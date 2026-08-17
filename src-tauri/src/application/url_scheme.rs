use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::application::workbench_actions;
use crate::domain::{parse_trove_url, ActionDispatchOptions, WorkbenchAction};

pub fn handle_trove_url(app: &AppHandle, raw: &str) {
    match parse_trove_url(raw) {
        Ok(url_action) => {
            let action = WorkbenchAction::from(url_action);
            let options = ActionDispatchOptions::url_scheme();
            let state = app.try_state::<AppState>().map(|s| s.inner());
            if let Err(err) = workbench_actions::dispatch(app, state, action, options) {
                tracing::warn!(url = raw, error = %err, "workbench action dispatch failed");
            }
        }
        Err(err) => {
            tracing::warn!(url = raw, error = %err, "rejected trove url");
        }
    }
}
