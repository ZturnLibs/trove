use crate::domain::{format_utc, Clock, DomainError, SystemClock};
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
    #[serde(default = "default_backup_keep")]
    pub backup_retention_count: u32,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default)]
    pub last_focus_carry_dismissed_date: Option<String>,
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
        "Command+Shift+6".into()
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl+Shift+6".into()
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            backup_retention_count: 10,
            onboarding_completed: false,
            last_focus_carry_dismissed_date: None,
        }
    }
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                quick_capture: "Command+Shift+Space".into(),
                search: "Command+Shift+F".into(),
                clipboard: "Command+Shift+V".into(),
                focus_main: "Command+Shift+A".into(),
                screenshot_region: default_screenshot_region(),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                quick_capture: "Ctrl+Shift+Space".into(),
                search: "Ctrl+Shift+F".into(),
                clipboard: "Ctrl+Shift+V".into(),
                focus_main: "Ctrl+Shift+A".into(),
                screenshot_region: default_screenshot_region(),
            }
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
            Some(raw) => serde_json::from_str(&raw)
                .map_err(|e| DomainError::Internal(format!("invalid settings json: {e}"))),
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

        let mut updated = settings.clone();
        updated.theme = ThemePreference::Dark;
        service.save(&updated).unwrap();
        assert_eq!(service.get().unwrap().theme, ThemePreference::Dark);
    }

    #[test]
    fn auto_check_updates_defaults_true() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let service = SettingsService::new(db);
        assert!(service.get().unwrap().auto_check_updates);
    }
}
