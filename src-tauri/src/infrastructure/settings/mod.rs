use crate::domain::{format_utc, AIConfig, Clock, DomainError, SystemClock};
use crate::infrastructure::db::Database;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: ThemePreference,
    pub launch_at_login: bool,
    pub shortcuts: ShortcutSettings,
    pub clipboard_capture_enabled: bool,
    #[serde(default = "default_retention_days")]
    pub clipboard_retention_days: u32,
    #[serde(default = "default_max_items")]
    pub clipboard_max_items: u32,
    #[serde(default = "crate::domain::default_excluded_apps")]
    pub clipboard_excluded_apps: Vec<String>,
    #[serde(default = "default_true")]
    pub clipboard_smart_actions_enabled: bool,
    #[serde(default = "default_true")]
    pub today_smart_sort_enabled: bool,
    #[serde(default = "default_true")]
    pub auto_backup_on_launch: bool,
    #[serde(default = "default_true")]
    pub auto_check_updates: bool,
    #[serde(default = "default_true")]
    pub automation_enabled: bool,
    #[serde(default = "default_backup_keep")]
    pub backup_retention_count: u32,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default)]
    pub last_focus_carry_dismissed_date: Option<String>,
    #[serde(default)]
    pub ai: AIConfig,
}

fn default_true() -> bool {
    true
}

fn default_backup_keep() -> u32 {
    10
}

fn default_screenshot_region() -> String {
    #[cfg(target_os = "macos")]
    {
        "Command+Alt+X".into()
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl+Alt+X".into()
    }
}

fn default_max_items() -> u32 {
    500
}

fn default_retention_days() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutSettings {
    pub quick_capture: String,
    pub search: String,
    pub clipboard: String,
    pub focus_main: String,
    #[serde(default = "default_screenshot_region")]
    pub screenshot_region: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            launch_at_login: false,
            shortcuts: ShortcutSettings::default(),
            clipboard_capture_enabled: true,
            clipboard_retention_days: 30,
            clipboard_max_items: 500,
            clipboard_excluded_apps: crate::domain::default_excluded_apps(),
            clipboard_smart_actions_enabled: true,
            today_smart_sort_enabled: true,
            auto_backup_on_launch: true,
            auto_check_updates: true,
            automation_enabled: true,
            backup_retention_count: 10,
            onboarding_completed: false,
            last_focus_carry_dismissed_date: None,
            ai: AIConfig::default(),
        }
    }
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        // 默认键位选用 ⌘⌥（Ctrl+Alt）组合：避开系统键（Spotlight ⌘Space、输入法 ⌃Space /
        // Ctrl+Space、系统截图 ⌘⇧3/4/5）、常见输入法键（微软拼音简繁 Ctrl+Shift+F）、
        // 以及高频应用全局键（Chrome/Edge 标签页搜索 Ctrl+Shift+A、无格式粘贴 Ctrl+Shift+V 等）。
        #[cfg(target_os = "macos")]
        {
            Self {
                quick_capture: "Command+Alt+Space".into(),
                search: "Command+Alt+F".into(),
                clipboard: "Command+Alt+C".into(),
                focus_main: "Command+Alt+T".into(),
                screenshot_region: default_screenshot_region(),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                quick_capture: "Ctrl+Alt+Space".into(),
                search: "Ctrl+Alt+F".into(),
                clipboard: "Ctrl+Alt+C".into(),
                focus_main: "Ctrl+Alt+T".into(),
                screenshot_region: default_screenshot_region(),
            }
        }
    }
}

impl ShortcutSettings {
    /// v2.0.x 及更早版本的出厂默认键位。仅用于一次性迁移：存储值与旧默认完全一致
    /// （即用户从未改过）时，升级后自动换成新默认，消除与其他应用/系统的冲突；
    /// 用户自定义过的键位不受影响。
    pub fn legacy_defaults() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                quick_capture: "Command+Shift+Space".into(),
                search: "Command+Shift+F".into(),
                clipboard: "Command+Shift+V".into(),
                focus_main: "Command+Shift+A".into(),
                screenshot_region: "Command+Shift+6".into(),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                quick_capture: "Ctrl+Shift+Space".into(),
                search: "Ctrl+Shift+F".into(),
                clipboard: "Ctrl+Shift+V".into(),
                focus_main: "Ctrl+Shift+A".into(),
                screenshot_region: "Ctrl+Shift+6".into(),
            }
        }
    }

    /// 存储值仍是旧出厂默认（用户未自定义）时替换为新默认。返回是否有变更。
    pub fn migrate_legacy_defaults(&mut self) -> bool {
        if *self == Self::legacy_defaults() {
            *self = Self::default();
            true
        } else {
            false
        }
    }
}

pub struct SettingsService {
    db: Database,
    clock: SystemClock,
}

impl SettingsService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    pub fn get(&self) -> Result<AppSettings, DomainError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let value: Option<String> = conn
            .query_row(
                "SELECT value_json FROM settings WHERE key = ?1",
                ["app.settings"],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        match value {
            Some(raw) => {
                let mut settings: AppSettings = serde_json::from_str(&raw)
                    .map_err(|e| DomainError::Internal(format!("invalid settings json: {e}")))?;
                // 一次性迁移：从未自定义过快捷键的老安装，升级到新出厂默认。
                if settings.shortcuts.migrate_legacy_defaults() {
                    self.save(&settings)?;
                }
                Ok(settings)
            }
            None => {
                let defaults = AppSettings::default();
                self.save(&defaults)?;
                Ok(defaults)
            }
        }
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), DomainError> {
        if settings.backup_retention_count == 0 || settings.backup_retention_count > 100 {
            return Err(DomainError::Validation("备份保留数量需在 1–100".into()));
        }
        settings.ai.validate()?;
        let conn = self
            .db
            .connect()
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let json =
            serde_json::to_string(settings).map_err(|e| DomainError::Internal(e.to_string()))?;
        let now = format_utc(self.clock.now());
        conn.execute(
            "INSERT INTO settings (key, value_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
               value_json = excluded.value_json,
               updated_at = excluded.updated_at",
            rusqlite::params!["app.settings", json, now],
        )
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    pub fn get_raw(&self, key: &str) -> Result<Option<Value>, DomainError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let value: Option<String> = conn
            .query_row(
                "SELECT value_json FROM settings WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        value
            .map(|raw| {
                serde_json::from_str(&raw)
                    .map_err(|e| DomainError::Internal(format!("invalid settings json: {e}")))
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    #[test]
    fn defaults_persist_and_roundtrip() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let service = SettingsService::new(db);
        let settings = service.get().unwrap();
        assert_eq!(settings.theme, ThemePreference::System);
        assert_eq!(settings.ai, AIConfig::default());

        let mut updated = settings.clone();
        updated.theme = ThemePreference::Dark;
        service.save(&updated).unwrap();
        assert_eq!(service.get().unwrap().theme, ThemePreference::Dark);
    }

    #[test]
    fn legacy_settings_json_defaults_ai_to_off() {
        // v1.x exports carry no `ai` key; restored settings must keep AI off.
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let conn = db.connect().unwrap();
        let legacy = serde_json::json!({
            "theme": "system",
            "launchAtLogin": false,
            "shortcuts": {
                "quickCapture": "Command+Shift+Space",
                "search": "Command+Shift+F",
                "clipboard": "Command+Shift+V",
                "focusMain": "Command+Shift+A",
                "screenshotRegion": "Command+Shift+6"
            },
            "clipboardCaptureEnabled": true,
            "clipboardRetentionDays": 30,
            "clipboardMaxItems": 500
        });
        conn.execute(
            "INSERT INTO settings (key, value_json, updated_at) VALUES ('app.settings', ?1, '2026-01-01T00:00:00Z')",
            rusqlite::params![legacy.to_string()],
        )
        .unwrap();
        drop(conn);
        let service = SettingsService::new(db);
        let settings = service.get().unwrap();
        assert_eq!(settings.ai.mode, crate::domain::AIMode::Off);
        assert!(!settings.ai.features.extract);
    }

    #[test]
    fn defaults_differ_from_legacy_defaults() {
        // 新默认必须与旧默认不同，否则迁移与「避免冲突」目标无意义。
        assert_ne!(ShortcutSettings::default(), ShortcutSettings::legacy_defaults());
    }

    #[test]
    fn untouched_legacy_shortcut_defaults_are_migrated() {
        // 用户从未改过快捷键（存储值 = 旧出厂默认）时，读取时自动换成新默认并持久化。
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let conn = db.connect().unwrap();
        let legacy = serde_json::json!({
            "theme": "system",
            "launchAtLogin": false,
            "shortcuts": serde_json::to_value(ShortcutSettings::legacy_defaults()).unwrap()
        });
        conn.execute(
            "INSERT INTO settings (key, value_json, updated_at) VALUES ('app.settings', ?1, '2026-01-01T00:00:00Z')",
            rusqlite::params![legacy.to_string()],
        )
        .unwrap();
        drop(conn);
        let service = SettingsService::new(db);
        let settings = service.get().unwrap();
        assert_eq!(settings.shortcuts, ShortcutSettings::default());
        // 迁移结果已持久化，再次读取不再触发变更。
        let again = service.get().unwrap();
        assert_eq!(again.shortcuts, ShortcutSettings::default());
    }

    #[test]
    fn customized_shortcuts_survive_migration() {
        // 用户自定义过的键位原样保留，不做迁移。
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let conn = db.connect().unwrap();
        let mut shortcuts = ShortcutSettings::legacy_defaults();
        shortcuts.quick_capture = "Ctrl+Alt+P".into();
        let custom = serde_json::json!({
            "theme": "system",
            "launchAtLogin": false,
            "shortcuts": serde_json::to_value(shortcuts).unwrap()
        });
        conn.execute(
            "INSERT INTO settings (key, value_json, updated_at) VALUES ('app.settings', ?1, '2026-01-01T00:00:00Z')",
            rusqlite::params![custom.to_string()],
        )
        .unwrap();
        drop(conn);
        let service = SettingsService::new(db);
        let settings = service.get().unwrap();
        assert_eq!(settings.shortcuts.quick_capture, "Ctrl+Alt+P");
        assert_ne!(settings.shortcuts, ShortcutSettings::default());
    }

    #[test]
    fn auto_check_updates_defaults_true() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let service = SettingsService::new(db);
        assert!(service.get().unwrap().auto_check_updates);
    }
}
