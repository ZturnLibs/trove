//! Offline evaluation runner for the v2.0 AI service boundary (slice 1).
//!
//! Consumes `tests/fixtures/ai-eval/*.json` with no network and no model:
//! it regression-tests the deterministic parts of the pipeline that every
//! future slice inherits — sanitize red lines, structured-output
//! validation, off/degraded behavior, and key isolation.
//!
//! The `#[ignore]` online cases need a local Ollama and are run manually
//! before each AI feature slice: `cargo test --test ai_eval_offline -- --ignored`.

use std::sync::Arc;

use serde::Deserialize;
use tempfile::tempdir;
use trove_lib::application::ai_suggestions::AISuggestionService;
use trove_lib::domain::{
    parse_suggestion_content, AIFeature, AIFeatureToggles, AIMode, CompletionOutput,
    CompletionRequest, ContextItem, SuggestionStatus,
};
use trove_lib::infrastructure::ai::{
    build_provider, clear_provider_key, provider_key_exists, write_provider_key, AIProvider,
    OffProvider,
};
use trove_lib::infrastructure::db::Database;
use trove_lib::infrastructure::settings::SettingsService;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskSamples {
    samples: Vec<TaskSample>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskSample {
    text: String,
    #[serde(default)]
    source_app: Option<String>,
    #[serde(default)]
    sensitive_source: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DateSamples {
    samples: Vec<DateSample>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DateSample {
    text: String,
    #[serde(default)]
    ambiguous: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchPairs {
    pairs: Vec<SearchPair>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchPair {
    query: String,
    target_title: String,
    keyword_hit: bool,
}

fn load<T: for<'de> Deserialize<'de>>(name: &str) -> T {
    let path = format!("{}/tests/fixtures/ai-eval/{}", env!("CARGO_MANIFEST_DIR"), name);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

struct CountingProvider {
    output: Option<String>,
    calls: std::sync::atomic::AtomicUsize,
}

impl AIProvider for CountingProvider {
    fn probe(&self) -> trove_lib::domain::ProbeReport {
        trove_lib::domain::ProbeReport {
            mode: AIMode::Ollama,
            reachable: true,
            model: Some("fake".into()),
            latency_ms: Some(1),
            hint: None,
        }
    }
    fn complete(&self, _request: &CompletionRequest) -> Option<CompletionOutput> {
        self.calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.output
            .clone()
            .map(|raw_json| CompletionOutput { raw_json })
    }
}

fn setup_service() -> (
    tempfile::TempDir,
    AISuggestionService,
    Arc<SettingsService>,
    Arc<CountingProvider>,
) {
    let dir = tempdir().unwrap();
    let db = Database::open(dir.path().join("workbench.db")).unwrap();
    let settings = Arc::new(SettingsService::new(db.clone()));
    let provider = Arc::new(CountingProvider {
        output: None,
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    let service = AISuggestionService::with_provider(db, settings.clone(), provider.clone());
    (dir, service, settings, provider)
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

fn ctx(entity_type: &str, entity_id: &str, text: &str, source_app: Option<&str>) -> ContextItem {
    ContextItem {
        entity_type: entity_type.into(),
        entity_id: entity_id.into(),
        text: text.into(),
        source_app: source_app.map(str::to_string),
    }
}

// ---------------------------------------------------------------------------
// Red-line regression: sanitize
// ---------------------------------------------------------------------------

#[test]
fn eval_sanitize_filters_sensitive_sources_from_fixtures() {
    let (_dir, service, _settings, _provider) = setup_service();
    let tasks: TaskSamples = load("extract_tasks.json");

    let items: Vec<ContextItem> = tasks
        .samples
        .iter()
        .enumerate()
        .map(|(i, s)| ctx("clipboard", &format!("c{i}"), &s.text, s.source_app.as_deref()))
        .collect();

    let kept = service.sanitize_context(&items);
    for (i, sample) in tasks.samples.iter().enumerate() {
        let kept_i = kept.iter().any(|c| c.entity_id == format!("c{i}"));
        if sample.sensitive_source {
            assert!(!kept_i, "sensitive sample #{i} must be sanitized out");
        }
    }
    // Non-sensitive samples survive.
    let normal = tasks.samples.iter().enumerate().filter(|(_, s)| !s.sensitive_source).count();
    assert_eq!(kept.len(), normal);
}

#[test]
fn eval_sanitize_drops_sensitive_memory_rows() {
    let (dir, service, _settings, _provider) = setup_service();
    let conn = Database::open(dir.path().join("workbench.db")).unwrap().connect().unwrap();
    conn.execute(
        "INSERT INTO memories (id, title, body, pinned, archived, quick_insert, trigger_word,
             mention_use_count, sensitive, created_at, updated_at, revision)
         VALUES ('m-s', 't', 'SENSITIVE-REDACTION-SAMPLE', 0, 0, 0, NULL, 0, 1, 't', 't', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO memories (id, title, body, pinned, archived, quick_insert, trigger_word,
             mention_use_count, sensitive, created_at, updated_at, revision)
         VALUES ('m-ok', 't', '周会记录', 0, 0, 0, NULL, 0, 0, 't', 't', 1)",
        [],
    )
    .unwrap();
    drop(conn);

    let kept = service.sanitize_context(&[
        ctx("memory", "m-s", "SENSITIVE-REDACTION-SAMPLE", None),
        ctx("memory", "m-ok", "周会记录", None),
    ]);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].entity_id, "m-ok");
}

// ---------------------------------------------------------------------------
// Structured-output validation over fixture texts
// ---------------------------------------------------------------------------

#[test]
fn eval_schema_accepts_wellformed_and_rejects_bad_outputs() {
    let dates: DateSamples = load("extract_dates.json");
    let tasks: TaskSamples = load("extract_tasks.json");
    assert!(dates.samples.len() >= 10, "date fixtures ≥ 10");
    assert!(tasks.samples.len() >= 10, "task fixtures ≥ 10");

    // Every date fixture round-trips through the schema with its ambiguity
    // expectation honored.
    for sample in &dates.samples {
        let raw = format!(
            r#"{{"items":[{{"title":"任务","detail":null,"dueDate":null,"dueTime":null,"ambiguous":{},"sourceExcerpt":{}}}], "summary":null}}"#,
            sample.ambiguous,
            serde_json::to_string(&sample.text).unwrap()
        );
        let content = parse_suggestion_content(&raw)
            .unwrap_or_else(|e| panic!("fixture {:?} should parse: {e}", sample.text));
        assert_eq!(content.items[0].ambiguous, sample.ambiguous);
    }

    // Malformed outputs never pass.
    for bad in [
        "not json",
        r#"{"items":[],"summary":null}"#,
        r#"{"items":[{"title":"","detail":null,"dueDate":null,"dueTime":null,"ambiguous":false,"sourceExcerpt":"x"}],"summary":null}"#,
        r#"{"items":[{"title":"x","detail":null,"dueDate":"maybe","dueTime":null,"ambiguous":false,"sourceExcerpt":"x"}],"summary":null}"#,
    ] {
        assert!(parse_suggestion_content(bad).is_err(), "should reject: {bad}");
    }
}

// ---------------------------------------------------------------------------
// Degraded paths: off / disabled / unreachable
// ---------------------------------------------------------------------------

#[test]
fn eval_disabled_feature_makes_zero_provider_calls() {
    let (_dir, service, _settings, provider) = setup_service(); // all toggles off
    let result = service
        .request(
            AIFeature::Extract,
            "memory",
            "m1",
            &[ctx("task", "t1", "会议记录文本", None)],
        )
        .unwrap();
    assert!(result.is_none());
    assert_eq!(
        provider.calls.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "disabled feature must not touch the provider"
    );
}

#[test]
fn eval_off_provider_end_to_end_writes_nothing() {
    let (dir, _service, settings, _provider) = setup_service();
    enable_extract(&settings);
    let mut s = settings.get().unwrap();
    s.ai.mode = AIMode::Off;
    settings.save(&s).unwrap();

    let service = AISuggestionService::with_provider(
        Database::open(dir.path().join("workbench.db")).unwrap(),
        settings.clone(),
        Arc::new(OffProvider),
    );
    let result = service
        .request(
            AIFeature::Extract,
            "memory",
            "m1",
            &[ctx("task", "t1", "会议记录文本", None)],
        )
        .unwrap();
    assert!(result.is_none());
    assert!(service.list(None, None).unwrap().is_empty());
}

#[test]
fn eval_invalid_output_is_discarded_with_audit_only() {
    let (dir, _service, settings, _provider) = setup_service();
    enable_extract(&settings);
    let provider = Arc::new(CountingProvider {
        output: Some("definitely not json".into()),
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    let service = AISuggestionService::with_provider(
        Database::open(dir.path().join("workbench.db")).unwrap(),
        settings.clone(),
        provider,
    );

    let conn = Database::open(dir.path().join("workbench.db")).unwrap().connect().unwrap();
    conn.execute(
        "INSERT INTO memories (id, title, body, pinned, archived, quick_insert, trigger_word,
             mention_use_count, sensitive, created_at, updated_at, revision)
         VALUES ('m1', 't', 'b', 0, 0, 0, NULL, 0, 0, 't', 't', 1)",
        [],
    )
    .unwrap();
    drop(conn);

    let result = service
        .request(AIFeature::Extract, "memory", "m1", &[ctx("memory", "m1", "正文", None)])
        .unwrap();
    assert!(result.is_none());
    let history = service.list(None, None).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, SuggestionStatus::Dismissed);
    assert!(history[0].payload.items.is_empty());
}

// ---------------------------------------------------------------------------
// Key isolation
// ---------------------------------------------------------------------------

#[test]
fn eval_provider_key_stays_outside_database_and_settings() {
    let (dir, _service, settings, _provider) = setup_service();
    write_provider_key(dir.path(), "sk-eval-secret").unwrap();
    assert!(provider_key_exists(dir.path()));

    enable_extract(&settings);
    let raw = settings.get_raw("app.settings").unwrap().expect("settings json");
    let serialized = serde_json::to_string(&raw).unwrap();
    assert!(
        !serialized.contains("sk-eval-secret"),
        "key must never appear inside the settings payload"
    );

    clear_provider_key(dir.path()).unwrap();
    assert!(!provider_key_exists(dir.path()));

    // And the off-mode provider never reads it anyway.
    let provider = build_provider(&settings.get().unwrap().ai, dir.path());
    assert!(!provider.probe().reachable);
}

// ---------------------------------------------------------------------------
// Semantic-search fixture sanity (consumed by the vector-index slice)
// ---------------------------------------------------------------------------

#[test]
fn eval_semantic_search_fixtures_are_loaded_and_non_keyword() {
    let pairs: SearchPairs = load("search_semantics.json");
    assert!(pairs.pairs.len() >= 10);
    // Keyword search cannot hit these by construction; record the contract.
    for pair in &pairs.pairs {
        assert!(!pair.keyword_hit, "fixture pairs must be non-keyword");
        assert_ne!(pair.query, pair.target_title);
    }
}

// ---------------------------------------------------------------------------
// Online (ignored): requires a local Ollama. Run manually before slices
// that ship model-backed features: cargo test --test ai_eval_offline -- --ignored
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires local Ollama (OLLAMA_URL, default http://localhost:11434)"]
fn online_extract_pipeline_against_ollama() {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let model = std::env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL (e.g. qwen3:4b)");

    let dir = tempdir().unwrap();
    let mut config = trove_lib::domain::AIConfig::default();
    config.mode = AIMode::Ollama;
    config.ollama_url = url;
    config.ollama_model = model;

    let provider = build_provider(&config, dir.path());
    let probe = provider.probe();
    assert!(probe.reachable, "Ollama not reachable: {:?}", probe.hint);

    let tasks: TaskSamples = load("extract_tasks.json");
    let sample = tasks.samples.iter().find(|s| !s.sensitive_source).unwrap();
    let request = CompletionRequest::new(
        AIFeature::Extract.prompt_template().unwrap(),
        &sample.text,
    );
    let output = provider
        .complete(&request)
        .expect("completion succeeded");
    let content =
        parse_suggestion_content(&output.raw_json).expect("structured output validates");
    assert!(
        !content.items.is_empty() || content.summary.is_some(),
        "model returned usable content"
    );
}
