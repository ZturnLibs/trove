//! v2.0 AI suggestion domain types (slice 1: service boundary only).
//!
//! Contract (post-v1 §9.4/9.5, parent task global contract):
//! - Business modules only ever see structured suggestions; raw model text
//!   never becomes a database write.
//! - Every suggestion carries `SuggestionSource` references back to the
//!   original entity so the UI can jump to provenance.
//! - Dates extracted with low confidence must be flagged `ambiguous`; the
//!   UI marks them for confirmation instead of guessing (nl_parse parity).

use serde::{Deserialize, Serialize};

use super::DomainError;

/// Feature kinds planned for v2.0. Slice 1 ships toggles + plumbing only;
/// `prompt_template` returns `None` for features not yet opened, in which
/// case the service short-circuits without touching any provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AIFeature {
    Extract,
    Related,
    Summary,
    Suggest,
    Split,
}

impl AIFeature {
    pub fn as_str(&self) -> &'static str {
        match self {
            AIFeature::Extract => "extract",
            AIFeature::Related => "related",
            AIFeature::Summary => "summary",
            AIFeature::Suggest => "suggest",
            AIFeature::Split => "split",
        }
    }

    /// OpenAI-compatible prompt for this feature. `None` = feature not yet
    /// shipped; `AISuggestionService::request` must not call any provider.
    pub fn prompt_template(&self) -> Option<&'static str> {
        match self {
            AIFeature::Extract => Some(EXTRACT_SYSTEM_PROMPT),
            AIFeature::Summary => Some(WEEKLY_SUMMARY_SYSTEM_PROMPT),
            AIFeature::Related => Some(RELATED_SYSTEM_PROMPT),
            AIFeature::Suggest => Some(SUGGEST_SYSTEM_PROMPT),
            _ => None,
        }
    }
}

/// System prompt for long-text task extraction (slice 2 consumes the
/// pipeline end to end; slice 1 exercises it via the offline eval runner).
pub const EXTRACT_SYSTEM_PROMPT: &str = r#"你是个人工作台的任务提取助手。从用户提供的文本中识别候选任务。
规则：
1. 只输出 JSON 对象：{"items":[{"title":string,"detail":string|null,"dueDate":string|null,"dueTime":string|null,"ambiguous":boolean,"sourceExcerpt":string}],"summary":string|null}
2. dueDate 格式 YYYY-MM-DD，dueTime 格式 HH:MM；日期/时间不确定或需要上下文才能推断时置 null 并把 ambiguous 设为 true。
3. 严禁编造原文没有的内容；sourceExcerpt 必须是原文中的连续片段。
4. 不确定是否为任务的条目跳过，宁缺毋滥。"#;

/// System prompt for the weekly review summary (slice 3). All numbers come
/// from deterministic queries; the model only organizes prose (§9.3).
pub const WEEKLY_SUMMARY_SYSTEM_PROMPT: &str = r#"你是个人工作台的回顾助手。把用户给定的本周统计数字组织成一段中文小结。
规则：
1. 只输出 JSON 对象：{"summary":string,"items":[]}
2. summary 不超过 200 字；只陈述给定数字与事实，可给温和提示（如“逾期 3 项，可先挑 1 项处理”）。
3. 严禁评价表现、打分、排名或使用“落后/失败/糟糕/拖延”等词；不得编造数字之外的信息。
4. 可以提及给定的任务名，但不得改写任务名。"#;

/// System prompt for related-content suggestions (slice 4). The model only
/// picks and motivates candidates retrieved locally; fabricated titles are
/// dropped server-side by exact-match back-mapping.
pub const RELATED_SYSTEM_PROMPT: &str = r#"你是个人工作台的相关内容推荐助手。给定一个任务和候选内容列表，选出真正相关的条目（最多 5 条）。
规则：
1. 只输出 JSON 对象：{"items":[{"title":string,"detail":string|null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":string}],"summary":null}
2. title 必须与候选列表中的标题完全一致；sourceExcerpt 必须与候选摘要完全一致。不得编造候选列表之外的条目。
3. detail 用一句话说明相关理由（如“都涉及 Q4 预算”）。
4. 不确定相关的宁可不选。"#;

/// System prompt for daily work suggestions (slice 5). The model picks 1–3
/// candidates from a locally computed pool with deterministic feature lines;
/// reasons must cite those features, never invent facts.
pub const SUGGEST_SYSTEM_PROMPT: &str = r#"你是个人工作台的今日规划助手。给定今天的候选任务（含确定性特征），挑出最值得今天聚焦的 1–3 项。
规则：
1. 只输出 JSON 对象：{"items":[{"title":string,"detail":string|null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":string}],"summary":null}
2. title 必须与候选列表完全一致；sourceExcerpt 必须与该项特征行完全一致。
3. detail 用一句话说明为什么今天做，必须基于特征（如“今天 18:00 截止”“已延期 2 次”“高优先级”）。
4. 严禁编造特征之外的信息；不确定的宁可不选。"#;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIFeatureToggles {
    pub extract: bool,
    pub related: bool,
    pub summary: bool,
    pub suggest: bool,
    pub split: bool,
}

impl AIFeatureToggles {
    pub fn enabled(&self, feature: AIFeature) -> bool {
        match feature {
            AIFeature::Extract => self.extract,
            AIFeature::Related => self.related,
            AIFeature::Summary => self.summary,
            AIFeature::Suggest => self.suggest,
            AIFeature::Split => self.split,
        }
    }
}

/// Provider mode. Default `Off`: zero provider calls, all v1.x paths intact
/// (§9.1 gate 4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AIMode {
    #[default]
    Off,
    Ollama,
    Custom,
}

impl AIMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AIMode::Off => "off",
            AIMode::Ollama => "ollama",
            AIMode::Custom => "custom",
        }
    }
}

/// User-level AI configuration stored inside `AppSettings`. The provider API
/// key deliberately does NOT live here: `AppSettings` persists in the
/// settings table which travels with full-database backups/exports, so the
/// key is kept in a separate file outside the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIConfig {
    pub mode: AIMode,
    pub ollama_url: String,
    /// Model name for the local Ollama endpoint. Empty = not selected yet;
    /// the provider reports unreachable with a model-selection hint.
    pub ollama_model: String,
    pub custom_endpoint: String,
    pub custom_model: String,
    pub features: AIFeatureToggles,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            mode: AIMode::Off,
            ollama_url: "http://localhost:11434".into(),
            ollama_model: String::new(),
            custom_endpoint: String::new(),
            custom_model: String::new(),
            features: AIFeatureToggles::default(),
        }
    }
}

impl AIConfig {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.mode == AIMode::Custom && self.custom_endpoint.trim().is_empty() {
            return Err(DomainError::Validation("自定义远程模式需填写 endpoint".into()));
        }
        if self.mode == AIMode::Custom && self.custom_model.trim().is_empty() {
            return Err(DomainError::Validation("自定义远程模式需填写模型名".into()));
        }
        Ok(())
    }
}

/// A piece of user content offered to the model as minimal context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextItem {
    pub entity_type: String,
    pub entity_id: String,
    pub text: String,
    /// Source application for clipboard-derived items; used by sanitize.
    pub source_app: Option<String>,
}

/// Provenance reference attached to each suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionSource {
    pub entity_type: String,
    pub entity_id: String,
    pub text_offset: usize,
    pub excerpt: String,
}

/// One structured suggestion item. Generic envelope shared by features;
/// slice 2 (extract) is the first consumer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedItem {
    pub title: String,
    pub detail: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub ambiguous: bool,
    pub source_excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionContent {
    pub items: Vec<SuggestedItem>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
    Dismissed,
}

impl SuggestionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SuggestionStatus::Pending => "pending",
            SuggestionStatus::Accepted => "accepted",
            SuggestionStatus::Rejected => "rejected",
            SuggestionStatus::Dismissed => "dismissed",
        }
    }
}

/// Persisted suggestion row (derived data: rebuildable, excluded from the
/// JSON export whitelist, safe to clear at any time).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AISuggestionRecord {
    pub id: String,
    pub feature_type: String,
    pub source_entity_type: String,
    pub source_entity_id: String,
    pub payload: SuggestionContent,
    pub sources: Vec<SuggestionSource>,
    pub status: SuggestionStatus,
    pub provider: String,
    pub model: String,
    pub created_at: String,
    pub decided_at: Option<String>,
}

/// What the application layer hands to a provider implementation.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub system_prompt: String,
    pub user_context: String,
    /// Bounded by the caller (minimal necessary context, §9.4).
    pub max_context_chars: usize,
}

impl CompletionRequest {
    pub fn new(system_prompt: &str, user_context: &str) -> Self {
        Self {
            system_prompt: system_prompt.to_string(),
            user_context: user_context.to_string(),
            max_context_chars: 12_000,
        }
    }

    pub fn truncated_context(&self) -> String {
        if self.user_context.chars().count() <= self.max_context_chars {
            return self.user_context.clone();
        }
        let cut: String = self
            .user_context
            .chars()
            .take(self.max_context_chars)
            .collect();
        format!("{cut}\n…[内容过长已截断]")
    }
}

/// Structured model output before domain validation.
#[derive(Debug, Clone)]
pub struct CompletionOutput {
    pub raw_json: String,
}

/// Provider connectivity report surfaced by the settings page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeReport {
    pub mode: AIMode,
    pub reachable: bool,
    pub model: Option<String>,
    pub latency_ms: Option<u64>,
    /// i18n key for the guidance copy shown on failure (tone follows
    /// docs/empty-states-and-permissions.md).
    pub hint: Option<String>,
}

/// Validate raw model JSON into `SuggestionContent`.
///
/// Rejects (returns `Err`): malformed JSON, empty items + no summary,
/// non-empty `dueDate`/`dueTime` that violate formats while `ambiguous` is
/// false, empty titles, or fabricated `sourceExcerpt`s that quote nothing.
/// Invalid outputs are discarded by the service; business data is never
/// touched (§9.5 structured-only contract).
pub fn parse_suggestion_content(raw: &str) -> Result<SuggestionContent, DomainError> {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    let parsed: SuggestionContent = serde_json::from_str(stripped)
        .map_err(|e| DomainError::Validation(format!("模型输出不符合结构: {e}")))?;

    if parsed.items.is_empty() && parsed.summary.as_deref().unwrap_or("").trim().is_empty() {
        return Err(DomainError::Validation("模型输出为空".into()));
    }

    for item in &parsed.items {
        if item.title.trim().is_empty() {
            return Err(DomainError::Validation("建议条目缺少标题".into()));
        }
        if !item.ambiguous {
            if let Some(date) = item.due_date.as_deref() {
                if !date.is_empty()
                    && chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
                {
                    return Err(DomainError::Validation(format!("日期格式非法: {date}")));
                }
            }
            if let Some(time) = item.due_time.as_deref() {
                if !time.is_empty() && chrono::NaiveTime::parse_from_str(time, "%H:%M").is_err() {
                    return Err(DomainError::Validation(format!("时间格式非法: {time}")));
                }
            }
        }
        if item.source_excerpt.trim().is_empty() {
            return Err(DomainError::Validation("建议条目缺少原文引用".into()));
        }
    }

    Ok(parsed)
}

/// Apply selected items of an extract suggestion (slice 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractApplyInput {
    pub suggestion_id: String,
    /// Indices into `payload.items`; deduplicated, must be in range.
    pub selected_indices: Vec<usize>,
}

impl ExtractApplyInput {
    /// Sorted + deduplicated indices; errors on empty or out-of-range.
    pub fn normalize(&self, items_len: usize) -> Result<Vec<usize>, DomainError> {
        if self.selected_indices.is_empty() {
            return Err(DomainError::Validation("至少选择一条建议".into()));
        }
        let mut unique: Vec<usize> = self
            .selected_indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if let Some(&max) = unique.last() {
            if max >= items_len {
                return Err(DomainError::Validation("选择超出建议范围".into()));
            }
        }
        if unique.is_empty() {
            return Err(DomainError::Validation("至少选择一条建议".into()));
        }
        Ok(unique)
    }
}

/// What the user needs after applying: created tasks (jumpable) + the
/// finalized suggestion record (audit state).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractApplyResult {
    pub tasks: Vec<super::Task>,
    pub suggestion: AISuggestionRecord,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_config_defaults_to_off_and_disabled() {
        let config = AIConfig::default();
        assert_eq!(config.mode, AIMode::Off);
        assert!(!config.features.enabled(AIFeature::Extract));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn custom_mode_requires_endpoint_and_model() {
        let mut config = AIConfig::default();
        config.mode = AIMode::Custom;
        assert!(config.validate().is_err());
        config.custom_endpoint = "https://api.example.com".into();
        assert!(config.validate().is_err());
        config.custom_model = "gpt-test".into();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn unopened_features_have_no_prompt() {
        assert!(AIFeature::Extract.prompt_template().is_some());
        assert!(AIFeature::Summary.prompt_template().is_some());
        assert!(AIFeature::Related.prompt_template().is_some());
        assert!(AIFeature::Suggest.prompt_template().is_some());
        assert!(AIFeature::Split.prompt_template().is_none());
    }

    #[test]
    fn summary_prompt_forbids_judgement_words() {
        let prompt = AIFeature::Summary.prompt_template().unwrap();
        assert!(prompt.contains("严禁评价"));
        assert!(prompt.contains("落后")); // banned-words list is explicit
        assert!(prompt.contains("200 字"));
    }

    #[test]
    fn related_prompt_pins_exact_title_copying() {
        let prompt = AIFeature::Related.prompt_template().expect("opened");
        assert!(prompt.contains("完全一致"));
        assert!(prompt.contains("不得编造候选列表之外"));
    }

    #[test]
    fn suggest_prompt_pins_feature_citation() {
        let prompt = AIFeature::Suggest.prompt_template().expect("opened");
        assert!(prompt.contains("必须基于特征"));
        assert!(prompt.contains("严禁编造"));
        assert!(prompt.contains("宁可不选"));
    }

    #[test]
    fn summary_only_output_validates_via_existing_schema() {
        let raw = r#"{"items":[],"summary":"本周完成 3 项，逾期 1 项。"}"#;
        let content = parse_suggestion_content(raw).unwrap();
        assert!(content.items.is_empty());
        assert_eq!(content.summary.as_deref(), Some("本周完成 3 项，逾期 1 项。"));
    }

    #[test]
    fn completion_request_truncates_by_chars() {
        let req = CompletionRequest::new("sys", "短文本");
        assert_eq!(req.truncated_context(), "短文本");
    }

    #[test]
    fn parse_accepts_well_formed_output() {
        let raw = r#"{"items":[{"title":"确认合同","detail":null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":"找老张确认合同"}],"summary":null}"#;
        let content = parse_suggestion_content(raw).unwrap();
        assert_eq!(content.items.len(), 1);
        assert!(content.items[0].ambiguous);
    }

    #[test]
    fn parse_accepts_fenced_json() {
        let raw = "```json\n{\"items\":[],\"summary\":\"本周完成 3 项\"}\n```";
        let content = parse_suggestion_content(raw).unwrap();
        assert_eq!(content.summary.as_deref(), Some("本周完成 3 项"));
    }

    #[test]
    fn parse_rejects_bad_date_when_not_ambiguous() {
        let raw = r#"{"items":[{"title":"x","detail":null,"dueDate":"明天","dueTime":null,"ambiguous":false,"sourceExcerpt":"x"}],"summary":null}"#;
        assert!(parse_suggestion_content(raw).is_err());
    }

    #[test]
    fn parse_allows_bad_date_when_ambiguous() {
        // Unparsed dates are permitted only when flagged ambiguous; the UI
        // marks them for confirmation instead of guessing (nl_parse parity).
        let raw = r#"{"items":[{"title":"x","detail":null,"dueDate":"下周?","dueTime":null,"ambiguous":true,"sourceExcerpt":"x"}],"summary":null}"#;
        assert!(parse_suggestion_content(raw).is_ok());
    }

    #[test]
    fn parse_rejects_empty_everything() {
        assert!(parse_suggestion_content(r#"{"items":[],"summary":""}"#).is_err());
        assert!(parse_suggestion_content(r#"{"items":[],"summary":null}"#).is_err());
        assert!(parse_suggestion_content("not json").is_err());
    }

    #[test]
    fn apply_input_normalizes_indices() {
        let input = ExtractApplyInput {
            suggestion_id: "s1".into(),
            selected_indices: vec![2, 0, 2],
        };
        assert_eq!(input.normalize(3).unwrap(), vec![0, 2]);
        assert!(input.normalize(2).is_err(), "index 2 out of range");
        let empty = ExtractApplyInput {
            suggestion_id: "s1".into(),
            selected_indices: vec![],
        };
        assert!(empty.normalize(3).is_err());
    }

    #[test]
    fn parse_rejects_missing_excerpt() {
        let raw = r#"{"items":[{"title":"x","detail":null,"dueDate":null,"dueTime":null,"ambiguous":false,"sourceExcerpt":""}],"summary":null}"#;
        assert!(parse_suggestion_content(raw).is_err());
    }
}
