use crate::app_state::AppState;
use crate::commands;
use crate::infrastructure::settings::ShortcutSettings;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutApplyResult {
    pub ok: bool,
    pub errors: Vec<String>,
}

fn default_fallback(kind: &str) -> Shortcut {
    #[cfg(target_os = "macos")]
    let (mods, code) = match kind {
        "capture" => (Modifiers::SUPER | Modifiers::SHIFT, Code::Space),
        "search" => (Modifiers::SUPER | Modifiers::SHIFT, Code::KeyF),
        "clipboard" => (Modifiers::SUPER | Modifiers::SHIFT, Code::KeyV),
        "main" => (Modifiers::SUPER | Modifiers::SHIFT, Code::KeyA),
        "screenshot" => (Modifiers::SUPER | Modifiers::SHIFT, Code::Digit6),
        _ => (Modifiers::SUPER | Modifiers::SHIFT, Code::KeyA),
    };
    #[cfg(not(target_os = "macos"))]
    let (mods, code) = match kind {
        "capture" => (Modifiers::CONTROL | Modifiers::SHIFT, Code::Space),
        "search" => (Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyF),
        "clipboard" => (Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyV),
        "main" => (Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyA),
        "screenshot" => (Modifiers::CONTROL | Modifiers::SHIFT, Code::Digit6),
        _ => (Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyA),
    };
    Shortcut::new(Some(mods), code)
}

/// Unregister all global shortcuts and re-register from persisted settings.
pub fn apply_shortcuts(app: &AppHandle) -> ShortcutApplyResult {
    let mut result = ShortcutApplyResult {
        ok: true,
        errors: Vec::new(),
    };

    let settings = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.get().ok())
        .map(|s| s.shortcuts)
        .unwrap_or_else(ShortcutSettings::default);

    let pairs: [(&str, String); 5] = [
        ("capture", settings.quick_capture.clone()),
        ("search", settings.search.clone()),
        ("clipboard", settings.clipboard.clone()),
        ("main", settings.focus_main.clone()),
        ("screenshot", settings.screenshot_region.clone()),
    ];

    let mut seen = std::collections::HashSet::new();
    for (kind, raw) in &pairs {
        let key = raw.to_ascii_lowercase();
        if !seen.insert(key) {
            result.ok = false;
            result
                .errors
                .push(format!("快捷键「{raw}」重复用于多个动作（含 {kind}）"));
        }
    }
    if !result.ok {
        return result;
    }

    let gs = app.global_shortcut();
    if let Err(err) = gs.unregister_all() {
        tracing::warn!(error = %err, "unregister_all shortcuts failed (may be first run)");
    }

    for (kind, raw) in pairs {
        let shortcut = match raw.parse::<Shortcut>() {
            Ok(s) => s,
            Err(err) => {
                result.ok = false;
                result
                    .errors
                    .push(format!("无法解析「{raw}」（{kind}）: {err}，已用默认值"));
                default_fallback(kind)
            }
        };

        let kind_owned = kind.to_string();
        if let Err(err) = gs.on_shortcut(shortcut, move |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            match kind_owned.as_str() {
                "capture" => {
                    let _ = commands::window_show_quick(app.clone(), Some("capture".into()));
                }
                "search" => {
                    let _ = commands::window_show_quick(app.clone(), Some("search".into()));
                }
                "clipboard" => {
                    let _ = commands::window_show_quick(app.clone(), Some("clip".into()));
                }
                "main" => {
                    let _ = commands::window_show_main(app.clone());
                }
                "screenshot" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        match state.clipboard.capture_region_screenshot() {
                            Ok(Some(_item)) => {
                                let _ = app.emit(
                                    "domain://changed",
                                    serde_json::json!({
                                        "entityType": "clipboard",
                                        "entityId": "screenshot",
                                        "change": "created",
                                        "revision": 0
                                    }),
                                );
                            }
                            Ok(None) => {}
                            Err(err) => {
                                tracing::warn!(error = %err, "region screenshot failed");
                            }
                        }
                    }
                }
                _ => {}
            }
        }) {
            result.ok = false;
            result
                .errors
                .push(format!("注册「{raw}」（{kind}）失败: {err}"));
            tracing::warn!(error = %err, kind, raw, "failed to register global shortcut");
        }
    }

    result
}
