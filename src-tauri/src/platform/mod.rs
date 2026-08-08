//! Platform capability adapters.

pub mod ocr;

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
    pub ocr: CapabilityStatus,
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
            notes: "用于到期提醒。若系统拒绝通知权限，提醒仍会写入本地列表，但不会弹出系统通知。".into(),
        },
        global_shortcuts: CapabilityStatus {
            available: true,
            notes: "用于唤起快速记录、搜索与剪切板。若与其他应用冲突，可在设置中查看当前快捷键并恢复默认。".into(),
        },
        clipboard_read: CapabilityStatus {
            available: true,
            notes: "采集文本与图片。可随时暂停；密码管理器等应用默认排除。正文不会写入日志。图片 OCR 仅本地执行。".into(),
        },
        direct_paste: CapabilityStatus {
            available: false,
            notes: "直接粘贴暂未提供，请使用「再次复制」后手动粘贴。".into(),
        },
        autostart: CapabilityStatus {
            available: true,
            notes: "可选。开启后登录系统时后台运行，以便提醒与剪切板采集继续工作；关闭主窗口不会退出。".into(),
        },
        tray: CapabilityStatus {
            available: true,
            notes: "关闭主窗口会隐藏到托盘；只有「退出」才会结束进程。".into(),
        },
        ocr: ocr_capability(),
    }
}

fn ocr_capability() -> CapabilityStatus {
    CapabilityStatus {
        available: cfg!(any(target_os = "macos", target_os = "windows")),
        notes: ocr_capability_notes(),
    }
}

fn ocr_capability_notes() -> String {
    if cfg!(target_os = "macos") {
        "macOS 使用本机 Vision 识别图片文字。".into()
    } else if cfg!(target_os = "windows") {
        "Windows 使用本机 Media OCR 识别图片文字；需安装对应语言包。".into()
    } else {
        "当前平台暂不支持，图片无法按文字搜索。".into()
    }
}
