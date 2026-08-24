//! Rebuildable semantic (vector) index (v2.0 slice 8, §9.3).
//!
//! Contract:
//! - Derived data only: excluded from the JSON export whitelist, safe to
//!   clear/rebuild at any time (§9.4 "关闭 AI 后可删除本地语义索引").
//! - Rebuild applies user exclusions (lists / tags / clipboard types) and a
//!   hard row cap; keyword search is never affected.
//! - Search degrades to empty on any failure (off, model mismatch, …).

use rusqlite::Connection;

use crate::domain::{
    stamp, AIConfig, AIMode, DomainError, SemanticHit, SemanticIndexStatus, SystemClock,
};
use crate::infrastructure::ai::{build_provider, AIProvider};
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;

use std::path::PathBuf;
use std::sync::Arc;

/// Hard cap on indexed rows (memory + rebuild time bound).
pub const SEMANTIC_INDEX_MAX_ROWS: i64 = 20_000;
/// Minimum cosine similarity to surface a hit.
pub const SEMANTIC_MIN_SCORE: f32 = 0.35;

thread_local! {
    static PROVIDER_OVERRIDE: std::cell::RefCell<Option<std::sync::Arc<dyn AIProvider>>> =
        const { std::cell::RefCell::new(None) };
}

/// Arc-backed provider adapter so the test override can be reused across
/// multiple service calls on one thread.
struct ProviderArc(std::sync::Arc<dyn AIProvider>);

impl AIProvider for ProviderArc {
    fn probe(&self) -> crate::domain::ProbeReport {
        self.0.probe()
    }
    fn complete(&self, r: &crate::domain::CompletionRequest) -> Option<crate::domain::CompletionOutput> {
        self.0.complete(r)
    }
    fn embed(&self, texts: &[&str]) -> Option<Vec<Vec<f32>>> {
        self.0.embed(texts)
    }
}

pub struct SemanticIndexService {
    db: Database,
    settings: Arc<SettingsService>,
    data_dir: PathBuf,
    clock: SystemClock,
}

impl SemanticIndexService {
    pub fn new(db: Database, settings: Arc<SettingsService>, data_dir: PathBuf) -> Self {
        Self {
            db,
            settings,
            data_dir,
            clock: SystemClock,
        }
    }

    /// Test constructor with a provider fake (embedding path).
    pub fn with_provider(
        db: Database,
        settings: Arc<SettingsService>,
        data_dir: PathBuf,
        provider: Box<dyn AIProvider>,
    ) -> Self {
        let service = Self::new(db, settings, data_dir);
        service.set_provider_override(provider);
        service
    }

    fn set_provider_override(&self, provider: Box<dyn AIProvider>) {
        PROVIDER_OVERRIDE
            .with(|cell| *cell.borrow_mut() = Some(std::sync::Arc::from(provider)));
    }

    /// Test-only: clone the inner database handle for corpus rebuilds.
    #[cfg(test)]
    pub fn db_for_test(&self) -> Database {
        self.db.clone()
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db
            .connect()
            .map_err(|e| DomainError::Internal(e.to_string()))
    }

    fn config(&self) -> Result<AIConfig, DomainError> {
        Ok(self.settings.get()?.ai)
    }

    fn embedding_model(&self, config: &AIConfig) -> String {
        config.embedding_model.trim().to_string()
    }

    fn provider_for_embed(&self, config: &AIConfig) -> Box<dyn AIProvider> {
        // Tests inject a deterministic provider via the thread-local override
        // (persisted across calls — no take()).
        if let Some(provider) = PROVIDER_OVERRIDE.with(|cell| cell.borrow().clone()) {
            return Box::new(ProviderArc(provider));
        }
        build_provider(config, &self.data_dir)
    }

    /// Full rebuild: read the search corpus (with exclusions), embed in
    /// batches, replace the table. Idempotent: two runs produce the same
    /// row set (content-dependent embeddings aside).
    pub fn rebuild(&self) -> Result<SemanticIndexStatus, DomainError> {
        let config = self.config()?;
        let model = self.embedding_model(&config);
        if config.mode == AIMode::Off || model.is_empty() {
            return Err(DomainError::Validation(
                "请先在设置中选择 AI 模式与 embedding 模型".into(),
            ));
        }

        let conn = self.connect()?;
        let corpus = load_corpus(&conn, &config.semantic_exclusions, SEMANTIC_INDEX_MAX_ROWS)?;
        if corpus.is_empty() {
            conn.execute("DELETE FROM semantic_index", [])
                .map_err(|e| DomainError::Internal(e.to_string()))?;
            return self.status();
        }

        let texts: Vec<&str> = corpus.iter().map(|c| c.text.as_str()).collect();
        let provider = self.provider_for_embed(&config);
        let embeddings = provider.embed(&texts).ok_or_else(|| {
            DomainError::Validation("embedding 服务不可用，请检查模型与连接".into())
        })?;
        if embeddings.len() != corpus.len() {
            return Err(DomainError::Internal("embedding 数量不匹配".into()));
        }

        let now = stamp(&self.clock);
        // Replace-all semantics: previous model's rows never linger.
        conn.execute("DELETE FROM semantic_index", [])
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "INSERT INTO semantic_index (entity_type, entity_id, embedding, model, dims, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        for (row, embedding) in corpus.iter().zip(embeddings.iter()) {
            if embedding.is_empty() {
                continue;
            }
            let bytes: Vec<u8> = embedding
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            stmt.execute(rusqlite::params![
                row.entity_type,
                row.entity_id,
                bytes,
                model.clone(),
                embedding.len() as i64,
                now,
            ])
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        }
        drop(stmt);
        self.status()
    }

    /// Semantic search: cosine Top-K over the in-table vectors. Degrades to
    /// an empty list when disabled, mismatched, or on any failure.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SemanticHit>, DomainError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let config = self.config()?;
        if config.mode == AIMode::Off
            || !config.features.semantic_search
            || self.embedding_model(&config).is_empty()
        {
            return Ok(Vec::new());
        }

        let conn = self.connect()?;
        let indexed_model: Option<String> = conn
            .query_row(
                "SELECT model FROM semantic_index LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();
        let model = self.embedding_model(&config);
        if indexed_model.as_deref() != Some(model.as_str()) {
            return Ok(Vec::new()); // mismatch → rebuild needed, degrade silently
        }

        let provider = self.provider_for_embed(&config);
        let Some(mut query_vec) = provider.embed(&[trimmed]).and_then(|v| v.into_iter().next())
        else {
            return Ok(Vec::new());
        };
        let norm = vector_norm(&query_vec);
        if norm == 0.0 {
            return Ok(Vec::new());
        }
        for f in query_vec.iter_mut() {
            *f /= norm;
        }

        // Stream rows; keep the best-K by score.
        let mut stmt = conn
            .prepare(
                "SELECT s.entity_type, s.entity_id, s.embedding, COALESCE(d.title, s.entity_id)
                 FROM semantic_index s
                 LEFT JOIN search_documents d
                   ON d.entity_type = s.entity_type AND d.entity_id = s.entity_id",
            )
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let mut best: Vec<(f32, SemanticHit)> = Vec::new();
        for row in rows {
            let (entity_type, entity_id, bytes, title) =
                row.map_err(|e| DomainError::Internal(e.to_string()))?;
            let score = cosine_with_normalized(&bytes, &query_vec);
            if score < SEMANTIC_MIN_SCORE {
                continue;
            }
            best.push((
                score,
                SemanticHit {
                    entity_type,
                    entity_id,
                    title,
                    score,
                    matched_type: "semantic".into(),
                },
            ));
            // Bound the heap manually: sort-once at the end is fine for ≤20k.
        }
        best.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(best
            .into_iter()
            .take(limit.max(1))
            .map(|(_, hit)| hit)
            .collect())
    }

    pub fn status(&self) -> Result<SemanticIndexStatus, DomainError> {
        let config = self.config()?;
        let conn = self.connect()?;
        let (rows, model, last): (i64, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT COUNT(*), MIN(model), MAX(indexed_at) FROM semantic_index",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let eligible = eligible_corpus_count(&conn, &config.semantic_exclusions)?;
        let configured = self.embedding_model(&config);
        let configured_opt = if configured.is_empty() { None } else { Some(configured.clone()) };
        Ok(SemanticIndexStatus {
            rows,
            model: model.clone(),
            last_indexed_at: last,
            eligible,
            capped: eligible > SEMANTIC_INDEX_MAX_ROWS,
            configured_model: configured_opt.clone(),
            model_mismatch: model.is_some()
                && configured_opt.is_some()
                && model.as_deref() != configured_opt.as_deref(),
        })
    }

    /// §9.4: clearing the index never touches business data.
    pub fn clear(&self) -> Result<(), DomainError> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM semantic_index", [])
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }
}

struct CorpusRow {
    entity_type: String,
    entity_id: String,
    text: String,
}

fn load_corpus(
    conn: &Connection,
    exclusions: &crate::domain::SemanticExclusions,
    cap: i64,
) -> Result<Vec<CorpusRow>, DomainError> {
    let mut sql = String::from(
        "SELECT entity_type, entity_id, title, body FROM search_documents
         WHERE entity_type IN ('task','memory','clipboard')",
    );
    let mut params: Vec<String> = Vec::new();
    if !exclusions.list_ids.is_empty() {
        // Task rows: exclude tasks belonging to excluded lists.
        sql.push_str(&format!(
            " AND NOT (entity_type = 'task' AND entity_id IN (SELECT t.id FROM tasks t WHERE t.list_id IN ({})))",
            placeholders(exclusions.list_ids.len())
        ));
        params.extend(exclusions.list_ids.iter().cloned());
    }
    if !exclusions.tag_ids.is_empty() {
        sql.push_str(&format!(
            " AND NOT (entity_type = 'memory' AND entity_id IN (SELECT mt.memory_id FROM memory_tags mt WHERE mt.tag_id IN ({})))",
            placeholders(exclusions.tag_ids.len())
        ));
        params.extend(exclusions.tag_ids.iter().cloned());
    }
    if !exclusions.clipboard_types.is_empty() {
        sql.push_str(&format!(
            " AND NOT (entity_type = 'clipboard' AND entity_id IN (SELECT c.id FROM clipboard_items c WHERE c.kind IN ({})))",
            placeholders(exclusions.clipboard_types.len())
        ));
        params.extend(exclusions.clipboard_types.iter().cloned());
    }
    sql.push_str(" ORDER BY updated_at DESC LIMIT ?");
    params.push(cap.to_string());

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(CorpusRow {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                text: format!("{}\n{}", row.get::<_, String>(2)?, row.get::<_, String>(3)?),
            })
        })
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| DomainError::Internal(e.to_string()))?);
    }
    Ok(out)
}

fn eligible_corpus_count(
    conn: &Connection,
    exclusions: &crate::domain::SemanticExclusions,
) -> Result<i64, DomainError> {
    // Cheap approximation: total corpus; the cap logic is applied at load.
    let _ = exclusions;
    conn.query_row(
        "SELECT COUNT(*) FROM search_documents WHERE entity_type IN ('task','memory','clipboard')",
        [],
        |row| row.get(0),
    )
    .map_err(|e| DomainError::Internal(e.to_string()))
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

fn vector_norm(v: &[f32]) -> f32 {
    v.iter().map(|f| f * f).sum::<f32>().sqrt()
}

fn cosine_with_normalized(bytes: &[u8], normalized_query: &[f32]) -> f32 {
    let dims = bytes.len() / 4;
    if dims == 0 || dims != normalized_query.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm = 0.0f32;
    for i in 0..dims {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&bytes[i * 4..i * 4 + 4]);
        let f = f32::from_le_bytes(buf);
        dot += f * normalized_query[i];
        norm += f * f;
    }
    let denom = norm.sqrt();
    if denom == 0.0 {
        return 0.0;
    }
    dot / denom
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CreateTaskInput, SemanticExclusions};
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    struct FixedEmbedProvider;

    impl AIProvider for FixedEmbedProvider {
        fn probe(&self) -> crate::domain::ProbeReport {
            crate::domain::ProbeReport {
                mode: AIMode::Ollama,
                reachable: true,
                model: Some("fake-embed".into()),
                latency_ms: Some(1),
                hint: None,
            }
        }
        fn complete(&self, _r: &crate::domain::CompletionRequest) -> Option<crate::domain::CompletionOutput> {
            None
        }
        // Deterministic embeddings keyed on the first char: texts starting
        // with the same character map to the same unit vector, so equal-ish
        // content scores 1.0 and unrelated content is orthogonal.
        fn embed(&self, texts: &[&str]) -> Option<Vec<Vec<f32>>> {
            Some(
                texts
                    .iter()
                    .map(|t| {
                        let mut v = vec![0.0f32; 8];
                        let idx = t
                            .chars()
                            .next()
                            .map(|c| (c as usize) % 8)
                            .unwrap_or(0);
                        v[idx] = 1.0;
                        v
                    })
                    .collect(),
            )
        }
    }

    fn setup_with_provider(
        provider: Box<dyn AIProvider>,
    ) -> (
        tempfile::TempDir,
        SemanticIndexService,
        Arc<SettingsService>,
        crate::application::tasks::TaskService,
    ) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("t.db")).unwrap();
        let settings = Arc::new(SettingsService::new(db.clone()));
        let base = settings.get().unwrap();
        let next = crate::infrastructure::settings::AppSettings {
            ai: AIConfig {
                mode: AIMode::Ollama,
                embedding_model: "fake-embed".into(),
                features: crate::domain::AIFeatureToggles {
                    semantic_search: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..base
        };
        settings.save(&next).unwrap();
        let tasks = crate::application::tasks::TaskService::new(db.clone());
        tasks.ensure_seed_data().unwrap();
        // Populate the search corpus the way app startup does.
        crate::application::search::SearchService::new(db.clone())
            .rebuild_all()
            .unwrap();

        let service = SemanticIndexService::with_provider(db, settings.clone(), dir.path().into(), provider);
        (dir, service, settings, tasks)
    }

    #[test]
    fn rebuild_is_idempotent_and_clear_empties() {
        let (_dir, service, _settings, tasks) = setup_with_provider(Box::new(FixedEmbedProvider));
        tasks
            .create_task(CreateTaskInput {
                title: "差旅票据整理".into(),
                notes: Some("报销流程".into()),
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        crate::application::search::SearchService::new(service.db_for_test()).rebuild_all().unwrap();
        let first = service.rebuild().unwrap();
        assert!(first.rows > 0);
        let second = service.rebuild().unwrap();
        assert_eq!(first.rows, second.rows, "rebuild is replace-all + idempotent");
        assert_eq!(second.model.as_deref(), Some("fake-embed"));

        service.clear().unwrap();
        assert_eq!(service.status().unwrap().rows, 0);
    }

    #[test]
    fn search_finds_semantically_equal_text_and_degrades_when_off() {
        let (_dir, service, _settings, tasks) = setup_with_provider(Box::new(FixedEmbedProvider));
        tasks
            .create_task(CreateTaskInput {
                title: "差旅票据整理".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        crate::application::search::SearchService::new(service.db_for_test()).rebuild_all().unwrap();
        service.rebuild().unwrap();

        // Exact same text → cosine 1.0 → hit.
        let hits = service.search("差旅票据整理", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].score > 0.99);
        assert_eq!(hits[0].entity_type, "task");

        // Disabling the feature degrades to empty (no provider call needed).
        let settings2 = _settings;
        let mut s = settings2.get().unwrap();
        s.ai.features.semantic_search = false;
        settings2.save(&s).unwrap();
        assert!(service.search("差旅票据整理", 5).unwrap().is_empty());
    }

    #[test]
    fn exclusions_keep_list_out_of_index() {
        let (_dir, service, settings, tasks) = setup_with_provider(Box::new(FixedEmbedProvider));
        let task = tasks
            .create_task(CreateTaskInput {
                title: "排除我".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let list_id = tasks.get_task(task.id).unwrap().list_id.to_string();

        let mut s = settings.get().unwrap();
        s.ai.semantic_exclusions = SemanticExclusions {
            list_ids: vec![list_id],
            ..Default::default()
        };
        settings.save(&s).unwrap();

        crate::application::search::SearchService::new(service.db_for_test()).rebuild_all().unwrap();
        service.rebuild().unwrap();
        let hits = service.search("排除我", 5).unwrap();
        assert!(
            hits.iter().all(|h| h.entity_id != task.id.to_string()),
            "excluded list task must not surface"
        );
    }

    #[test]
    fn model_mismatch_degrades_to_empty() {
        let (_dir, service, settings, tasks) = setup_with_provider(Box::new(FixedEmbedProvider));
        tasks
            .create_task(CreateTaskInput {
                title: "任意任务".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        crate::application::search::SearchService::new(service.db_for_test()).rebuild_all().unwrap();
        service.rebuild().unwrap();

        let mut s = settings.get().unwrap();
        s.ai.embedding_model = "another-model".into();
        settings.save(&s).unwrap();

        assert_eq!(service.status().unwrap().model_mismatch, true);
        assert!(service.search("任意", 5).unwrap().is_empty());
    }


    #[test]
    fn rebuild_without_model_is_rejected() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("t.db")).unwrap();
        let settings = Arc::new(SettingsService::new(db.clone()));
        let service =
            SemanticIndexService::with_provider(db, settings, dir.path().into(), Box::new(FixedEmbedProvider));
        assert!(service.rebuild().is_err(), "off mode + no model → validation error");
    }
}
