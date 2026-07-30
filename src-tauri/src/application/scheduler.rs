use crate::application::reminders::ReminderService;
use crate::domain::local_now_naive;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn start(app: AppHandle, reminders: Arc<ReminderService>) {
    std::thread::spawn(move || {
        if let Err(err) = reminders.reconcile_on_startup() {
            tracing::warn!(error = %err, "reminder reconcile failed");
        }

        loop {
            if let Err(err) = tick(&app, &reminders) {
                tracing::warn!(error = %err, "reminder scheduler tick failed");
            }
            std::thread::sleep(Duration::from_secs(20));
        }
    });
}

fn tick(app: &AppHandle, reminders: &ReminderService) -> Result<(), String> {
    let now = local_now_naive();
    let due = reminders
        .due_occurrences(now)
        .map_err(|e| e.to_string())?;

    for occ in due {
        let body = if occ.task_id.is_some() {
            "任务提醒到期"
        } else {
            "提醒到期"
        };

        let shown = app
            .notification()
            .builder()
            .title(&occ.title)
            .body(body)
            .show();

        match shown {
            Ok(()) => {
                let _ = reminders.mark_notified(occ.id, None);
                let _ = app.emit(
                    "domain://changed",
                    serde_json::json!({
                        "entityType": "reminder",
                        "entityId": occ.id.to_string(),
                        "change": "updated",
                        "revision": occ.revision,
                    }),
                );
                // Keep a soft open path for the main window.
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
            }
            Err(err) => {
                tracing::warn!(error = %err, "failed to show notification; will retry");
            }
        }
    }
    Ok(())
}
