use serde::{Deserialize, Serialize};

use super::DomainError;

pub const MAX_URL_LEN: usize = 2048;
pub const MAX_TITLE_LEN: usize = 500;
pub const MAX_NOTES_LEN: usize = 5000;
pub const MAX_SEARCH_QUERY_LEN: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UrlCreateKind {
    Task,
    Reminder,
    Memory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum UrlSchemeAction {
    Navigate { path: String },
    Search { query: String },
    CreatePreview {
        kind: UrlCreateKind,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        due_date: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fire_at: Option<String>,
    },
}

pub fn parse_trove_url(raw: &str) -> Result<UrlSchemeAction, DomainError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(DomainError::Validation("URL 为空".into()));
    }
    if raw.len() > MAX_URL_LEN {
        return Err(DomainError::Validation(format!(
            "URL 超过 {MAX_URL_LEN} 字符上限"
        )));
    }

    let parsed = url::Url::parse(raw).map_err(|_| DomainError::Validation("URL 格式无效".into()))?;
    if parsed.scheme() != "trove" {
        return Err(DomainError::Validation("仅支持 trove:// scheme".into()));
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Err(DomainError::Validation("URL 含非法认证信息".into()));
    }
    if parsed.host_str().is_some_and(|h| h.contains(':')) {
        return Err(DomainError::Validation("URL host 无效".into()));
    }

    let action_key = parsed
        .host_str()
        .map(str::to_ascii_lowercase)
        .or_else(|| {
            let path = parsed.path().trim_matches('/');
            if path.is_empty() {
                None
            } else {
                Some(path.split('/').next()?.to_ascii_lowercase())
            }
        })
        .ok_or_else(|| DomainError::Validation("缺少动作路径".into()))?;

    match action_key.as_str() {
        "today" => Ok(UrlSchemeAction::Navigate {
            path: "/today".into(),
        }),
        "inbox" => Ok(UrlSchemeAction::Navigate {
            path: "/inbox".into(),
        }),
        "search" => {
            let query = parsed
                .query_pairs()
                .find(|(k, _)| k == "q")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default();
            validate_search_query(&query)?;
            Ok(UrlSchemeAction::Search { query })
        }
        "create" => parse_create_action(&parsed),
        other => Err(DomainError::Validation(format!("未知动作: {other}"))),
    }
}

fn parse_create_action(parsed: &url::Url) -> Result<UrlSchemeAction, DomainError> {
    let kind_raw = query_param(parsed, "type")
        .ok_or_else(|| DomainError::Validation("create 缺少 type 参数".into()))?;
    let kind = match kind_raw.to_ascii_lowercase().as_str() {
        "task" => UrlCreateKind::Task,
        "reminder" => UrlCreateKind::Reminder,
        "memory" => UrlCreateKind::Memory,
        other => return Err(DomainError::Validation(format!("不支持的 type: {other}"))),
    };

    let title = query_param(parsed, "title")
        .ok_or_else(|| DomainError::Validation("create 缺少 title 参数".into()))?;
    validate_title(&title)?;

    let notes = query_param(parsed, "notes")
        .or_else(|| query_param(parsed, "body"))
        .map(|value| validate_notes(&value).map(|_| value))
        .transpose()?;

    let due_date = query_param(parsed, "dueDate")
        .or_else(|| query_param(parsed, "due"))
        .map(|value| validate_due_date(&value).map(|_| value))
        .transpose()?;

    let fire_at = query_param(parsed, "fireAt")
        .or_else(|| query_param(parsed, "fire"))
        .map(|value| validate_fire_at(&value).map(|_| value))
        .transpose()?;

    if matches!(kind, UrlCreateKind::Reminder) && fire_at.is_none() {
        // Optional at parse time; application layer may fill a default before preview.
    }

    Ok(UrlSchemeAction::CreatePreview {
        kind,
        title,
        notes,
        due_date,
        fire_at,
    })
}

fn query_param(parsed: &url::Url, key: &str) -> Option<String> {
    parsed
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

fn validate_title(title: &str) -> Result<(), DomainError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Validation("title 不能为空".into()));
    }
    if trimmed.len() > MAX_TITLE_LEN {
        return Err(DomainError::Validation(format!(
            "title 超过 {MAX_TITLE_LEN} 字符上限"
        )));
    }
    Ok(())
}

fn validate_notes(notes: &str) -> Result<(), DomainError> {
    if notes.len() > MAX_NOTES_LEN {
        return Err(DomainError::Validation(format!(
            "notes 超过 {MAX_NOTES_LEN} 字符上限"
        )));
    }
    Ok(())
}

fn validate_search_query(query: &str) -> Result<(), DomainError> {
    if query.len() > MAX_SEARCH_QUERY_LEN {
        return Err(DomainError::Validation(format!(
            "搜索词超过 {MAX_SEARCH_QUERY_LEN} 字符上限"
        )));
    }
    Ok(())
}

fn validate_due_date(value: &str) -> Result<(), DomainError> {
    if value.len() > 32 {
        return Err(DomainError::Validation("dueDate 格式无效".into()));
    }
    if chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
        return Err(DomainError::Validation("dueDate 须为 YYYY-MM-DD".into()));
    }
    Ok(())
}

fn validate_fire_at(value: &str) -> Result<(), DomainError> {
    if value.len() > 32 {
        return Err(DomainError::Validation("fireAt 格式无效".into()));
    }
    if chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").is_err()
        && chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").is_err()
    {
        return Err(DomainError::Validation(
            "fireAt 须为 YYYY-MM-DDTHH:MM:SS 或 YYYY-MM-DD HH:MM".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_navigate_today() {
        let action = parse_trove_url("trove://today").unwrap();
        assert_eq!(
            action,
            UrlSchemeAction::Navigate {
                path: "/today".into()
            }
        );
    }

    #[test]
    fn parse_navigate_inbox() {
        let action = parse_trove_url("trove://inbox").unwrap();
        assert_eq!(
            action,
            UrlSchemeAction::Navigate {
                path: "/inbox".into()
            }
        );
    }

    #[test]
    fn parse_search_with_query() {
        let action = parse_trove_url("trove://search?q=hello%20world").unwrap();
        assert_eq!(
            action,
            UrlSchemeAction::Search {
                query: "hello world".into()
            }
        );
    }

    #[test]
    fn parse_create_task() {
        let action =
            parse_trove_url("trove://create?type=task&title=Buy%20milk&notes=2%25").unwrap();
        assert_eq!(
            action,
            UrlSchemeAction::CreatePreview {
                kind: UrlCreateKind::Task,
                title: "Buy milk".into(),
                notes: Some("2%".into()),
                due_date: None,
                fire_at: None,
            }
        );
    }

    #[test]
    fn rejects_wrong_scheme() {
        assert!(parse_trove_url("https://evil.com").is_err());
    }

    #[test]
    fn rejects_unknown_action() {
        assert!(parse_trove_url("trove://exec?cmd=rm").is_err());
    }

    #[test]
    fn rejects_oversized_title() {
        let title = "a".repeat(MAX_TITLE_LEN + 1);
        let url = format!("trove://create?type=task&title={title}");
        assert!(parse_trove_url(&url).is_err());
    }

    #[test]
    fn rejects_invalid_create_type() {
        assert!(parse_trove_url("trove://create?type=shell&title=x").is_err());
    }
}
