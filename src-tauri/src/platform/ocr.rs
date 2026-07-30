//! Local OCR adapters. Never upload image bytes.

#[derive(Debug, Clone, Default)]
pub struct OcrResult {
    pub text: String,
    pub engine_version: String,
}

/// Recognize text from PNG bytes. Failures return empty text (capture must not fail).
pub fn recognize_png(png_bytes: &[u8]) -> OcrResult {
    #[cfg(target_os = "macos")]
    {
        match macos::recognize_png(png_bytes) {
            Ok(text) => OcrResult {
                text: text.trim().to_string(),
                engine_version: "macos-vision-1".into(),
            },
            Err(err) => {
                tracing::debug!(error = %err, "local OCR unavailable or failed");
                OcrResult {
                    text: String::new(),
                    engine_version: "macos-vision-1".into(),
                }
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = png_bytes;
        OcrResult {
            text: String::new(),
            engine_version: "none".into(),
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    //! Uses Vision via a short-lived helper process so we avoid fragile objc bindings.
    //! The helper is pure Apple frameworks and never leaves the machine.
    use std::process::{Command, Stdio};

    const SWIFT_HELPER: &str = r#"
import Foundation
import Vision

guard CommandLine.arguments.count > 1 else {
    fputs("missing path\n", stderr)
    exit(2)
}
let url = URL(fileURLWithPath: CommandLine.arguments[1])
guard let request = try? VNRecognizeTextRequest(),
      let handler = try? VNImageRequestHandler(url: url, options: [:]) else {
    exit(3)
}
request.recognitionLevel = .accurate
request.usesLanguageCorrection = true
try handler.perform([request])
let observations = request.results ?? []
var lines: [String] = []
for obs in observations {
    if let candidate = obs.topCandidates(1).first {
        lines.append(candidate.string)
    }
}
print(lines.joined(separator: "\n"))
"#;

    pub fn recognize_png(png_bytes: &[u8]) -> Result<String, String> {
        let dir = std::env::temp_dir().join("workbench-ocr");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let png_path = dir.join(format!("{}.png", uuid::Uuid::new_v4()));
        let swift_path = dir.join("ocr_helper.swift");
        std::fs::write(&png_path, png_bytes).map_err(|e| e.to_string())?;
        if !swift_path.exists() {
            std::fs::write(&swift_path, SWIFT_HELPER).map_err(|e| e.to_string())?;
        }

        let output = Command::new("swift")
            .arg(&swift_path)
            .arg(&png_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("spawn swift OCR helper: {e}"))?;

        let _ = std::fs::remove_file(&png_path);
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("swift OCR failed: {err}"));
        }
        String::from_utf8(output.stdout).map_err(|e| e.to_string())
    }
}
