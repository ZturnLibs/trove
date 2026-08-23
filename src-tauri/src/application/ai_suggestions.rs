//! AI suggestion service (v2.0 slice 1): sanitize → provider → validate →
//! ledger. Business modules only ever receive structured records; invalid
//! model output is discarded with an audit row and never touches business
//! data (post-v1 §9.5).

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension};

use crate::application::clipboard::ClipboardService;
use crate::application::links::EntityLinkService;
use crate::application::reminders::ReminderService;
use crate::application::memories::MemoryService;
use crate::application::search::SearchService;
use crate::application::tasks::TaskService;
use crate::application::weekly_review::WeeklyReviewService;
use crate::domain::{
    new_id, parse_suggestion_content, stamp, AIFeature, AISuggestionRecord, CompletionRequest,
    ContextItem, DomainError, EntityLink, ExtractApplyInput, ExtractApplyResult, SearchEntityType,
    SearchQuery, SuggestionSource, SuggestionStatus, SystemClock, Task, LINK_KIND_RELATED,
};
use crate::infrastructure::ai::{build_provider, AIProvider};
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;

/// Stable source id for the weekly review summary ledger entries.
pub const WEEKLY_SOURCE_ID: &str = "weekly";

/// Substring candidate pool size for related-content suggestions (slice 4).
const RELATED_CANDIDATES: i64 = 12;
/// Max CJK bigrams probed per request (bounds LIKE sweeps).
const RELATED_FRAGMENT_CAP: usize = 24;

/// CJK bigrams (plus ASCII words) used as deterministic retrieval fragments.
fn bigrams(text: &str) -> Vec<String> {
    let mut frags: Vec<String> = Vec::new();
    let mut ascii_word = String::new();
    let flush_ascii = |acc: &mut String, out: &mut Vec<String>| {
        if acc.chars().count() >= 2 {
            out.push(acc.clone());
        }
        acc.clear();
    };
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            ascii_word.push(ch);
        } else {
            flush_ascii(&mut ascii_word, &mut frags);
            if is_cjk(ch) {
                frags.push(ch.to_string());
            }
        }
    }
    flush_ascii(&mut ascii_word, &mut frags);

    // Build bigrams over consecutive CJK single chars.
    let cjk: Vec<char> = text.chars().filter(|c| is_cjk(*c)).collect();
    let mut out: Vec<String> = Vec::new();
    for pair in cjk.windows(2) {
        out.push(pair.iter().collect());
    }
    out.extend(frags);
    out.sort();
    out.dedup();
    out
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF | 0x20000..=0x2FA1F)
}

/// Bounded, content-free label for clipboard items in prompts (image kind
/// never carries text; text kind shows a short prefix only).
fn display_title(item: &crate::domain::ClipboardItem) -> String {
    let base = item.ocr_text.as_deref().unwrap_or(&item.content);
    let mut label: String = base.chars().take(20).collect();
    if base.chars().count() > 20 {
        label.push('…');
    }
    label
}

fn excluded_source(app: &str, user_excluded: Vec<String>) -> bool {
    crate::domain::default_excluded_apps()
        .iter()
        .chain(user_excluded.iter())
        .any(|ex| app.eq_ignore_ascii_case(ex) || app.contains(ex.as_str()))
}

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

    /// Test/eval constructor with a provider fake (used by the offline
    /// eval runner in tests/ai_eval_offline.rs).
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

    /// Idempotency guard for slice 2: an unresolved suggestion for the same
    /// feature + source is returned instead of hitting the provider again.
    pub fn find_pending(
        &self,
        feature: AIFeature,
        source_entity_id: &str,
    ) -> Result<Option<AISuggestionRecord>, DomainError> {
        let conn = self.connect()?;
        let row = conn
            .query_row(
                "SELECT id, feature_type, source_entity_type, source_entity_id, payload, sources_json,
                        status, provider, model, created_at, decided_at
                 FROM ai_suggestions
                 WHERE feature_type = ?1 AND source_entity_id = ?2 AND status = 'pending'
                 ORDER BY created_at DESC LIMIT 1",
                params![feature.as_str(), source_entity_id],
                |row| { Ok(SuggestionRow {
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
                }) },
            )
            .optional()
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        row.map(row_from_db).transpose()
    }

    /// Slice 2 entry: extract task drafts from one memory's title + body.
    pub fn request_extract(
        &self,
        memory_id: &str,
        memories: &MemoryService,
    ) -> Result<Option<AISuggestionRecord>, DomainError> {
        if let Some(existing) = self.find_pending(AIFeature::Extract, memory_id)? {
            return Ok(Some(existing));
        }
        let memory = memories
            .get(memory_id.parse().map_err(|_| DomainError::Validation("记忆 id 非法".into()))?)?;
        if memory.sensitive {
            return Ok(None); // red line: sensitive memories never reach a provider
        }
        let context = vec![ContextItem {
            entity_type: "memory".into(),
            entity_id: memory.id.to_string(),
            text: format!("{}\n{}", memory.title, memory.body),
            source_app: None,
        }];
        self.request(AIFeature::Extract, "memory", memory_id, &context)
    }

    /// Apply selected draft items: create tasks (inbox), record provenance
    /// links, accept the suggestion. Items with ambiguous/unparseable dates
    /// are created WITHOUT dates (never guessed into the database).
    pub fn apply_extract(
        &self,
        input: ExtractApplyInput,
        tasks: &TaskService,
        links: &EntityLinkService,
        search: &SearchService,
    ) -> Result<ExtractApplyResult, DomainError> {
        let record = self.get(&input.suggestion_id)?;
        if record.feature_type != AIFeature::Extract.as_str() {
            return Err(DomainError::Validation("该建议不是任务提取类型".into()));
        }
        if record.status != SuggestionStatus::Pending {
            return Err(DomainError::Validation("建议已处理，不能重复应用".into()));
        }
        let selected = input.normalize(record.payload.items.len())?;

        let mut created: Vec<Task> = Vec::with_capacity(selected.len());
        for idx in selected {
            let item = &record.payload.items[idx];
            // Defensive double-check (slice-1 validation already guarantees
            // this for non-ambiguous items): dates land only when parseable.
            let due_date = if item.ambiguous {
                None
            } else {
                item.due_date
                    .as_deref()
                    .filter(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok())
            };
            let due_time = if due_date.is_some() {
                item.due_time
                    .as_deref()
                    .filter(|t| chrono::NaiveTime::parse_from_str(t, "%H:%M").is_ok())
            } else {
                None
            };

            let mut notes = item.detail.clone().unwrap_or_default();
            if !notes.is_empty() {
                notes.push('\n');
            }
            notes.push_str(&format!("来源：AI 从记忆《{}》提取", record.source_entity_id));
            notes.push_str(&format!("；原文：{}", item.source_excerpt));

            let task = tasks.create_task(crate::domain::CreateTaskInput {
                title: item.title.clone(),
                notes: Some(notes),
                priority: None,
                list_id: None,
                due_date: due_date.map(str::to_string),
                due_time: due_time.map(str::to_string),
                tag_names: None,
            })?;
            search.upsert(SearchEntityType::Task, task.id, &task.title, &task.notes)?;
            if let Ok(entity_id) = record.source_entity_id.parse() {
                links.link("memory", entity_id, "task", task.id, "ai_extract")?;
            }
            created.push(task);
        }

        let suggestion = self.decide(&input.suggestion_id, SuggestionStatus::Accepted)?;
        Ok(ExtractApplyResult {
            tasks: created,
            suggestion,
        })
    }

    /// Dismiss still-pending suggestions for a feature+source (used when a
    /// fresher generation supersedes them or the owning flow completes).
    fn dismiss_pending(
        &self,
        feature: AIFeature,
        source_entity_id: &str,
    ) -> Result<(), DomainError> {
        let conn = self.connect()?;
        conn.execute(
            "UPDATE ai_suggestions SET status = 'dismissed', decided_at = ?3
             WHERE feature_type = ?1 AND source_entity_id = ?2 AND status = 'pending'",
            params![feature.as_str(), source_entity_id, stamp(&self.clock)],
        )
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Slice 3: organize the weekly review's deterministic numbers into a
    /// short prose summary. Numbers come from `snapshot()`; the model only
    /// writes prose and must echo no task bodies (titles only in context).
    pub fn request_weekly_summary(
        &self,
        weekly: &WeeklyReviewService,
        tasks: &TaskService,
        reminders: &ReminderService,
        clipboard: &ClipboardService,
    ) -> Result<Option<AISuggestionRecord>, DomainError> {
        // A fresh generation supersedes any pending one.
        self.dismiss_pending(AIFeature::Summary, WEEKLY_SOURCE_ID)?;

        let snap = weekly.snapshot(tasks, reminders, clipboard)?;
        const TITLES: usize = 8;

        let mut lines = vec![format!(
            "本周统计：收件箱未处理 {}；逾期 {}；等待/跟进 {}；长期未动 {}；近7天完成 {}；周期提醒 {}；大体积剪贴板 {}。",
            snap.inbox_count,
            snap.overdue_count,
            snap.waiting_follow_up_count,
            snap.stale_active_count,
            snap.completed_last_7_days_count,
            snap.upcoming_recurring_count,
            snap.large_clipboard_count,
        )];

        let section = |name: &str, titles: Vec<String>| {
            if titles.is_empty() {
                None
            } else {
                Some(format!("{}：{}", name, titles.join("、")))
            }
        };
        let take_titles = |items: &[crate::domain::Task]| {
            items.iter().take(TITLES).map(|t| t.title.clone()).collect::<Vec<_>>()
        };
        lines.extend(section("收件箱示例", take_titles(&snap.inbox_unprocessed)));
        lines.extend(section("逾期示例", take_titles(&snap.overdue)));
        lines.extend(section("等待示例", take_titles(&snap.waiting_follow_up)));
        lines.extend(section("近7天完成示例", take_titles(&snap.completed_last_7_days)));
        // Clipboard items: titles only; sanitize drops excluded sources.
        let clipboard_titles = snap
            .large_clipboard_items
            .iter()
            .filter(|c| match &c.source_app {
                Some(app) => !excluded_source(app, self.settings.get().unwrap_or_default().clipboard_excluded_apps.clone()),
                None => true,
            })
            .take(TITLES)
            .map(|c| display_title(c))
            .collect::<Vec<_>>();
        lines.extend(section("大剪贴板示例", clipboard_titles));

        let context = vec![ContextItem {
            entity_type: "review".into(),
            entity_id: WEEKLY_SOURCE_ID.into(),
            text: lines.join("\n"),
            source_app: None,
        }];
        self.request(AIFeature::Summary, "review", WEEKLY_SOURCE_ID, &context)
    }

    /// Called by `weekly_review_complete`: no dangling pending summary.
    pub fn dismiss_pending_weekly_summary(&self) -> Result<(), DomainError> {
        self.dismiss_pending(AIFeature::Summary, WEEKLY_SOURCE_ID)
    }

    /// Slice 4: suggest related memories/clipboard items for a task.
    /// Candidates come from local FTS only; the model picks and motivates.
    /// Fabricated titles are dropped by exact-match back-mapping.
    pub fn request_related(
        &self,
        task_id: &str,
        tasks: &TaskService,
        search: &crate::application::search::SearchService,
        memories: &MemoryService,
        links: &EntityLinkService,
    ) -> Result<Option<AISuggestionRecord>, DomainError> {
        self.dismiss_pending(AIFeature::Related, task_id)?;

        let entity_id: crate::domain::EntityId = task_id
            .parse()
            .map_err(|_| DomainError::Validation("任务 id 非法".into()))?;
        let task = tasks.get_task(entity_id)?;

        // Deterministic candidate pool: CJK-bigram substring retrieval over
        // the search corpus (FTS tokenization misses Chinese paraphrases).
        let rejected = self.rejected_pair_ids(task_id)?;
        let sensitive_memories = self.sensitive_memory_ids()?;
        let mut pool: std::collections::HashMap<String, (crate::domain::SearchHit, i64)> =
            std::collections::HashMap::new();
        for fragment in bigrams(&format!("{} {}", task.title, task.notes))
            .into_iter()
            .take(RELATED_FRAGMENT_CAP)
        {
            let hits = search.query_substring(SearchQuery {
                query: fragment,
                types: Some(vec![SearchEntityType::Memory, SearchEntityType::Clipboard]),
                limit: Some(RELATED_CANDIDATES),
            })?;
            for hit in hits.memories.into_iter().chain(hits.clipboard) {
                let id_str = hit.entity_id.to_string();
                let entry = pool.entry(id_str).or_insert((hit, 0));
                entry.1 += 1;
            }
        }

        // list_for_entity matches both directions; collect the other side's id.
        let linked_both: std::collections::HashSet<String> = links
            .list_for_entity("task", entity_id)?
            .into_iter()
            .flat_map(|l| {
                if l.source_id == entity_id {
                    vec![l.target_id.to_string()]
                } else {
                    vec![l.source_id.to_string()]
                }
            })
            .collect();

        let mut ranked: Vec<(crate::domain::SearchHit, i64)> = pool
            .into_values()
            .filter(|(hit, _)| {
                let id_str = hit.entity_id.to_string();
                !linked_both.contains(&id_str)
                    && !rejected.contains(&id_str)
                    && !(hit.entity_type == SearchEntityType::Memory
                        && sensitive_memories.contains(&id_str))
            })
            .collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.updated_at.cmp(&b.0.updated_at)));
        ranked.truncate(RELATED_CANDIDATES as usize);

        let mut candidates: Vec<(String, String, String, String)> = Vec::new(); // (title, excerpt, entity_type, entity_id)
        for (hit, _) in ranked {
            let id_str = hit.entity_id.to_string();
            let excerpt: String = hit.snippet.chars().take(20).collect();
            candidates.push((hit.title.clone(), excerpt, hit.entity_type.as_str().to_string(), id_str));
        }
        if candidates.is_empty() {
            return Ok(None);
        }
        let _ = memories; // reserved for future body-level filtering

        let listing = candidates
            .iter()
            .enumerate()
            .map(|(i, (title, excerpt, _, _))| format!("{}. 《{}》 摘要：{}", i + 1, title, excerpt))
            .collect::<Vec<_>>()
            .join("\n");

        let context = vec![
            ContextItem {
                entity_type: "task".into(),
                entity_id: task_id.to_string(),
                text: format!("任务：《{}》\n说明：{}", task.title, task.notes),
                source_app: None,
            },
            ContextItem {
                entity_type: "task".into(),
                entity_id: task_id.to_string(),
                text: format!("候选列表：\n{}", listing),
                source_app: None,
            },
        ];

        let record = match self.request(AIFeature::Related, "task", task_id, &context)? {
            Some(record) => record,
            None => return Ok(None),
        };

        // Back-mapping: drop any item whose title is not an exact candidate.
        let by_title: std::collections::HashMap<&str, &(String, String, String, String)> =
            candidates.iter().map(|c| (c.0.as_str(), c)).collect();
        let matched: Vec<_> = record
            .payload
            .items
            .iter()
            .filter(|item| by_title.contains_key(item.title.as_str()))
            .cloned()
            .collect();
        if matched.is_empty() {
            // Everything fabricated: drop the record as invalid (audit row).
            self.decide(&record.id.to_string(), SuggestionStatus::Dismissed)?;
            return Ok(None);
        }
        let sources: Vec<SuggestionSource> = matched
            .iter()
            .filter_map(|item| {
                by_title.get(item.title.as_str()).map(|(_, _, etype, eid)| SuggestionSource {
                    entity_type: etype.clone(),
                    entity_id: eid.clone(),
                    text_offset: 0,
                    excerpt: item.source_excerpt.clone(),
                })
            })
            .collect();
        // Rewrite the ledger row with only matched items + provenance.
        let updated = AISuggestionRecord {
            payload: crate::domain::SuggestionContent {
                items: matched,
                summary: None,
            },
            sources,
            ..record.clone()
        };
        self.replace_payload(&updated)?;
        Ok(Some(updated))
    }

    /// Confirm selected related items: idempotent related links, then accept.
    pub fn confirm_related(
        &self,
        input: ExtractApplyInput,
        task_id: &str,
        links: &EntityLinkService,
    ) -> Result<Vec<EntityLink>, DomainError> {
        let record = self.get(&input.suggestion_id)?;
        if record.feature_type != AIFeature::Related.as_str() {
            return Err(DomainError::Validation("该建议不是相关内容类型".into()));
        }
        if record.status != SuggestionStatus::Pending {
            return Err(DomainError::Validation("建议已处理，不能重复应用".into()));
        }
        let selected = input.normalize(record.payload.items.len())?;
        let source: crate::domain::EntityId = task_id
            .parse()
            .map_err(|_| DomainError::Validation("任务 id 非法".into()))?;

        let existing: std::collections::HashSet<String> = links
            .list_for_entity("task", source)?
            .into_iter()
            .map(|l| l.target_id.to_string())
            .collect();

        let mut created = Vec::new();
        for idx in selected {
            let Some(src) = record.sources.get(idx) else { continue };
            if existing.contains(&src.entity_id) {
                continue; // idempotent: never duplicate a user-visible link
            }
            let target: crate::domain::EntityId = src
                .entity_id
                .parse()
                .map_err(|_| DomainError::Validation("候选 id 非法".into()))?;
            created.push(links.link("task", source, &src.entity_type, target, LINK_KIND_RELATED)?);
        }
        self.decide(&input.suggestion_id, SuggestionStatus::Accepted)?;
        Ok(created)
    }

    /// Reject one item as irrelevant: persisted as a dismissed pair record so
    /// future requests for this task filter it out. Closes the main record
    /// when nothing remains.
    pub fn reject_related_item(
        &self,
        suggestion_id: &str,
        index: usize,
    ) -> Result<AISuggestionRecord, DomainError> {
        let record = self.get(suggestion_id)?;
        if record.feature_type != AIFeature::Related.as_str()
            || record.status != SuggestionStatus::Pending
        {
            return Err(DomainError::Validation("建议不可用或已处理".into()));
        }
        if index >= record.payload.items.len() || index >= record.sources.len() {
            return Err(DomainError::Validation("条目不存在".into()));
        }

        // Pair audit row (dismissed): future candidate filter reads these.
        let pair = AISuggestionRecord {
            id: new_id().to_string(),
            feature_type: AIFeature::Related.as_str().to_string(),
            source_entity_type: record.source_entity_type.clone(),
            source_entity_id: record.source_entity_id.clone(),
            payload: crate::domain::SuggestionContent {
                items: vec![record.payload.items[index].clone()],
                summary: None,
            },
            sources: vec![record.sources[index].clone()],
            status: SuggestionStatus::Dismissed,
            provider: record.provider.clone(),
            model: record.model.clone(),
            created_at: stamp(&self.clock),
            decided_at: Some(stamp(&self.clock)),
        };
        self.insert(&pair)?;

        // Remove the item from the pending record.
        let mut items = record.payload.items.clone();
        let mut sources = record.sources.clone();
        items.remove(index);
        sources.remove(index);
        if items.is_empty() {
            let closed = AISuggestionRecord {
                payload: crate::domain::SuggestionContent { items, summary: None },
                sources,
                ..record.clone()
            };
            self.replace_payload(&closed)?;
            return self.decide(suggestion_id, SuggestionStatus::Rejected);
        }
        let updated = AISuggestionRecord {
            payload: crate::domain::SuggestionContent { items, summary: None },
            sources,
            ..record.clone()
        };
        self.replace_payload(&updated)?;
        Ok(updated)
    }

    fn replace_payload(&self, record: &AISuggestionRecord) -> Result<(), DomainError> {
        let payload_json = serde_json::to_string(&record.payload)
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let sources_json = serde_json::to_string(&record.sources)
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let conn = self.connect()?;
        conn.execute(
            "UPDATE ai_suggestions SET payload = ?2, sources_json = ?3 WHERE id = ?1",
            params![record.id, payload_json, sources_json],
        )
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    fn rejected_pair_ids(
        &self,
        source_entity_id: &str,
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT sources_json FROM ai_suggestions
                 WHERE feature_type = 'related' AND source_entity_id = ?1
                   AND status IN ('rejected','dismissed')",
            )
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map([source_entity_id], |row| row.get::<_, String>(0))
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let mut ids = std::collections::HashSet::new();
        for row in rows {
            let json = row.map_err(|e| DomainError::Internal(e.to_string()))?;
            if let Ok(sources) = serde_json::from_str::<Vec<SuggestionSource>>(&json) {
                ids.extend(sources.into_iter().map(|s| s.entity_id));
            }
        }
        Ok(ids)
    }

    fn sensitive_memory_ids(&self) -> Result<std::collections::HashSet<String>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT id FROM memories WHERE sensitive = 1 AND deleted_at IS NULL")
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let mut ids = std::collections::HashSet::new();
        for row in rows {
            ids.insert(row.map_err(|e| DomainError::Internal(e.to_string()))?);
        }
        Ok(ids)
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
    use crate::domain::{AIMode, AIFeatureToggles, CreateMemoryInput, UpdateMemoryInput};
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

    fn setup() -> (tempfile::TempDir, AISuggestionService, Arc<SettingsService>, Arc<FakeProvider>) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let settings = Arc::new(SettingsService::new(db.clone()));
        let provider = Arc::new(FakeProvider {
            output: None,
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        let service = AISuggestionService::with_provider(db, settings.clone(), provider.clone());
        (dir, service, settings, provider)
    }

    fn enable_extract(settings: &SettingsService) {
        let base = settings.get().unwrap();
        let next = crate::infrastructure::settings::AppSettings {
            ai: crate::domain::AIConfig {
                mode: AIMode::Ollama,
                ollama_model: "fake".into(),
                features: AIFeatureToggles {
                    extract: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..base
        };
        settings.save(&next).unwrap();
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
        let (_dir, service, _settings, _provider) = setup(); // features all default off
        let result = service
            .request(AIFeature::Extract, "memory", "m1", &[ctx("memory", "m1", "开会记录", None)])
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn sensitive_memory_is_sanitized_out() {
        let (_dir, service, settings, _provider) = setup();
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
        let (_dir, service, _settings, _provider) = setup();
        let kept = service.sanitize_context(&[
            ctx("clipboard", "c1", "复制的密码", Some("1Password")),
            ctx("clipboard", "c2", "普通文本", Some("Safari")),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].entity_id, "c2");
    }

    #[test]
    fn user_excluded_apps_are_sanitized_out() {
        let (_dir, service, settings, _provider) = setup();
        let mut base = settings.get().unwrap();
        base.clipboard_excluded_apps.push("微信".into());
        settings.save(&base).unwrap();

        let kept = service.sanitize_context(&[ctx("clipboard", "c1", "x", Some("微信"))]);
        assert!(kept.is_empty());
    }

    #[test]
    fn valid_output_lands_as_pending_with_sources() {
        let (dir, _service, settings, _provider) = setup();
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
        let (dir, _service, settings, _provider) = setup();
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
        let (dir, _service, settings, _provider) = setup();
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
        let (dir, _service, settings, _provider) = setup();
        enable_extract(&settings);
        let current = settings.get().unwrap();
        let off = crate::infrastructure::settings::AppSettings {
            ai: crate::domain::AIConfig {
                mode: AIMode::Off,
                ..current.ai.clone()
            },
            ..current
        };
        settings.save(&off).unwrap();
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
    // ------------------------------------------------------------------
    // Slice 2: request_extract / apply_extract
    // ------------------------------------------------------------------

    fn support_services(dir: &std::path::Path) -> (TaskService, EntityLinkService, SearchService, MemoryService) {
        let db = Database::open(dir.join("workbench.db")).unwrap();
        let tasks = TaskService::new(db.clone());
        tasks.ensure_seed_data().unwrap();
        let links = EntityLinkService::new(db.clone());
        let search = SearchService::new(db.clone());
        let memories = MemoryService::new(db);
        (tasks, links, search, memories)
    }

    const GOOD_OUTPUT: &str = r#"{"items":[
        {"title":"交投放数据","detail":"王琳负责","dueDate":"2026-08-25","dueTime":"10:00","ambiguous":false,"sourceExcerpt":"王琳在 10 号前提交投放数据"},
        {"title":"确认合同","detail":null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":"找老张确认合同"}
    ],"summary":null}"#;

    #[test]
    fn request_extract_is_idempotent_while_pending() {
        let (dir, _service, settings, _provider) = setup();
        enable_extract(&settings);
        let (_tasks, _links, _search, memories) = support_services(dir.path());
        let memory = memories
            .create(CreateMemoryInput {
                title: "周会记录".into(),
                body: Some("王琳在 10 号前提交投放数据；找老张确认合同".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();

        // Provider that panics on a second call: the idempotency guard must
        // prevent it from ever being reached twice for the same source.
        struct OnceProvider;
        impl AIProvider for OnceProvider {
            fn probe(&self) -> crate::domain::ProbeReport {
                crate::domain::ProbeReport {
                    mode: AIMode::Ollama,
                    reachable: true,
                    model: Some("once".into()),
                    latency_ms: Some(1),
                    hint: None,
                }
            }
            fn complete(&self, _request: &CompletionRequest) -> Option<crate::domain::CompletionOutput> {
                crate::domain::CompletionOutput {
                    raw_json: GOOD_OUTPUT.into(),
                }
                .into()
            }
        }
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let service = AISuggestionService::with_provider(db, settings, Arc::new(OnceProvider));

        let first = service
            .request_extract(&memory.id.to_string(), &memories)
            .unwrap()
            .expect("record");
        assert_eq!(first.payload.items.len(), 2);
        assert_eq!(first.status, SuggestionStatus::Pending);

        // Second request resolves from the pending ledger without any
        // provider round-trip and returns the same record.
        let second = service
            .request_extract(&memory.id.to_string(), &memories)
            .unwrap()
            .expect("record");
        assert_eq!(first.id, second.id);
    }

    #[test]
    fn request_extract_skips_sensitive_memory() {
        let (dir, _service, settings, _provider) = setup();
        enable_extract(&settings);
        let (_tasks, _links, _search, memories) = support_services(dir.path());
        let memory = memories
            .create(CreateMemoryInput {
                title: "密码".into(),
                body: Some("sk-xxx".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();
        memories
            .update(UpdateMemoryInput {
                id: memory.id,
                title: "密码".into(),
                body: "sk-xxx".into(),
                pinned: false,
                archived: false,
                quick_insert: false,
                trigger_word: None,
                sensitive: true,
                tag_names: vec![],
            })
            .unwrap();

        let provider = Arc::new(FakeProvider {
            output: Some(GOOD_OUTPUT),
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let service = AISuggestionService::with_provider(db, settings.clone(), provider.clone());
        let result = service.request_extract(&memory.id.to_string(), &memories).unwrap();
        assert!(result.is_none());
        assert_eq!(
            provider.calls.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "sensitive memory must short-circuit before the provider"
        );
    }

    fn seeded_pending_suggestion(dir: &std::path::Path, settings: &Arc<SettingsService>) -> (AISuggestionService, String, MemoryService) {
        let (tasks, links, search, memories) = support_services(dir);
        let _ = (&tasks, &links, &search);
        let memory = memories
            .create(CreateMemoryInput {
                title: "周会记录".into(),
                body: Some("王琳在 10 号前提交投放数据；找老张确认合同".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();
        let provider = Arc::new(FakeProvider {
            output: Some(GOOD_OUTPUT),
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        let db = Database::open(dir.join("workbench.db")).unwrap();
        let service = AISuggestionService::with_provider(db, settings.clone(), provider);
        let record = service
            .request_extract(&memory.id.to_string(), &memories)
            .unwrap()
            .expect("record");
        (service, record.id.to_string(), memories)
    }

    #[test]
    fn apply_extract_creates_tasks_and_marks_accepted() {
        let (dir, _service, settings, _provider) = setup();
        enable_extract(&settings);
        let (service, suggestion_id, memories) = seeded_pending_suggestion(dir.path(), &settings);
        let (tasks, links, search, _m) = support_services(dir.path());

        let result = service
            .apply_extract(
                ExtractApplyInput {
                    suggestion_id: suggestion_id.clone(),
                    selected_indices: vec![0, 1],
                },
                &tasks,
                &links,
                &search,
            )
            .unwrap();

        assert_eq!(result.tasks.len(), 2);
        assert_eq!(result.suggestion.status, SuggestionStatus::Accepted);
        // Non-ambiguous item carries the parsed date…
        assert_eq!(result.tasks[0].due_date.as_deref(), Some("2026-08-25"));
        assert_eq!(result.tasks[0].due_time.as_deref(), Some("10:00"));
        // …ambiguous item never gets a guessed date.
        assert_eq!(result.tasks[1].due_date, None);
        assert!(result.tasks[0].notes.contains("AI 从记忆"));
        // Provenance link recorded for each created task.
        for task in &result.tasks {
            let linked = links
                .list_outgoing("memory", memories.get(result.suggestion.source_entity_id.parse().unwrap()).unwrap().id)
                .unwrap();
            assert!(linked.iter().any(|l| l.target_id == task.id), "ai_extract link missing");
        }

        // Applying again is rejected (idempotency guard).
        assert!(service
            .apply_extract(
                ExtractApplyInput {
                    suggestion_id,
                    selected_indices: vec![0],
                },
                &tasks,
                &links,
                &search,
            )
            .is_err());
    }

    #[test]
    fn apply_extract_rejects_bad_indices_and_non_extract() {
        let (dir, _service, settings, _provider) = setup();
        enable_extract(&settings);
        let (service, suggestion_id, _m) = seeded_pending_suggestion(dir.path(), &settings);
        let (tasks, links, search, _m2) = support_services(dir.path());

        let out_of_range = service
            .apply_extract(
                ExtractApplyInput {
                    suggestion_id: suggestion_id.clone(),
                    selected_indices: vec![9],
                },
                &tasks,
                &links,
                &search,
            );
        assert!(out_of_range.is_err());
        // Suggestion stays pending after a failed apply.
        assert_eq!(service.get(&suggestion_id).unwrap().status, SuggestionStatus::Pending);
    }

    #[test]
    fn apply_extract_partial_failure_keeps_pending() {
        let (dir, _service, settings, _provider) = setup();
        enable_extract(&settings);
        let (service, suggestion_id, _m) = seeded_pending_suggestion(dir.path(), &settings);
        let (tasks, links, search, _m2) = support_services(dir.path());

        // Index 0 is valid; force index 1 to fail validation by emptying its
        // title through a hand-crafted second suggestion? Instead simulate a
        // partial failure via an out-of-range later index: normalize fails
        // atomically before any creation.
        let result = service
            .apply_extract(
                ExtractApplyInput {
                    suggestion_id: suggestion_id.clone(),
                    selected_indices: vec![0, 5],
                },
                &tasks,
                &links,
                &search,
            );
        assert!(result.is_err());
        assert_eq!(service.get(&suggestion_id).unwrap().status, SuggestionStatus::Pending);
    }

    // ------------------------------------------------------------------
    // Slice 3: weekly summary
    // ------------------------------------------------------------------

    const SUMMARY_OUTPUT: &str = r#"{"items":[],"summary":"本周完成 3 项，逾期 1 项，可先挑 1 项处理。"}"#;

    #[test]
    fn weekly_summary_regenerations_dismiss_pending() {
        let (dir, _service, settings, _provider) = setup();
        let base = settings.get().unwrap();
        let next = crate::infrastructure::settings::AppSettings {
            ai: crate::domain::AIConfig {
                mode: AIMode::Ollama,
                ollama_model: "fake".into(),
                features: AIFeatureToggles {
                    summary: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..base
        };
        settings.save(&next).unwrap();

        let (tasks, _links, _search, _m) = support_services(dir.path());
        let reminders = ReminderService::new(Database::open(dir.path().join("workbench.db")).unwrap());
        let clipboard = ClipboardService::new(
            Database::open(dir.path().join("workbench.db")).unwrap(),
            std::path::PathBuf::from(dir.path().join("assets")),
        );
        let weekly = WeeklyReviewService::new(Database::open(dir.path().join("workbench.db")).unwrap());

        let provider = Arc::new(FakeProvider {
            output: Some(SUMMARY_OUTPUT),
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let service = AISuggestionService::with_provider(db, settings.clone(), provider);

        let first = service
            .request_weekly_summary(&weekly, &tasks, &reminders, &clipboard)
            .unwrap()
            .expect("record");
        assert_eq!(first.feature_type, "summary");
        assert_eq!(first.source_entity_id, WEEKLY_SOURCE_ID);
        assert!(first.payload.summary.is_some());
        assert!(first.payload.items.is_empty());

        // Regenerate: old pending dismissed, new pending created.
        let second = service
            .request_weekly_summary(&weekly, &tasks, &reminders, &clipboard)
            .unwrap()
            .expect("record");
        assert_ne!(first.id, second.id);
        assert_eq!(service.get(&first.id.to_string()).unwrap().status, SuggestionStatus::Dismissed);

        // Completing the review clears the remaining pending summary.
        service.dismiss_pending_weekly_summary().unwrap();
        assert_eq!(service.get(&second.id.to_string()).unwrap().status, SuggestionStatus::Dismissed);
    }


    // ------------------------------------------------------------------
    // Slice 4: related content
    // ------------------------------------------------------------------

    fn enable_related(settings: &SettingsService) {
        let base = settings.get().unwrap();
        let next = crate::infrastructure::settings::AppSettings {
            ai: crate::domain::AIConfig {
                mode: AIMode::Ollama,
                ollama_model: "fake".into(),
                features: AIFeatureToggles {
                    related: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..base
        };
        settings.save(&next).unwrap();
    }

    const RELATED_OUTPUT: &str = r#"{"items":[
        {"title":"差旅票据整理","detail":"都涉及报销","dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":"差旅票据"},
        {"title":"我编造的条目","detail":null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":"x"}
    ],"summary":null}"#;

    fn related_setup() -> (
        tempfile::TempDir,
        Arc<SettingsService>,
        TaskService,
        crate::application::search::SearchService,
        MemoryService,
        EntityLinkService,
    ) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let settings = Arc::new(SettingsService::new(db.clone()));
        enable_related(&settings);
        let tasks = TaskService::new(db.clone());
        tasks.ensure_seed_data().unwrap();
        let search = crate::application::search::SearchService::new(db.clone());
        let memories = MemoryService::new(db.clone());
        let links = EntityLinkService::new(db.clone());
        (dir, settings, tasks, search, memories, links)
    }

    fn related_service(
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

    #[test]
    fn related_drops_fabricated_titles_and_links_nothing_by_default() {
        let (dir, settings, tasks, search, memories, links) = related_setup();
        // Seed a memory that FTS can find from the task text.
        memories
            .create(CreateMemoryInput {
                title: "差旅票据整理".into(),
                body: Some("报销流程与票据".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();
        let task = tasks
            .create_task(crate::domain::CreateTaskInput {
                title: "整理报销票据".into(),
                notes: Some("差旅报销".into()),
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let service = related_service(dir.path(), &settings, Some(RELATED_OUTPUT));
        let record = service
            .request_related(&task.id.to_string(), &tasks, &search, &memories, &links)
            .unwrap()
            .expect("record");

        // The fabricated title is gone; only the real candidate survives.
        assert_eq!(record.payload.items.len(), 1);
        assert_eq!(record.payload.items[0].title, "差旅票据整理");
        assert_eq!(record.sources.len(), 1);
        assert_eq!(record.sources[0].entity_type, "memory");

        // Requesting alone must not create any link (§9.3 no auto-association).
        assert!(links.list_for_entity("task", task.id).unwrap().is_empty());
    }

    #[test]
    fn related_confirm_is_idempotent_and_reject_reduces_future() {
        let (dir, settings, tasks, search, memories, links) = related_setup();
        memories
            .create(CreateMemoryInput {
                title: "差旅票据整理".into(),
                body: Some("报销流程与票据".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();
        let task = tasks
            .create_task(crate::domain::CreateTaskInput {
                title: "整理报销票据".into(),
                notes: Some("差旅报销".into()),
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let service = related_service(dir.path(), &settings, Some(RELATED_OUTPUT));
        let record = service
            .request_related(&task.id.to_string(), &tasks, &search, &memories, &links)
            .unwrap()
            .expect("record");

        // Confirm the matched item → related link written, suggestion accepted.
        let created = service
            .confirm_related(
                ExtractApplyInput {
                    suggestion_id: record.id.to_string(),
                    selected_indices: vec![0],
                },
                &task.id.to_string(),
                &links,
            )
            .unwrap();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].link_kind, "related");
        assert_eq!(
            service.get(&record.id.to_string()).unwrap().status,
            SuggestionStatus::Accepted
        );

        // Reject flow on a fresh task: item rejected → pair filtered later.
        let task2 = tasks
            .create_task(crate::domain::CreateTaskInput {
                title: "再整理一次报销".into(),
                notes: Some("差旅报销".into()),
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let record2 = service
            .request_related(&task2.id.to_string(), &tasks, &search, &memories, &links)
            .unwrap()
            .expect("record");
        let after_reject = service
            .reject_related_item(&record2.id.to_string(), 0)
            .unwrap();
        assert!(after_reject.payload.items.is_empty(), "last item closes the record");
        assert_eq!(
            service.get(&record2.id.to_string()).unwrap().status,
            SuggestionStatus::Rejected
        );

        // Next request for the same task: rejected pair must not reappear.
        let again = related_service(dir.path(), &settings, Some(RELATED_OUTPUT));
        let result = again
            .request_related(&task2.id.to_string(), &tasks, &search, &memories, &links)
            .unwrap();
        assert!(
            result.is_none(),
            "rejected pair filtered out → no candidates left"
        );
    }

    #[test]
    fn related_skips_sensitive_and_linked_candidates() {
        let (dir, settings, tasks, search, memories, links) = related_setup();
        let sensitive = memories
            .create(CreateMemoryInput {
                title: "银行卡信息".into(),
                body: Some("密码相关".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();
        memories
            .update(UpdateMemoryInput {
                id: sensitive.id,
                title: "银行卡信息".into(),
                body: "密码相关".into(),
                pinned: false,
                archived: false,
                quick_insert: false,
                trigger_word: None,
                sensitive: true,
                tag_names: vec![],
            })
            .unwrap();
        let task = tasks
            .create_task(crate::domain::CreateTaskInput {
                title: "银行卡信息处理".into(),
                notes: Some("密码相关".into()),
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        // Link the sensitive memory manually: it must be excluded twice over.
        links
            .link("task", task.id, "memory", sensitive.id, "related")
            .unwrap();

        let service = related_service(dir.path(), &settings, Some(RELATED_OUTPUT));
        let result = service
            .request_related(&task.id.to_string(), &tasks, &search, &memories, &links)
            .unwrap();
        assert!(result.is_none(), "sensitive + already linked → no candidates");
    }

}
