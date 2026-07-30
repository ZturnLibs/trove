use crate::application::clipboard::ClipboardService;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub fn start(app: AppHandle, clipboard: Arc<ClipboardService>) {
    std::thread::spawn(move || {
        let mut last_text_hash: Option<String> = None;
        let mut last_image_hash: Option<String> = None;
        let mut ticks: u64 = 0;
        loop {
            // Text changes.
            if let Ok(content) = app.clipboard().read_text() {
                let hash = ClipboardService::hash_content(&content);
                if last_text_hash.as_ref() != Some(&hash) {
                    last_text_hash = Some(hash);
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
                            tracing::warn!(error = %err, "clipboard text capture failed");
                        }
                    }
                }
            }

            // Image changes.
            if let Ok(image) = app.clipboard().read_image() {
                let rgba = image.rgba().to_vec();
                let width = image.width();
                let height = image.height();
                // Hash raw pixels for change detection (cheap).
                let hash = {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(width.to_le_bytes());
                    hasher.update(height.to_le_bytes());
                    hasher.update(&rgba);
                    hex::encode(hasher.finalize())
                };
                if last_image_hash.as_ref() != Some(&hash) {
                    last_image_hash = Some(hash);
                    match clipboard.capture_image(width, height, &rgba, None) {
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
                            tracing::warn!(error = %err, "clipboard image capture failed");
                        }
                    }
                }
            }

            ticks = ticks.wrapping_add(1);
            if ticks % 150 == 0 {
                let _ = clipboard.enforce_limits();
            }
            std::thread::sleep(Duration::from_millis(800));
        }
    });
}
