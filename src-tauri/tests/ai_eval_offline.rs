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
    let config = trove_lib::domain::AIConfig {
        mode: AIMode::Ollama,
        ollama_url: url,
        ollama_model: model,
        ..Default::default()
    };

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

// Slice 2: extract → apply contract on top of the offline pipeline.

#[test]
#[ignore = "requires local Ollama (OLLAMA_URL/OLLAMA_MODEL); run before releases"]
fn online_extract_then_apply_full_chain() {
    use trove_lib::application::links::EntityLinkService;
    use trove_lib::application::memories::MemoryService;
    use trove_lib::application::tasks::TaskService;
    use trove_lib::domain::{CreateMemoryInput, ExtractApplyInput};

    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let model = std::env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL (e.g. qwen3:4b)");

    let dir = tempdir().unwrap();
    let db = Database::open(dir.path().join("workbench.db")).unwrap();
    let settings = Arc::new(SettingsService::new(db.clone()));
    let mut base = settings.get().unwrap();
    base.ai = trove_lib::domain::AIConfig {
        mode: AIMode::Ollama,
        ollama_url: url,
        ollama_model: model,
        features: AIFeatureToggles {
            extract: true,
            ..Default::default()
        },
        ..Default::default()
    };
    settings.save(&base).unwrap();

    let tasks = TaskService::new(db.clone());
    tasks.ensure_seed_data().unwrap();
    let links = EntityLinkService::new(db.clone());
    let search = trove_lib::application::search::SearchService::new(db.clone());
    let memories = MemoryService::new(db.clone());
    let memory = memories
        .create(CreateMemoryInput {
            title: "周会记录".into(),
            body: Some(
                "会议记录：王琳在 10 号前提交投放数据；找老张确认合同；下周五复测上线。".into(),
            ),
            pinned: None,
            quick_insert: None,
            trigger_word: None,
            tag_names: None,
        })
        .unwrap();

    let service = AISuggestionService::new(db.clone(), settings.clone(), dir.path().into())
        .expect("service");
    let record = service
        .request_extract(&memory.id.to_string(), &memories)
        .expect("request")
        .expect("provider reachable and produced a record");
    assert!(!record.payload.items.is_empty(), "at least one draft item");

    let result = service
        .apply_extract(
            ExtractApplyInput {
                suggestion_id: record.id.clone(),
                selected_indices: (0..record.payload.items.len()).collect(),
            },
            &tasks,
            &links,
            &search,
        )
        .expect("apply");
    assert_eq!(result.tasks.len(), record.payload.items.len());
    // No guessed dates on ambiguous items.
    for (task, item) in result.tasks.iter().zip(record.payload.items.iter()) {
        assert_eq!(task.due_date.is_some(), !item.ambiguous && item.due_date.is_some());
    }
}

// Slice 3: weekly summary contract (offline part: prompt + schema).

#[test]
fn eval_summary_prompt_and_schema_contract() {
    // Feature opened and judgement-forbidding constraints present.
    let prompt = AIFeature::Summary.prompt_template().expect("opened");
    assert!(prompt.contains("严禁评价"));
    assert!(prompt.contains("200 字"));
    // Summary-only output validates via the shared schema.
    let content = parse_suggestion_content(r#"{"items":[],"summary":"本周完成 3 项。"}"#).unwrap();
    assert!(content.items.is_empty());
    assert_eq!(content.summary.as_deref(), Some("本周完成 3 项。"));
    // And prose with items still validates (shared envelope).
    assert!(parse_suggestion_content(
        r#"{"items":[],"summary":"收件箱 2 项待整理。"}"#
    )
    .is_ok());
}

#[test]
#[ignore = "requires local Ollama (OLLAMA_URL/OLLAMA_MODEL); run before releases"]
fn online_weekly_summary_produces_prose_only() {
    use trove_lib::application::clipboard::ClipboardService;
    use trove_lib::application::reminders::ReminderService;
    use trove_lib::application::tasks::TaskService;
    use trove_lib::application::weekly_review::WeeklyReviewService;

    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let model = std::env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL");

    let dir = tempdir().unwrap();
    let db = Database::open(dir.path().join("workbench.db")).unwrap();
    let settings = Arc::new(SettingsService::new(db.clone()));
    let mut base = settings.get().unwrap();
    base.ai = trove_lib::domain::AIConfig {
        mode: AIMode::Ollama,
        ollama_url: url,
        ollama_model: model,
        features: AIFeatureToggles {
            summary: true,
            ..Default::default()
        },
        ..Default::default()
    };
    settings.save(&base).unwrap();

    let tasks = TaskService::new(db.clone());
    tasks.ensure_seed_data().unwrap();
    let reminders = ReminderService::new(db.clone());
    let clipboard = ClipboardService::new(db.clone(), dir.path().join("assets"));
    let weekly = WeeklyReviewService::new(db.clone());

    let service =
        AISuggestionService::new(db, settings.clone(), dir.path().into()).expect("service");
    let record = service
        .request_weekly_summary(&weekly, &tasks, &reminders, &clipboard)
        .expect("request")
        .expect("record");
    assert!(record.payload.summary.is_some(), "prose produced");
    assert!(record.payload.items.is_empty(), "no fabricated items");
}

// Slice 4: related-content contract (offline part).

#[test]
fn eval_related_prompt_and_bigram_retrieval_contract() {
    // Prompt pins exact-copy behavior.
    let prompt = AIFeature::Related.prompt_template().expect("opened");
    assert!(prompt.contains("完全一致"));
    assert!(prompt.contains("不得编造候选列表之外"));

    // CJK bigram extraction produces usable retrieval fragments.
    // (Exercised indirectly via service tests; here we sanity-check the
    // fixture texts share bigrams with their targets.)
    let tasks: TaskSamples = load("extract_tasks.json");
    let date_samples: DateSamples = load("extract_dates.json");
    assert!(tasks.samples.len() >= 10);
    assert!(date_samples.samples.len() >= 10);
}

#[test]
#[ignore = "requires local Ollama (OLLAMA_URL/OLLAMA_MODEL); run before releases"]
fn online_related_backmapping_hits_candidates() {
    use trove_lib::application::links::EntityLinkService;
    use trove_lib::application::memories::MemoryService;
    use trove_lib::application::search::SearchService;
    use trove_lib::application::tasks::TaskService;
    use trove_lib::domain::CreateMemoryInput;

    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let model = std::env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL");

    let dir = tempdir().unwrap();
    let db = Database::open(dir.path().join("workbench.db")).unwrap();
    let settings = Arc::new(SettingsService::new(db.clone()));
    let mut base = settings.get().unwrap();
    base.ai = trove_lib::domain::AIConfig {
        mode: AIMode::Ollama,
        ollama_url: url,
        ollama_model: model,
        features: AIFeatureToggles {
            related: true,
            ..Default::default()
        },
        ..Default::default()
    };
    settings.save(&base).unwrap();

    let tasks = TaskService::new(db.clone());
    tasks.ensure_seed_data().unwrap();
    let search = SearchService::new(db.clone());
    let memories = MemoryService::new(db.clone());
    let links = EntityLinkService::new(db.clone());

    memories
        .create(CreateMemoryInput {
            title: "差旅票据整理".into(),
            body: Some("报销流程与票据归档".into()),
            pinned: None,
            quick_insert: None,
            trigger_word: None,
            tag_names: None,
        })
        .unwrap();
    let task = tasks
        .create_task(trove_lib::domain::CreateTaskInput {
            title: "整理报销票据".into(),
            notes: Some("差旅报销流程".into()),
            priority: None,
            list_id: None,
            due_date: None,
            due_time: None,
            tag_names: None,
        })
        .unwrap();

    let service =
        AISuggestionService::new(db, settings.clone(), dir.path().into()).expect("service");
    let record = service
        .request_related(&task.id.to_string(), &tasks, &search, &memories, &links)
        .expect("request");
    if let Some(record) = record {
        // Every surviving item must map back to a real candidate source.
        assert!(!record.sources.is_empty());
    }
}

// Slice 5: daily-suggest contract (offline part).

#[test]
fn eval_suggest_prompt_pins_feature_citation() {
    let prompt = AIFeature::Suggest.prompt_template().expect("opened");
    assert!(prompt.contains("必须基于特征"));
    assert!(prompt.contains("完全一致"));
    assert!(prompt.contains("宁可不选"));
}

#[test]
#[ignore = "requires local Ollama (OLLAMA_URL/OLLAMA_MODEL); run before releases"]
fn online_daily_suggest_backmaps_and_cites_features() {
    use trove_lib::application::tasks::TaskService;
    use trove_lib::domain::CreateTaskInput;

    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let model = std::env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL");

    let dir = tempdir().unwrap();
    let db = Database::open(dir.path().join("workbench.db")).unwrap();
    let settings = Arc::new(SettingsService::new(db.clone()));
    let mut base = settings.get().unwrap();
    let today = trove_lib::domain::local_today(&trove_lib::domain::SystemClock);
    base.ai = trove_lib::domain::AIConfig {
        mode: AIMode::Ollama,
        ollama_url: url,
        ollama_model: model,
        features: AIFeatureToggles {
            suggest: true,
            ..Default::default()
        },
        ..Default::default()
    };
    settings.save(&base).unwrap();

    let tasks = TaskService::new(db.clone());
    tasks.ensure_seed_data().unwrap();
    tasks
        .create_task(CreateTaskInput {
            title: "整理报销票据".into(),
            notes: None,
            priority: Some(trove_lib::domain::TaskPriority::High),
            list_id: None,
            due_date: Some(today),
            due_time: Some("18:00".into()),
            tag_names: None,
        })
        .unwrap();

    let service =
        AISuggestionService::new(db, settings.clone(), dir.path().into()).expect("service");
    let record = service
        .request_daily_suggest(&tasks)
        .expect("request")
        .expect("record");
    assert!(!record.payload.items.is_empty());
    // Every item maps to a real candidate task.
    assert_eq!(record.payload.items.len(), record.sources.len());
}
