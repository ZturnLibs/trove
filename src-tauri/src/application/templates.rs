use crate::domain::{
    new_id, stamp, CreateMemoryInput, CreateReminderInput, CreateTaskInput, DomainError, EntityId,
    RecurrenceRule, SystemClock, TaskPriority,
};
use crate::infrastructure::db::Database;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TemplateKind {
    Task,
    Reminder,
    Memory,
}

impl TemplateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Reminder => "reminder",
            Self::Memory => "memory",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "task" => Ok(Self::Task),
            "reminder" => Ok(Self::Reminder),
            "memory" => Ok(Self::Memory),
            _ => Err(DomainError::Validation("无效模板类型".into())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemTemplate {
    pub id: EntityId,
    pub kind: TemplateKind,
    pub name: String,
    pub payload: Value,
    pub created_at: String,
    pub updated_at: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTemplateInput {
    pub kind: TemplateKind,
    pub name: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePreview {
    pub kind: TemplateKind,
    pub title: String,
    pub body: String,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub fire_at: Option<String>,
    pub priority: Option<TaskPriority>,
    pub recurrence: Option<RecurrenceRule>,
    pub tag_names: Vec<String>,
}

pub struct TemplateService {
    db: Database,
    clock: SystemClock,
}

impl TemplateService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn create(&self, input: CreateTemplateInput) -> Result<ItemTemplate, DomainError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("模板名称不能为空".into()));
        }
        let id = new_id();
        let now = stamp(&self.clock);
        let payload = serde_json::to_string(&input.payload).map_err(internal)?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO item_templates (id, kind, name, payload_json, created_at, updated_at, revision, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1, NULL)",
            params![id.to_string(), input.kind.as_str(), name, payload, now],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn list(&self) -> Result<Vec<ItemTemplate>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, name, payload_json, created_at, updated_at, revision
                 FROM item_templates WHERE deleted_at IS NULL
                 ORDER BY updated_at DESC",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([], map_template)
            .map_err(internal)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(internal)?);
        }
        Ok(out)
    }

    pub fn get(&self, id: EntityId) -> Result<ItemTemplate, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, kind, name, payload_json, created_at, updated_at, revision
             FROM item_templates WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_template,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("模板不存在".into()))
    }

    pub fn delete(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE item_templates SET deleted_at = ?1, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }

    pub fn preview(&self, id: EntityId) -> Result<TemplatePreview, DomainError> {
        let template = self.get(id)?;
        resolve_preview(&template)
    }

    pub fn to_task_input(&self, id: EntityId) -> Result<CreateTaskInput, DomainError> {
        let preview = self.preview(id)?;
        if preview.kind != TemplateKind::Task {
            return Err(DomainError::Validation("不是任务模板".into()));
        }
        Ok(CreateTaskInput {
            title: preview.title,
            notes: Some(preview.body).filter(|s| !s.is_empty()),
            priority: preview.priority,
            list_id: None,
            due_date: preview.due_date,
            due_time: preview.due_time,
            tag_names: if preview.tag_names.is_empty() {
                None
            } else {
                Some(preview.tag_names)
            },
        })
    }

    pub fn to_reminder_input(&self, id: EntityId) -> Result<CreateReminderInput, DomainError> {
        let preview = self.preview(id)?;
        if preview.kind != TemplateKind::Reminder {
            return Err(DomainError::Validation("不是提醒模板".into()));
        }
        let fire_at = preview
            .fire_at
            .ok_or_else(|| DomainError::Validation("提醒模板缺少时间".into()))?;
        Ok(CreateReminderInput {
            title: preview.title,
            notes: Some(preview.body).filter(|s| !s.is_empty()),
            task_id: None,
            fire_at,
            timezone: Some("Asia/Shanghai".into()),
            recurrence: preview.recurrence,
            end_at: None,
        })
    }

    pub fn to_memory_input(&self, id: EntityId) -> Result<CreateMemoryInput, DomainError> {
        let preview = self.preview(id)?;
        if preview.kind != TemplateKind::Memory {
            return Err(DomainError::Validation("不是记忆模板".into()));
        }
        Ok(CreateMemoryInput {
            title: preview.title,
            body: Some(preview.body).filter(|s| !s.is_empty()),
            pinned: None,
            quick_insert: None,
            trigger_word: None,
            tag_names: if preview.tag_names.is_empty() {
                None
            } else {
                Some(preview.tag_names)
            },
        })
    }
}

fn resolve_preview(template: &ItemTemplate) -> Result<TemplatePreview, DomainError> {
    let payload = &template.payload;
    let title = payload
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(template.name.as_str())
        .to_string();
    let body = payload
        .get("body")
        .or_else(|| payload.get("notes"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let priority = payload
        .get("priority")
        .and_then(|v| v.as_str())
        .and_then(|s| TaskPriority::parse(s).ok());
    let tag_names = payload
        .get("tagNames")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let today = chrono::Local::now().date_naive();
    let due_date = match payload.get("relativeDueDays").and_then(|v| v.as_i64()) {
        Some(days) => Some((today + chrono::Duration::days(days)).format("%Y-%m-%d").to_string()),
        None => payload
            .get("dueDate")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    };
    let due_time = payload
        .get("dueTime")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let fire_at = match payload.get("relativeFireHours").and_then(|v| v.as_i64()) {
        Some(hours) => {
            let dt = chrono::Local::now() + chrono::Duration::hours(hours);
            Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        }
        None => payload
            .get("fireAt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    };

    let recurrence = payload
        .get("recurrence")
        .cloned()
        .and_then(|v| serde_json::from_value::<RecurrenceRule>(v).ok());

    Ok(TemplatePreview {
        kind: template.kind,
        title,
        body,
        due_date,
        due_time,
        fire_at,
        priority,
        recurrence,
        tag_names,
    })
}

fn map_template(row: &rusqlite::Row<'_>) -> Result<ItemTemplate, rusqlite::Error> {
    let kind = TemplateKind::parse(&row.get::<_, String>(1)?).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let payload_raw: String = row.get(3)?;
    let payload: Value = serde_json::from_str(&payload_raw).unwrap_or(Value::Object(Default::default()));
    Ok(ItemTemplate {
        id: row.get::<_, String>(0)?.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        kind,
        name: row.get(2)?,
        payload,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        revision: row.get(6)?,
    })
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn relative_due_days_resolved_on_preview() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("t.db")).unwrap();
        let svc = TemplateService::new(db);
        let tpl = svc
            .create(CreateTemplateInput {
                kind: TemplateKind::Task,
                name: "周报".into(),
                payload: serde_json::json!({
                    "title": "写周报",
                    "relativeDueDays": 1,
                    "priority": "high"
                }),
            })
            .unwrap();
        let preview = svc.preview(tpl.id).unwrap();
        assert_eq!(preview.title, "写周报");
        assert!(preview.due_date.is_some());
        assert_eq!(preview.priority, Some(TaskPriority::High));
    }
}
