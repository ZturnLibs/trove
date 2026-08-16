//! macOS security-scoped bookmark helpers (local only).

#[derive(Debug, Clone)]
pub struct BookmarkData {
    pub bytes: Vec<u8>,
}

pub fn create_bookmark(path: &str) -> Option<BookmarkData> {
    #[cfg(target_os = "macos")]
    {
        return macos::create_bookmark(path).ok();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
    }
}

pub fn resolve_bookmark(data: &[u8]) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        return macos::resolve_bookmark(data).ok();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = data;
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::BookmarkData;
    use std::process::{Command, Stdio};

    const SWIFT_HELPER: &str = r#"
import Foundation

enum Mode: String { case create, resolve }

guard CommandLine.arguments.count >= 3,
      let mode = Mode(rawValue: CommandLine.arguments[1]) else {
    fputs("usage\n", stderr)
    exit(2)
}

switch mode {
case .create:
    let path = CommandLine.arguments[2]
    let url = URL(fileURLWithPath: path)
    do {
        let data = try url.bookmarkData(
            options: [.withSecurityScope, .securityScopeAllowOnlyReadAccess],
            includingResourceValuesForKeys: nil,
            relativeTo: nil
        )
        FileHandle.standardOutput.write(data)
    } catch {
        fputs("\(error)\n", stderr)
        exit(3)
    }
case .resolve:
    let b64 = CommandLine.arguments[2]
    guard let data = Data(base64Encoded: b64) else { exit(4) }
    var stale = false
    do {
        let url = try URL(
            resolvingBookmarkData: data,
            options: [.withSecurityScope, .withoutUI],
            relativeTo: nil,
            bookmarkDataIsStale: &stale
        )
        let started = url.startAccessingSecurityScopedResource()
        defer { if started { url.stopAccessingSecurityScopedResource() } }
        print(url.path)
    } catch {
        fputs("\(error)\n", stderr)
        exit(5)
    }
}
"#;

    fn helper_path() -> Result<std::path::PathBuf, String> {
        let dir = std::env::temp_dir().join("workbench-file-bookmark");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let swift_path = dir.join("bookmark_helper.swift");
        if !swift_path.exists() {
            std::fs::write(&swift_path, SWIFT_HELPER).map_err(|e| e.to_string())?;
        }
        Ok(swift_path)
    }

    pub fn create_bookmark(path: &str) -> Result<BookmarkData, String> {
        let swift_path = helper_path()?;
        let output = Command::new("swift")
            .arg(&swift_path)
            .arg("create")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("spawn bookmark helper: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
        Ok(BookmarkData {
            bytes: output.stdout,
        })
    }

    pub fn resolve_bookmark(data: &[u8]) -> Result<String, String> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(data);
        let swift_path = helper_path()?;
        let output = Command::new("swift")
            .arg(&swift_path)
            .arg("resolve")
            .arg(&b64)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("spawn bookmark helper: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}
