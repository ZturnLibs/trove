//! Interactive region screenshot capture. PNG bytes only; never uploaded.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotError {
    Cancelled,
    Unavailable,
}

impl std::fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "用户取消了截图"),
            Self::Unavailable => write!(f, "当前平台暂不支持区域截图"),
        }
    }
}

/// Capture an interactive region as PNG bytes. User cancel returns `Cancelled`.
pub fn capture_region_png() -> Result<Vec<u8>, ScreenshotError> {
    #[cfg(target_os = "macos")]
    {
        return macos::capture_region_png();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::capture_region_png();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(ScreenshotError::Unavailable)
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::ScreenshotError;
    use std::process::Command;

    pub fn capture_region_png() -> Result<Vec<u8>, ScreenshotError> {
        let path = std::env::temp_dir().join(format!("trove-shot-{}.png", uuid::Uuid::new_v4()));
        let status = Command::new("screencapture")
            .args(["-i", "-x", "-t", "png", path.to_str().unwrap_or_default()])
            .status()
            .map_err(|_| ScreenshotError::Unavailable)?;
        if !status.success() {
            let _ = std::fs::remove_file(&path);
            return Err(ScreenshotError::Cancelled);
        }
        let bytes = std::fs::read(&path).map_err(|_| ScreenshotError::Unavailable)?;
        let _ = std::fs::remove_file(&path);
        if bytes.is_empty() {
            return Err(ScreenshotError::Cancelled);
        }
        Ok(bytes)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::ScreenshotError;

    pub fn capture_region_png() -> Result<Vec<u8>, ScreenshotError> {
        let _ = ScreenshotError::Unavailable;
        Err(ScreenshotError::Unavailable)
    }
}
