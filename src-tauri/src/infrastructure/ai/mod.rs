//! AI provider boundary (v2.0 slice 1).
//!
//! Sync trait on purpose: commands run on Tauri's thread pool and
//! probe/complete are user-triggered, bounded (20s) calls. `OffProvider`
//! is the default and performs zero I/O — the §9.1 gate-4 guarantee that
//! every v1.x path stays untouched when AI is off.
//!
//! Privacy contract: request/response bodies are NEVER logged here. The
//! application layer logs provider/model/feature/byte-count only.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Deserialize;

use crate::domain::{
    AIConfig, AIMode, CompletionOutput, CompletionRequest, DomainError, ProbeReport,
};

const PROBE_TIMEOUT_SECS: u64 = 5;
const COMPLETE_TIMEOUT_SECS: u64 = 20;
const KEY_FILE_NAME: &str = "ai_provider_key";

/// Provider abstraction injected into `AISuggestionService` as a trait
/// object (same pattern as `Clock`), so tests run against fakes.
pub trait AIProvider: Send + Sync {
    fn probe(&self) -> ProbeReport;
    fn complete(&self, request: &CompletionRequest) -> Option<CompletionOutput>;
}

/// Default provider. Zero I/O, always unavailable.
pub struct OffProvider;

impl AIProvider for OffProvider {
    fn probe(&self) -> ProbeReport {
        ProbeReport {
            mode: AIMode::Off,
            reachable: false,
            model: None,
            latency_ms: None,
            hint: Some("ai.off".into()),
        }
    }

    fn complete(&self, _request: &CompletionRequest) -> Option<CompletionOutput> {
        None
    }
}

/// OpenAI-compatible HTTP provider. Serves both `AIMode::Ollama`
/// (`{ollama_url}/v1/chat/completions`) and `AIMode::Custom`
/// (`{endpoint}/chat/completions`, endpoint includes `/v1`).
pub struct HttpProvider {
    base_url: String,
    model: String,
    mode: AIMode,
    key_path: PathBuf,
    requires_key: bool,
}

impl HttpProvider {
    pub fn new(config: &AIConfig, data_dir: &Path) -> Self {
        let key_path = data_dir.join(KEY_FILE_NAME);
        match config.mode {
            AIMode::Ollama => Self {
                base_url: format!("{}/v1", config.ollama_url.trim_end_matches('/')),
                model: config.ollama_model.trim().to_string(),
                mode: AIMode::Ollama,
                key_path,
                requires_key: false,
            },
            AIMode::Custom => Self {
                base_url: config.custom_endpoint.trim_end_matches('/').to_string(),
                model: config.custom_model.trim().to_string(),
                mode: AIMode::Custom,
                key_path,
                requires_key: true,
            },
            AIMode::Off => Self {
                base_url: String::new(),
                model: String::new(),
                mode: AIMode::Off,
                key_path,
                requires_key: false,
            },
        }
    }

    fn client(&self, timeout_secs: u64) -> Option<reqwest::blocking::Client> {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .ok()
    }

    fn configured(&self) -> bool {
        !self.base_url.is_empty() && !self.model.is_empty() && !self.requires_key_or_missing()
    }

    fn requires_key_or_missing(&self) -> bool {
        self.requires_key && !self.key_path.is_file()
    }
}

impl AIProvider for HttpProvider {
    fn probe(&self) -> ProbeReport {
        if self.mode == AIMode::Off {
            return OffProvider.probe();
        }
        if self.base_url.is_empty() {
            return ProbeReport {
                mode: self.mode,
                reachable: false,
                model: None,
                latency_ms: None,
                hint: Some("ai.probe.endpoint-missing".into()),
            };
        }
        if self.model.is_empty() {
            return ProbeReport {
                mode: self.mode,
                reachable: false,
                model: None,
                latency_ms: None,
                hint: Some("ai.probe.model-missing".into()),
            };
        }
        if self.requires_key_or_missing() {
            return ProbeReport {
                mode: self.mode,
                reachable: false,
                model: None,
                latency_ms: None,
                hint: Some("ai.probe.key-missing".into()),
            };
        }

        let started = Instant::now();
        let reachable = self
            .client(PROBE_TIMEOUT_SECS)
            .and_then(|client| {
                let mut req = client.get(format!("{}/models", self.base_url));
                if let Some(key) = read_provider_key(&self.key_path) {
                    req = req.bearer_auth(key);
                }
                req.send().ok()
            })
            .map(|resp| resp.status().is_success() || resp.status().as_u16() == 401)
            .unwrap_or(false);

        ProbeReport {
            mode: self.mode,
            reachable,
            model: Some(self.model.clone()),
            latency_ms: Some(started.elapsed().as_millis() as u64),
            hint: if reachable {
                None
            } else {
                Some(match self.mode {
                    AIMode::Ollama => "ai.probe.ollama-guide".into(),
                    _ => "ai.probe.unreachable".into(),
                })
            },
        }
    }

    fn complete(&self, request: &CompletionRequest) -> Option<CompletionOutput> {
        if !self.configured() {
            return None;
        }
        let key = read_provider_key(&self.key_path);
        let client = self.client(COMPLETE_TIMEOUT_SECS)?;

        // Minimal necessary context (§9.4): bounded, truncated upstream.
        let body = serde_json::json!({
            "model": self.model,
            "temperature": 0,
            "stream": false,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": request.system_prompt },
                { "role": "user", "content": request.truncated_context() },
            ],
        });

        let mut req = client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body);
        if let (true, Some(key)) = (self.requires_key, key) {
            req = req.bearer_auth(key);
        }

        // Failures map to None: suggestions are optional assistance; the
        // caller degrades silently (audited upstream, never blocking UX).
        let response = req.send().ok()?;
        if !response.status().is_success() {
            return None;
        }
        let parsed: ChatCompletionResponse = response.json().ok()?;
        let content = parsed.choices.first()?.message.content.clone();
        if content.trim().is_empty() {
            return None;
        }
        Some(CompletionOutput { raw_json: content })
    }
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: String,
}

/// Build the concrete provider for the current settings. `Off` settings or
/// missing prerequisites yield `OffProvider`-equivalent behavior via
/// `HttpProvider` short-circuits, keeping the trait object uniform.
pub fn build_provider(config: &AIConfig, data_dir: &Path) -> Box<dyn AIProvider> {
    match config.mode {
        AIMode::Off => Box::new(OffProvider),
        _ => Box::new(HttpProvider::new(config, data_dir)),
    }
}

// ---------------------------------------------------------------------------
// API key file (outside the database: never travels with backups/exports)
// ---------------------------------------------------------------------------

pub fn provider_key_path(data_dir: &Path) -> PathBuf {
    data_dir.join(KEY_FILE_NAME)
}

pub fn provider_key_exists(data_dir: &Path) -> bool {
    provider_key_path(data_dir).is_file()
}

pub fn write_provider_key(data_dir: &Path, key: &str) -> Result<(), DomainError> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Validation("API Key 不能为空".into()));
    }
    fs::write(provider_key_path(data_dir), trimmed)
        .map_err(|e| DomainError::Internal(format!("写入 API Key 失败: {e}")))
}

pub fn clear_provider_key(data_dir: &Path) -> Result<(), DomainError> {
    let path = provider_key_path(data_dir);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| DomainError::Internal(format!("清除 API Key 失败: {e}")))?;
    }
    Ok(())
}

/// Read the key; absent file is `None` (Custom mode then reports
/// key-missing on probe). Content is never logged or echoed back to the UI.
fn read_provider_key(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_provider_is_zero_io() {
        let report = OffProvider.probe();
        assert!(!report.reachable);
        assert_eq!(report.hint.as_deref(), Some("ai.off"));
        assert!(OffProvider
            .complete(&CompletionRequest::new("s", "u"))
            .is_none());
    }

    #[test]
    fn http_provider_unreachable_endpoint_returns_none_fast() {
        let dir = tempfile::tempdir().unwrap();
        let config = AIConfig {
            mode: AIMode::Custom,
            custom_endpoint: "http://127.0.0.1:9/v1".into(), // closed port
            custom_model: "test-model".into(),
            ..Default::default()
        };
        write_provider_key(dir.path(), "sk-test").unwrap();

        let provider = HttpProvider::new(&config, dir.path());
        let started = Instant::now();
        assert!(provider
            .complete(&CompletionRequest::new("system", "user"))
            .is_none());
        assert!(started.elapsed().as_secs() < COMPLETE_TIMEOUT_SECS + 5);
    }

    #[test]
    fn custom_mode_without_key_is_not_configured() {
        let dir = tempfile::tempdir().unwrap();
        let config = AIConfig {
            mode: AIMode::Custom,
            custom_endpoint: "https://api.example.com/v1".into(),
            custom_model: "m".into(),
            ..Default::default()
        };

        let provider = HttpProvider::new(&config, dir.path());
        assert!(!provider.configured());
        let report = provider.probe();
        assert!(!report.reachable);
        assert_eq!(report.hint.as_deref(), Some("ai.probe.key-missing"));

        write_provider_key(dir.path(), "sk-x").unwrap();
        assert!(provider.configured());
        clear_provider_key(dir.path()).unwrap();
        assert!(!provider_key_exists(dir.path()));
    }

    #[test]
    fn ollama_mode_without_model_hints_selection() {
        let dir = tempfile::tempdir().unwrap();
        let config = AIConfig {
            mode: AIMode::Ollama, // ollama_model left empty
            ..Default::default()
        };

        let provider = HttpProvider::new(&config, dir.path());
        let report = provider.probe();
        assert!(!report.reachable);
        assert_eq!(report.hint.as_deref(), Some("ai.probe.model-missing"));
    }

    #[test]
    fn ollama_base_url_normalizes_trailing_slash() {
        let dir = tempfile::tempdir().unwrap();
        let config = AIConfig {
            mode: AIMode::Ollama,
            ollama_url: "http://localhost:11434/".into(),
            ollama_model: "qwen3:4b".into(),
            ..Default::default()
        };

        let provider = HttpProvider::new(&config, dir.path());
        assert!(provider.configured());
        assert!(provider.probe().latency_ms.is_some());
    }

    #[test]
    fn build_provider_off_settings_yields_off() {
        let dir = tempfile::tempdir().unwrap();
        let provider = build_provider(&AIConfig::default(), dir.path());
        assert!(!provider.probe().reachable);
    }

    #[test]
    fn key_file_lives_outside_database_by_construction() {
        let dir = tempfile::tempdir().unwrap();
        write_provider_key(dir.path(), "sk-secret").unwrap();
        // The key sits as a plain file next to (not inside) workbench.db.
        assert!(dir.path().join(KEY_FILE_NAME).is_file());
        assert!(!dir.path().join("workbench.db").exists());
    }
}
