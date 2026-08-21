//! AI suggestion service (v2.0 slice 1): sanitize → provider → validate →
//! ledger. Business modules only ever receive structured records; invalid
//! model output is discarded with an audit row and never touches business
//! data (post-v1 §9.5).

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{
    new_id, parse_suggestion_content, stamp, AIFeature, AISuggestionRecord, CompletionRequest,
    ContextItem, DomainError, SuggestionSource, SuggestionStatus, SystemClock,
};
use crate::infrastructure::ai::{build_provider, AIProvider};
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;

pub struct AISuggestionService {
    db: Database,
    clock: SystemClock,
    settings: Arc<SettingsService>,
    provider: Arc<dyn AIProvider>,
}

impl AISuggestionService {
    pub fn new(
        db: Database,
        settings: Arc<SettingsService>,
        data_dir: PathBuf,
    ) -> Result<Self, String> {
        let config = settings.get().map_err(|e| e.to_string())?;
        let provider: Arc<dyn AIProvider> = Arc::from(build_provider(&config.ai, &data_dir));
        Ok(Self {
            db,
            clock: SystemClock,
            settings,
            provider,
        })
    }

    /// Test constructor with a provider fake.
    #[cfg(test)]
    pub fn with_provider(
        db: Database,
        settings: Arc<SettingsService>,
        provider: Arc<dyn AIProvider>,
    ) -> Self {
        Self {
            db,
            clock: SystemClock,
            settings,
            provider,
        }
    }

    /// §9.4 red line enforcement. Drops:
    /// - memories flagged `sensitive`
    /// - items whose source app matches the user exclusion list or the
    ///   built-in password-manager list (same semantics as clipboard capture)
    /// - empty texts
    pub fn sanitize_context(&self, items: &[ContextItem]) -> Vec<ContextItem> {
        let settings = self.settings.get().unwrap_or_default();
        let excluded: Vec<String> = settings
            .clipboard_excluded_apps
            .iter()
            .chain(crate::domain::default_excluded_apps().iter())
            .cloned()
            .collect();

        let conn = match self.db.connect() {
            Ok(conn) => conn,
            Err(_) => return Vec::new(),
        };

        items
            .iter()
            .filter(|item| !item.text.trim().is_empty())
            .filter(|item| match &item.source_app {
                Some(app) => !excluded
                    .iter()
                    .any(|ex| app.eq_ignore_ascii_case(ex) || app.contains(ex.as_str())),
                None => true,
            })
            .filter(|item| {
                if item.entity_type != "memory" {
                    return true;
                }
                let sensitive: Option<i64> = conn
                    .query_row(
                        "SELECT sensitive FROM memories WHERE id = ?1",
                        [&item.entity_id],
                        |row| row.get(0),
                    )
                    .optional()
                    .ok()
                    .flatten();
                sensitive.map(|s| s == 0).unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Full suggestion pipeline. Returns `None` (no side effects on
    /// business data) when the feature is disabled, unopened, sanitized to
    /// empty, the provider is off/unreachable, or output fails validation.
    pub fn request(
        &self,
        feature: AIFeature,
        source_entity_type: &str,
        source_entity_id: &str,
        context: &[ContextItem],
    ) -> Result<Option<AISuggestionRecord>, DomainError> {
        let settings = self.settings.get()?;
        if !settings.ai.features.enabled(feature) {
            return Ok(None);
        }
        let Some(system_prompt) = feature.prompt_template() else {
            return Ok(None); // feature not yet opened (slice gate)
        };

        let sanitized = self.sanitize_context(context);
        if sanitized.is_empty() {
            return Ok(None);
        }

        let user_context = sanitized
            .iter()
            .map(|item| format!("[{}:{}] {}", item.entity_type, item.entity_id, item.text))
            .collect::<Vec<_>>()
            .join("\n\n");

        let request = CompletionRequest::new(system_prompt, &user_context);
        let Some(output) = self.provider.complete(&request) else {
            // Provider off/unreachable: silent degrade, nothing recorded.
            return Ok(None);
        };

        // Metadata-only audit log: never the prompt or full output body.
        tracing::info!(
            feature = feature.as_str(),
            provider = settings.ai.mode.as_str(),
            bytes = output.raw_json.len(),
            "ai completion received"
        );

        match parse_suggestion_content(&output.raw_json) {
            Ok(content) => {
                let sources = content
                    .items
                    .iter()
                    .map(|item| SuggestionSource {
                        entity_type: source_entity_type.to_string(),
                        entity_id: source_entity_id.to_string(),
                        text_offset: 0,
                        excerpt: item.source_excerpt.clone(),
                    })
                    .collect();
                let record = AISuggestionRecord {
                    id: new_id().to_string(),
                    feature_type: feature.as_str().to_string(),
                    source_entity_type: source_entity_type.to_string(),
                    source_entity_id: source_entity_id.to_string(),
                    payload: content,
                    sources,
                    status: SuggestionStatus::Pending,
                    provider: settings.ai.mode.as_str().to_string(),
                    model: active_model_name(&settings.ai),
                    created_at: stamp(&self.clock),
                    decided_at: None,
                };
                self.insert(&record)?;
                Ok(Some(record))
            }
            Err(reason) => {
                // Invalid output: discard with an audit row; business data
                // stays untouched. The row keeps provenance for review.
                tracing::warn!(
                    feature = feature.as_str(),
                    reason = %reason,
                    "ai output rejected by validation"
                );
                self.insert_invalid(feature, source_entity_type, source_entity_id, &settings.ai)?;
                Ok(None)
            }
        }
    }

    pub fn list(
        &self,
        feature: Option<AIFeature>,
        status: Option<SuggestionStatus>,
    ) -> Result<Vec<AISuggestionRecord>, DomainError> {
        let conn = self.connect()?;
        let mut sql = String::from(
            "SELECT id, feature_type, source_entity_type, source_entity_id, payload, sources_json,
                    status, provider, model, created_at, decided_at
             FROM ai_suggestions WHERE 1=1",
        );
        let mut bindings: Vec<String> = Vec::new();
        if let Some(feature) = feature {
            sql.push_str(" AND feature_type = ?");
            bindings.push(feature.as_str().to_string());
        }
        if let Some(status) = status {
            sql.push_str(" AND status = ?");
            bindings.push(status.as_str().to_string());
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 200");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(bindings.iter()), |row| {
                Ok(SuggestionRow {
                    id: row.get(0)?,
                    feature_type: row.get(1)?,
                    source_entity_type: row.get(2)?,
                    source_entity_id: row.get(3)?,
                    payload_json: row.get(4)?,
                    sources_json: row.get(5)?,
                    status: row.get(6)?,
                    provider: row.get(7)?,
                    model: row.get(8)?,
                    created_at: row.get(9)?,
                    decided_at: row.get(10)?,
                })
            })
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row_from_db(row.map_err(|e| DomainError::Internal(e.to_string()))?)?);
        }
        Ok(records)
    }

    /// pending → accepted/rejected/dismissed. Terminal states are final.
    pub fn decide(
        &self,
        id: &str,
        status: SuggestionStatus,
    ) -> Result<AISuggestionRecord, DomainError> {
        let conn = self.connect()?;
        let updated = conn
            .execute(
                "UPDATE ai_suggestions SET status = ?2, decided_at = ?3
                 WHERE id = ?1 AND status = 'pending'",
                params![id, status.as_str(), stamp(&self.clock)],
            )
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        if updated == 0 {
            return Err(DomainError::Validation("建议不存在或已处理".into()));
        }
        self.get(id)
    }

    pub fn get(&self, id: &str) -> Result<AISuggestionRecord, DomainError> {
        let conn = self.connect()?;
        let row = conn
            .query_row(
                "SELECT id, feature_type, source_entity_type, source_entity_id, payload, sources_json,
                        status, provider, model, created_at, decided_at
                 FROM ai_suggestions WHERE id = ?1",
                [id],
                |row| {
                    Ok(SuggestionRow {
                        id: row.get(0)?,
                        feature_type: row.get(1)?,
                        source_entity_type: row.get(2)?,
                        source_entity_id: row.get(3)?,
                        payload_json: row.get(4)?,
                        sources_json: row.get(5)?,
                        status: row.get(6)?,
                        provider: row.get(7)?,
                        model: row.get(8)?,
                        created_at: row.get(9)?,
                        decided_at: row.get(10)?,
                    })
                },
            )
            .optional()
            .map_err(|e| DomainError::Internal(e.to_string()))?
            .ok_or_else(|| DomainError::Validation("建议不存在".into()))?;
        row_from_db(row)
    }

    /// Derived data only: clearing never touches business tables.
    pub fn clear_history(&self) -> Result<usize, DomainError> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM ai_suggestions", [])
            .map_err(|e| DomainError::Internal(e.to_string()))
    }

    fn insert(&self, record: &AISuggestionRecord) -> Result<(), DomainError> {
        let payload_json = serde_json::to_string(&record.payload)
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let sources_json = serde_json::to_string(&record.sources)
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO ai_suggestions (
                id, feature_type, source_entity_type, source_entity_id, payload, sources_json,
                status, provider, model, created_at, decided_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id.to_string(),
                record.feature_type,
                record.source_entity_type,
                record.source_entity_id,
                payload_json,
                sources_json,
                record.status.as_str(),
                record.provider,
                record.model,
                record.created_at,
                record.decided_at,
            ],
        )
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    fn insert_invalid(
        &self,
        feature: AIFeature,
        source_entity_type: &str,
        source_entity_id: &str,
        settings: &crate::domain::AIConfig,
    ) -> Result<(), DomainError> {
        let record = AISuggestionRecord {
            id: new_id().to_string(),
            feature_type: feature.as_str().to_string(),
            source_entity_type: source_entity_type.to_string(),
            source_entity_id: source_entity_id.to_string(),
            payload: crate::domain::SuggestionContent {
                items: Vec::new(),
                summary: None,
            },
            sources: Vec::new(),
            status: SuggestionStatus::Dismissed,
            provider: settings.mode.as_str().to_string(),
            model: active_model_name(settings),
            created_at: stamp(&self.clock),
            decided_at: Some(stamp(&self.clock)),
        };
        self.insert(&record)
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db
            .connect()
            .map_err(|e| DomainError::Internal(e.to_string()))
    }
}

fn active_model_name(config: &crate::domain::AIConfig) -> String {
    match config.mode {
        crate::domain::AIMode::Ollama => config.ollama_model.clone(),
        crate::domain::AIMode::Custom => config.custom_model.clone(),
        crate::domain::AIMode::Off => String::new(),
    }
}

struct SuggestionRow {
    id: String,
    feature_type: String,
    source_entity_type: String,
    source_entity_id: String,
    payload_json: String,
    sources_json: String,
    status: String,
    provider: String,
    model: String,
    created_at: String,
    decided_at: Option<String>,
}

fn row_from_db(row: SuggestionRow) -> Result<AISuggestionRecord, DomainError> {
    let payload: crate::domain::SuggestionContent = serde_json::from_str(&row.payload_json)
        .map_err(|e| DomainError::Internal(format!("invalid payload json: {e}")))?;
    let sources: Vec<SuggestionSource> = serde_json::from_str(&row.sources_json)
        .map_err(|e| DomainError::Internal(format!("invalid sources json: {e}")))?;
    Ok(AISuggestionRecord {
        id: row.id,
        feature_type: row.feature_type,
        source_entity_type: row.source_entity_type,
        source_entity_id: row.source_entity_id,
        payload,
        sources,
        status: match row.status.as_str() {
            "accepted" => SuggestionStatus::Accepted,
            "rejected" => SuggestionStatus::Rejected,
            "dismissed" => SuggestionStatus::Dismissed,
            _ => SuggestionStatus::Pending,
        },
        provider: row.provider,
        model: row.model,
        created_at: row.created_at,
        decided_at: row.decided_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AIMode, AIFeatureToggles};
    use crate::infrastructure::db::Database;
        use tempfile::tempdir;

    struct FakeProvider {
        output: Option<&'static str>,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl AIProvider for FakeProvider {
        fn probe(&self) -> crate::domain::ProbeReport {
            crate::domain::ProbeReport {
                mode: AIMode::Ollama,
                reachable: true,
                model: Some("fake".into()),
                latency_ms: Some(1),
                hint: None,
            }
        }
        fn complete(&self, _request: &CompletionRequest) -> Option<crate::domain::CompletionOutput> {
            self.calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.output
                .map(|raw| crate::domain::CompletionOutput { raw_json: raw.into() })
        }
    }

    fn setup() -> (tempfile::TempDir, AISuggestionService, Arc<SettingsService>) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let settings = Arc::new(SettingsService::new(db.clone()));
        let service = AISuggestionService::with_provider(
            db,
            settings.clone(),
            Arc::new(FakeProvider {
                output: None,
                calls: std::sync::atomic::AtomicUsize::new(0),
            }),
        );
        (dir, service, settings)
    }

    fn enable_extract(settings: &SettingsService) {
        let mut s = settings.get().unwrap();
        s.ai.mode = AIMode::Ollama;
        s.ai.ollama_model = "fake".into();
        s.ai.features = AIFeatureToggles {
            extract: true,
            ..Default::default()
        };
        settings.save(&s).unwrap();
    }

    fn service_with_provider(
        dir: &std::path::Path,
        settings: &Arc<SettingsService>,
        output: Option<&'static str>,
    ) -> AISuggestionService {
        let provider = Arc::new(FakeProvider {
            output,
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        AISuggestionService::with_provider(
            Database::open(dir.join("workbench.db")).unwrap(),
            settings.clone(),
            provider,
        )
    }

    fn ctx(entity_type: &str, entity_id: &str, text: &str, source_app: Option<&str>) -> ContextItem {
        ContextItem {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            text: text.into(),
            source_app: source_app.map(str::to_string),
        }
    }

    fn insert_memory(conn: &Connection, id: &str, body: &str, sensitive: bool) {
        conn.execute(
            "INSERT INTO memories (id, title, body, pinned, archived, quick_insert, trigger_word,
                 mention_use_count, sensitive, created_at, updated_at, revision)
             VALUES (?1, 't', ?2, 0, 0, 0, NULL, 0, ?3, 't', 't', 1)",
            params![id, body, sensitive as i64],
        )
        .unwrap();
    }

    #[test]
    fn disabled_feature_never_calls_provider() {
        let (_dir, service, _settings) = setup(); // features all default off
        let result = service
            .request(AIFeature::Extract, "memory", "m1", &[ctx("memory", "m1", "开会记录", None)])
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn sensitive_memory_is_sanitized_out() {
        let (dir, service, settings) = setup();
        let conn = service.db.connect().unwrap();
        insert_memory(&conn, "m-s", "密码备份流程", true);
        insert_memory(&conn, "m-ok", "周会记录", false);
        drop(conn);

        let kept = service.sanitize_context(&[
            ctx("memory", "m-s", "密码备份流程", None),
            ctx("memory", "m-ok", "周会记录", None),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].entity_id, "m-ok");

        enable_extract(&settings);
        // All-sensitive context → sanitized empty → no provider call.
        let result = service
            .request(AIFeature::Extract, "memory", "m-s", &[ctx("memory", "m-s", "密码", None)])
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn password_manager_source_is_sanitized_out() {
        let (_dir, service, _settings) = setup();
        let kept = service.sanitize_context(&[
            ctx("clipboard", "c1", "复制的密码", Some("1Password")),
            ctx("clipboard", "c2", "普通文本", Some("Safari")),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].entity_id, "c2");
    }

    #[test]
    fn user_excluded_apps_are_sanitized_out() {
        let (_dir, service, settings) = setup();
        let mut s = settings.get().unwrap();
        s.clipboard_excluded_apps.push("微信".into());
        settings.save(&s).unwrap();

        let kept = service.sanitize_context(&[ctx("clipboard", "c1", "x", Some("微信"))]);
        assert!(kept.is_empty());
    }

    #[test]
    fn valid_output_lands_as_pending_with_sources() {
        let (dir, service, settings) = setup();
        enable_extract(&settings);
        let conn = Database::open(dir.path().join("workbench.db")).unwrap().connect().unwrap();
        insert_memory(&conn, "m-ok", "找老张确认合同", false);
        drop(conn);
        let service = service_with_provider(
            dir.path(),
            &settings,
            Some(
                r#"{"items":[{"title":"确认合同","detail":null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":"找老张确认合同"}],"summary":null}"#,
            ),
        );

        let record = service
            .request(AIFeature::Extract, "memory", "m-ok", &[ctx("memory", "m-ok", "找老张确认合同", None)])
            .unwrap()
            .expect("record");
        assert_eq!(record.status, SuggestionStatus::Pending);
        assert_eq!(record.payload.items.len(), 1);
        assert_eq!(record.sources.len(), 1);
        assert_eq!(record.sources[0].entity_id, "m-ok");
    }

    #[test]
    fn invalid_output_is_discarded_with_audit_row() {
        let (dir, service, settings) = setup();
        enable_extract(&settings);
        let conn = Database::open(dir.path().join("workbench.db")).unwrap().connect().unwrap();
        insert_memory(&conn, "m1", "x", false);
        drop(conn);
        let service = service_with_provider(dir.path(), &settings, Some("this is not json"));

        let result = service
            .request(AIFeature::Extract, "memory", "m1", &[ctx("memory", "m1", "x", None)])
            .unwrap();
        assert!(result.is_none()); // caller sees nothing

        let history = service.list(None, None).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, SuggestionStatus::Dismissed);
        assert!(history[0].payload.items.is_empty()); // no business payload
    }

    #[test]
    fn decide_transitions_once_and_clear_history() {
        let (dir, service, settings) = setup();
        enable_extract(&settings);
        let conn = Database::open(dir.path().join("workbench.db")).unwrap().connect().unwrap();
        insert_memory(&conn, "m1", "a", false);
        drop(conn);
        let service = service_with_provider(
            dir.path(),
            &settings,
            Some(
                r#"{"items":[{"title":"a","detail":null,"dueDate":null,"dueTime":null,"ambiguous":false,"sourceExcerpt":"a"}],"summary":null}"#,
            ),
        );
        let record = service
            .request(AIFeature::Extract, "memory", "m1", &[ctx("memory", "m1", "a", None)])
            .unwrap()
            .unwrap();

        let accepted = service
            .decide(&record.id.to_string(), SuggestionStatus::Accepted)
            .unwrap();
        assert_eq!(accepted.status, SuggestionStatus::Accepted);
        assert!(accepted.decided_at.is_some());

        // Terminal: second decide fails.
        assert!(service
            .decide(&record.id.to_string(), SuggestionStatus::Rejected)
            .is_err());

        assert_eq!(service.clear_history().unwrap(), 1);
        assert!(service.list(None, None).unwrap().is_empty());
    }

    #[test]
    fn off_mode_provider_returns_none_end_to_end() {
        // Even with the feature toggle accidentally on, OffProvider yields
        // no suggestion and no ledger row (gate 4 double lock).
        let (dir, _service, settings) = setup();
        enable_extract(&settings);
        let mut s = settings.get().unwrap();
        s.ai.mode = AIMode::Off;
        settings.save(&s).unwrap();
        let conn = Database::open(dir.path().join("workbench.db")).unwrap().connect().unwrap();
        insert_memory(&conn, "m1", "x", false);
        drop(conn);
        let service = AISuggestionService::with_provider(
            Database::open(dir.path().join("workbench.db")).unwrap(),
            settings.clone(),
            Arc::new(crate::infrastructure::ai::OffProvider),
        );
        let result = service
            .request(AIFeature::Extract, "memory", "m1", &[ctx("memory", "m1", "x", None)])
            .unwrap();
        assert!(result.is_none());
        assert!(service.list(None, None).unwrap().is_empty());
        assert!(!dir.path().join("ai_provider_key").exists());
    }
}
