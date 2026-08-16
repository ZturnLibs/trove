use chrono::Days;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands;
use crate::domain::{format_local_datetime, local_now_naive, parse_trove_url, UrlCreateKind, UrlSchemeAction};

pub fn handle_trove_url(app: &AppHandle, raw: &str) {
    match parse_trove_url(raw) {
        Ok(action) => dispatch_url_action(app, action),
        Err(err) => {
            tracing::warn!(url = raw, error = %err, "rejected trove url");
        }
    }
}

fn dispatch_url_action(app: &AppHandle, action: UrlSchemeAction) {
    match action {
        UrlSchemeAction::Navigate { path } => {
            let _ = commands::window_show_main(app.clone());
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("main://navigate", path);
            }
        }
        UrlSchemeAction::Search { query } => {
            let _ = commands::window_show_quick(app.clone(), Some("search".into()));
            if let Some(window) = app.get_webview_window("quick") {
                let _ = window.emit("quick://set-search-query", query);
            }
        }
        UrlSchemeAction::CreatePreview {
            kind,
            title,
            notes,
            due_date,
            fire_at,
        } => {
            let fire_at = if matches!(kind, UrlCreateKind::Reminder) {
                Some(fire_at.unwrap_or_else(default_reminder_fire_at))
            } else {
                fire_at
            };
            let payload = UrlSchemeAction::CreatePreview {
                kind,
                title,
                notes,
                due_date,
                fire_at,
            };
            let _ = commands::window_show_main(app.clone());
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("url-scheme://pending-create", payload);
            }
        }
    }
}

fn default_reminder_fire_at() -> String {
    let tomorrow = local_now_naive().date() + Days::new(1);
    let dt = tomorrow.and_hms_opt(9, 0, 0).expect("valid time");
    format_local_datetime(dt)
}
