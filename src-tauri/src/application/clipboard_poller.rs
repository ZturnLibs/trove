use crate::application::clipboard::ClipboardService;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub fn start(app: AppHandle, clipboard: Arc<ClipboardService>) {
    std::thread::spawn(move || {
        let mut last_hash: Option<String> = None;
        let mut ticks: u64 = 0;
        loop {
            let text = app.clipboard().read_text().ok();
            if let Some(content) = text {
                let hash = ClipboardService::hash_content(&content);
                if last_hash.as_ref() != Some(&hash) {
                    last_hash = Some(hash);
                    match clipboard.capture_text(content, None) {
                        Ok(Some(item)) => {
                            let _ = app.emit(
                                "domain://changed",
                                serde_json::json!({
                                    "entityType": "clipboard",
                                    "entityId": item.id.to_string(),
                                    "change": "created",
                                    "revision": item.revision,
                                }),
                            );
                        }
                        Ok(None) => {}
                        Err(err) => {
                            // Never log clipboard content.
                            tracing::warn!(error = %err, "clipboard capture failed");
                        }
                    }
                }
            }

            ticks = ticks.wrapping_add(1);
            // Soft maintenance ~ every 2 minutes.
            if ticks % 150 == 0 {
                let _ = clipboard.enforce_limits();
            }
            std::thread::sleep(Duration::from_millis(800));
        }
    });
}
