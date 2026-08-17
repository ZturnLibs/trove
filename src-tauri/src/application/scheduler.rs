use crate::app_state::AppState;
use crate::application::automation::event_from_reminder_fired;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn start(app: AppHandle, state: AppState) {
    std::thread::spawn(move || {
        if let Err(err) = state.reminders.reconcile_on_startup() {
            tracing::warn!(error = %err, "reminder reconcile failed");
        }

        loop {
            if let Err(err) = tick(&app, &state) {
                tracing::warn!(error = %err, "reminder scheduler tick failed");
            }
            std::thread::sleep(Duration::from_secs(20));
        }
    });
}

fn tick(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let now = crate::domain::local_now_naive();
    let due = state
        .reminders
        .due_occurrences(now)
        .map_err(|e| e.to_string())?;

    for occ in due {
        let scheduled_at = occ
            .scheduled_at
            .replace('T', " ")
            .chars()
            .take(16)
            .collect::<String>();
        let body = format!("计划时间 {scheduled_at}");

        let shown = app
            .notification()
            .builder()
            .title(&occ.title)
            .body(body)
            .show();

        match shown {
            Ok(()) => {
                let _ = state.reminders.mark_notified(occ.id, None);
                let _ = app.emit(
                    "domain://changed",
                    serde_json::json!({
                        "entityType": "reminder",
                        "entityId": occ.id.to_string(),
                        "change": "updated",
                        "revision": occ.revision,
                    }),
                );
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.emit(
                        "reminder://fired",
                        serde_json::json!({
                            "occurrenceId": occ.id.to_string(),
                            "reminderId": occ.reminder_id.to_string(),
                            "taskId": occ.task_id.map(|id| id.to_string()),
                            "title": occ.title,
                        }),
                    );
                }
                let event = event_from_reminder_fired(&occ);
                if let Err(err) = state.automation.run_for_event(
                    app,
                    &state.settings,
                    &state.tasks,
                    &state.memories,
                    event,
                    false,
                ) {
                    tracing::warn!(error = %err, "automation on reminder fired failed");
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, "failed to show notification; will retry");
            }
        }
    }
    Ok(())
}
