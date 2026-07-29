//! Platform capability adapters.
//! Stage 0 keeps stubs that report availability; concrete OS behavior
//! is filled in as notification / paste / autostart features land.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub notifications: CapabilityStatus,
    pub global_shortcuts: CapabilityStatus,
    pub clipboard_read: CapabilityStatus,
    pub direct_paste: CapabilityStatus,
    pub autostart: CapabilityStatus,
    pub tray: CapabilityStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityStatus {
    pub available: bool,
    pub notes: String,
}

pub fn detect_capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        notifications: CapabilityStatus {
            available: true,
            notes: "Requires user permission; denied state must degrade gracefully.".into(),
        },
        global_shortcuts: CapabilityStatus {
            available: true,
            notes: "Conflicts should be reported and remappable.".into(),
        },
        clipboard_read: CapabilityStatus {
            available: true,
            notes: "Text only in v1; pause/exclude rules enforced later.".into(),
        },
        direct_paste: CapabilityStatus {
            available: cfg!(any(target_os = "macos", target_os = "windows")),
            notes: if cfg!(target_os = "macos") {
                "Requires Accessibility permission; falls back to copy.".into()
            } else if cfg!(target_os = "windows") {
                "Uses system input APIs; falls back to copy on failure.".into()
            } else {
                "Deferred until Linux adaptation.".into()
            },
        },
        autostart: CapabilityStatus {
            available: true,
            notes: "Optional; off by default.".into(),
        },
        tray: CapabilityStatus {
            available: true,
            notes: "Close main window hides to tray; Exit ends process.".into(),
        },
    }
}
