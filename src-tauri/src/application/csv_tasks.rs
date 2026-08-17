use crate::application::tasks::TaskService;
use crate::domain::{
    new_id, stamp, validate_due_date, validate_due_time, CreateTaskInput, DomainError, EntityId,
    SystemClock, TaskPriority, TaskStatus,
};
use crate::infrastructure::db::Database;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

pub const CSV_COLUMNS: &[&str] = &[
    "title",
    "notes",
    "status",
    "priority",
    "list",
    "due_date",
    "due_time",
    "tags",
];

const MAX_PREVIEW_ERRORS: usize = 20;
const MAX_IMPORT_ROWS: usize = 5000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CsvFieldMapping {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub list: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvRowIssue {
    pub row: i64,
    pub title: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvSampleRow {
    pub title: String,
    pub list: Option<String>,
    pub due_date: Option<String>,
    pub priority: String,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvPreview {
    pub headers: Vec<String>,
    pub mapping: CsvFieldMapping,
    pub row_count: i64,
    pub valid_count: i64,
    pub duplicate_count: i64,
    pub error_count: i64,
    pub errors: Vec<CsvRowIssue>,
    pub duplicates: Vec<CsvRowIssue>,
    pub unmapped_lists: Vec<String>,
    pub sample: Vec<CsvSampleRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportInput {
    pub csv: String,
    #[serde(default)]
    pub skip_duplicates: bool,
    pub mapping: Option<CsvFieldMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportResult {
    pub batch_id: EntityId,
    pub created: i64,
    pub skipped: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatch {
    pub id: EntityId,
    pub source: String,
    pub created: i64,
    pub skipped: i64,
    pub status: String,
    pub created_at: String,
    pub undone_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvUndoResult {
    pub deleted: i64,
    pub kept: i64,
}

struct PreparedRow {
    title: String,
    notes: Option<String>,
    status: TaskStatus,
    priority: Option<TaskPriority>,
    list_name: Option<String>,
    due_date: Option<String>,
    due_time: Option<String>,
    tags: Option<Vec<String>>,
    duplicate: bool,
}

pub struct CsvTaskService {
    db: Database,
    tasks: TaskService,
    clock: SystemClock,
}

impl CsvTaskService {
    pub fn new(db: Database) -> Self {
        Self {
            tasks: TaskService::new(db.clone()),
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn export_csv(&self) -> Result<String, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT t.title, t.notes, t.status, t.priority, l.name,
                        t.due_date, t.due_time,
                        (SELECT GROUP_CONCAT(tg.name, ';')
                           FROM task_tags tt
                           JOIN tags tg ON tg.id = tt.tag_id
                          WHERE tt.task_id = t.id)
                 FROM tasks t
                 JOIN task_lists l ON l.id = t.list_id
                 WHERE t.deleted_at IS NULL
                 ORDER BY t.created_at ASC",
            )
            .map_err(internal)?;
        let mut rows = stmt.query([]).map_err(internal)?;
        let mut buf = Vec::new();
        {
            let mut writer = csv::Writer::from_writer(&mut buf);
            writer.write_record(CSV_COLUMNS).map_err(internal)?;
            while let Some(row) = rows.next().map_err(internal)? {
                let title: String = row.get(0).map_err(internal)?;
                let notes: String = row.get(1).unwrap_or_default();
                let status: String = row.get(2).map_err(internal)?;
                let priority: String = row.get(3).map_err(internal)?;
                let list: String = row.get(4).map_err(internal)?;
                let due_date: Option<String> = row.get(5).map_err(internal)?;
                let due_time: Option<String> = row.get(6).map_err(internal)?;
                let tags: Option<String> = row.get(7).map_err(internal)?;
                writer
                    .write_record([
                        title,
                        notes,
                        status,
                        priority,
                        list,
                        due_date.unwrap_or_default(),
                        due_time.unwrap_or_default(),
                        tags.unwrap_or_default(),
                    ])
                    .map_err(internal)?;
            }
            writer.flush().map_err(internal)?;
        }
        String::from_utf8(buf).map_err(internal)
    }

    pub fn preview(&self, csv: &str, mapping: Option<CsvFieldMapping>) -> Result<CsvPreview, DomainError> {
        let (headers, prepared, mapping) = self.parse_csv(csv, mapping)?;
        let mut errors = Vec::new();
        let mut duplicates = Vec::new();
        let mut sample = Vec::new();
        let mut unmapped = HashSet::new();
        let lists = self.list_name_map()?;
        let mut valid = 0i64;
        let mut dup = 0i64;
        let mut err = 0i64;

        for (idx, row) in prepared.into_iter().enumerate() {
            let row_no = (idx as i64) + 2;
            match row {
                Err(message) => {
                    err += 1;
                    if errors.len() < MAX_PREVIEW_ERRORS {
                        errors.push(CsvRowIssue {
                            row: row_no,
                            title: None,
                            message,
                        });
                    }
                }
                Ok(item) => {
                    if let Some(name) = &item.list_name {
                        if !lists.contains_key(&name.to_ascii_lowercase()) {
                            unmapped.insert(name.clone());
                        }
                    }
                    if item.duplicate {
                        dup += 1;
                        if duplicates.len() < MAX_PREVIEW_ERRORS {
                            duplicates.push(CsvRowIssue {
                                row: row_no,
                                title: Some(item.title.clone()),
                                message: "与现有任务标题和截止日期相同".into(),
                            });
                        }
                    } else {
                        valid += 1;
                    }
                    if sample.len() < 5 {
                        sample.push(CsvSampleRow {
                            title: item.title,
                            list: item.list_name,
                            due_date: item.due_date,
                            priority: item
                                .priority
                                .unwrap_or(TaskPriority::None)
                                .as_str()
                                .into(),
                            duplicate: item.duplicate,
                        });
                    }
                }
            }
        }

        let mut unmapped_lists: Vec<String> = unmapped.into_iter().collect();
        unmapped_lists.sort();
        Ok(CsvPreview {
            headers,
            mapping,
            row_count: valid + dup + err,
            valid_count: valid,
            duplicate_count: dup,
            error_count: err,
            errors,
            duplicates,
            unmapped_lists,
            sample,
        })
    }

    pub fn import(&self, input: CsvImportInput) -> Result<CsvImportResult, DomainError> {
        let preview = self.preview(&input.csv, input.mapping.clone())?;
        if preview.error_count > 0 {
            return Err(DomainError::Validation(format!(
                "CSV 有 {} 行错误，请修正后再导入",
                preview.error_count
            )));
        }
        if preview.duplicate_count > 0 && !input.skip_duplicates {
            return Err(DomainError::Validation(format!(
                "发现 {} 条重复任务；勾选跳过重复后再导入",
                preview.duplicate_count
            )));
        }

        let (_, prepared, mapping) = self.parse_csv(&input.csv, input.mapping)?;
        let lists = self.list_name_map()?;
        let inbox = self.tasks.inbox_list_id()?;
        let mut created_ids: Vec<EntityId> = Vec::new();
        let mut skipped = 0i64;

        for row in prepared {
            let Ok(item) = row else { continue };
            if item.duplicate {
                skipped += 1;
                continue;
            }
            let list_id = item
                .list_name
                .as_ref()
                .and_then(|name| lists.get(&name.to_ascii_lowercase()).copied())
                .unwrap_or(inbox);
            match self.tasks.create_task(CreateTaskInput {
                title: item.title,
                notes: item.notes,
                priority: item.priority,
                list_id: Some(list_id),
                due_date: item.due_date,
                due_time: item.due_time,
                tag_names: item.tags,
            }) {
                Ok(task) => {
                    if item.status == TaskStatus::Completed {
                        let _ = self.tasks.complete_task(task.id);
                    } else if item.status == TaskStatus::Archived {
                        let _ = self.tasks.archive_task(task.id);
                    }
                    created_ids.push(task.id);
                }
                Err(err) => {
                    for id in &created_ids {
                        let _ = self.tasks.delete_task(*id);
                    }
                    return Err(DomainError::Internal(format!(
                        "导入中断，已回滚：{}",
                        sanitize(&err.to_string())
                    )));
                }
            }
        }

        let batch_id = self.record_batch(&mapping, created_ids.len() as i64, skipped, &created_ids)?;
        Ok(CsvImportResult {
            batch_id,
            created: created_ids.len() as i64,
            skipped,
        })
    }

    pub fn list_batches(&self) -> Result<Vec<ImportBatch>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, source, stats_json, status, created_at, undone_at
                 FROM import_batches ORDER BY created_at DESC LIMIT 20",
            )
            .map_err(internal)?;
        let rows = stmt.query_map([], map_batch).map_err(internal)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(internal)?);
        }
        Ok(out)
    }

    pub fn undo_batch(&self, id: EntityId) -> Result<CsvUndoResult, DomainError> {
        let conn = self.connect()?;
        let (status, ids_json): (String, String) = conn
            .query_row(
                "SELECT status, task_ids_json FROM import_batches WHERE id = ?1",
                [id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(internal)?
            .ok_or_else(|| DomainError::NotFound("导入批次不存在".into()))?;
        if status != "applied" {
            return Err(DomainError::Validation("该批次已撤销或不可撤销".into()));
        }
        let ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
        let mut deleted = 0i64;
        let mut kept = 0i64;
        for id_str in ids {
            let Ok(task_id) = id_str.parse::<EntityId>() else {
                kept += 1;
                continue;
            };
            match self.tasks.get_task(task_id) {
                Ok(task) if task.revision <= 2 => {
                    self.tasks.delete_task(task_id)?;
                    deleted += 1;
                }
                Ok(_) => kept += 1,
                Err(_) => kept += 1,
            }
        }
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE import_batches SET status = 'undone', undone_at = ?1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        Ok(CsvUndoResult { deleted, kept })
    }

    fn record_batch(
        &self,
        mapping: &CsvFieldMapping,
        created: i64,
        skipped: i64,
        ids: &[EntityId],
    ) -> Result<EntityId, DomainError> {
        let id = new_id();
        let now = stamp(&self.clock);
        let mapping_json = serde_json::to_string(mapping).map_err(internal)?;
        let stats_json = serde_json::to_string(&serde_json::json!({
            "created": created,
            "skipped": skipped,
        }))
        .map_err(internal)?;
        let id_strs: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
        let task_ids_json = serde_json::to_string(&id_strs).map_err(internal)?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO import_batches
             (id, source, mapping_json, stats_json, task_ids_json, status, created_at, undone_at)
             VALUES (?1, 'csv', ?2, ?3, ?4, 'applied', ?5, NULL)",
            params![id.to_string(), mapping_json, stats_json, task_ids_json, now],
        )
        .map_err(internal)?;
        Ok(id)
    }

    fn list_name_map(&self) -> Result<HashMap<String, EntityId>, DomainError> {
        let lists = self.tasks.list_lists()?;
        Ok(lists
            .into_iter()
            .map(|l| (l.name.to_ascii_lowercase(), l.id))
            .collect())
    }

    fn existing_keys(&self) -> Result<HashSet<String>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT lower(title), COALESCE(due_date, '') FROM tasks WHERE deleted_at IS NULL",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(format!(
                    "{}|{}",
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?
                ))
            })
            .map_err(internal)?;
        let mut set = HashSet::new();
        for row in rows {
            set.insert(row.map_err(internal)?);
        }
        Ok(set)
    }

    fn parse_csv(
        &self,
        csv: &str,
        mapping: Option<CsvFieldMapping>,
    ) -> Result<(Vec<String>, Vec<Result<PreparedRow, String>>, CsvFieldMapping), DomainError> {
        if csv.trim().is_empty() {
            return Err(DomainError::Validation("CSV 为空".into()));
        }
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(Cursor::new(csv.as_bytes()));
        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| DomainError::Validation(format!("无法读取表头: {e}")))?
            .iter()
            .map(|h| h.trim().to_string())
            .collect();
        if headers.is_empty() {
            return Err(DomainError::Validation("CSV 没有表头".into()));
        }
        let mapping = mapping.unwrap_or_else(|| detect_mapping(&headers));
        if mapping.title.is_none() {
            return Err(DomainError::Validation(
                "未识别标题列（title / 标题 / 任务）".into(),
            ));
        }
        let existing = self.existing_keys()?;
        let mut prepared = Vec::new();
        for (idx, record) in reader.records().enumerate() {
            if idx >= MAX_IMPORT_ROWS {
                return Err(DomainError::Validation(format!(
                    "超过 {MAX_IMPORT_ROWS} 行上限"
                )));
            }
            let record = match record {
                Ok(r) => r,
                Err(e) => {
                    prepared.push(Err(format!("无法解析该行: {e}")));
                    continue;
                }
            };
            prepared.push(prepare_row(&headers, &record, &mapping, &existing));
        }
        Ok((headers, prepared, mapping))
    }
}

fn detect_mapping(headers: &[String]) -> CsvFieldMapping {
    CsvFieldMapping {
        title: find_header(headers, &["title", "任务", "标题", "name", "task"]),
        notes: find_header(headers, &["notes", "备注", "说明", "description", "desc"]),
        status: find_header(headers, &["status", "状态"]),
        priority: find_header(headers, &["priority", "优先级"]),
        list: find_header(headers, &["list", "清单", "list_name", "listname"]),
        due_date: find_header(headers, &["due_date", "duedate", "due", "截止日期", "到期"]),
        due_time: find_header(headers, &["due_time", "duetime", "截止时间"]),
        tags: find_header(headers, &["tags", "标签", "tag"]),
    }
}

fn find_header(headers: &[String], aliases: &[&str]) -> Option<String> {
    headers.iter().find(|h| {
        let key = normalize_header(h);
        aliases.iter().any(|a| key == normalize_header(a))
    }).cloned()
}

fn normalize_header(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
}

fn cell(headers: &[String], record: &csv::StringRecord, header: &Option<String>) -> String {
    let Some(name) = header else { return String::new() };
    let Some(idx) = headers.iter().position(|h| h == name) else {
        return String::new();
    };
    record.get(idx).unwrap_or("").trim().to_string()
}

fn prepare_row(
    headers: &[String],
    record: &csv::StringRecord,
    mapping: &CsvFieldMapping,
    existing: &HashSet<String>,
) -> Result<PreparedRow, String> {
    let title = cell(headers, record, &mapping.title);
    if title.is_empty() {
        return Err("标题为空".into());
    }
    let notes = nonempty(cell(headers, record, &mapping.notes));
    let due_date = nonempty(cell(headers, record, &mapping.due_date));
    if let Some(ref due) = due_date {
        validate_due_date(due).map_err(|e| e.to_string())?;
    }
    let due_time = nonempty(cell(headers, record, &mapping.due_time));
    if let Some(ref time) = due_time {
        validate_due_time(time).map_err(|e| e.to_string())?;
    }
    let status = parse_status(&cell(headers, record, &mapping.status))?;
    let priority = parse_priority(&cell(headers, record, &mapping.priority))?;
    let list_name = nonempty(cell(headers, record, &mapping.list));
    let tags = parse_tags(&cell(headers, record, &mapping.tags));
    let key = format!("{}|{}", title.to_ascii_lowercase(), due_date.clone().unwrap_or_default());
    Ok(PreparedRow {
        title,
        notes,
        status,
        priority,
        list_name,
        due_date,
        due_time,
        tags,
        duplicate: existing.contains(&key),
    })
}

fn nonempty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn parse_status(raw: &str) -> Result<TaskStatus, String> {
    if raw.is_empty() {
        return Ok(TaskStatus::Todo);
    }
    match raw.to_ascii_lowercase().as_str() {
        "todo" | "待办" | "open" | "incomplete" => Ok(TaskStatus::Todo),
        "completed" | "done" | "完成" | "已完成" => Ok(TaskStatus::Completed),
        "archived" | "归档" => Ok(TaskStatus::Archived),
        other => Err(format!("无法识别状态: {other}")),
    }
}

fn parse_priority(raw: &str) -> Result<Option<TaskPriority>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    match raw.to_ascii_lowercase().as_str() {
        "none" | "无" | "0" => Ok(Some(TaskPriority::None)),
        "low" | "低" | "1" => Ok(Some(TaskPriority::Low)),
        "medium" | "中" | "2" => Ok(Some(TaskPriority::Medium)),
        "high" | "高" | "3" => Ok(Some(TaskPriority::High)),
        other => Err(format!("无法识别优先级: {other}")),
    }
}

fn parse_tags(raw: &str) -> Option<Vec<String>> {
    if raw.is_empty() {
        return None;
    }
    let tags: Vec<String> = raw
        .split([';', ',', '，', '、'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if tags.is_empty() { None } else { Some(tags) }
}

fn map_batch(row: &rusqlite::Row<'_>) -> Result<ImportBatch, rusqlite::Error> {
    let stats_raw: String = row.get(2)?;
    let stats: serde_json::Value = serde_json::from_str(&stats_raw).unwrap_or_default();
    Ok(ImportBatch {
        id: row.get::<_, String>(0)?.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        source: row.get(1)?,
        created: stats.get("created").and_then(|v| v.as_i64()).unwrap_or(0),
        skipped: stats.get("skipped").and_then(|v| v.as_i64()).unwrap_or(0),
        status: row.get(3)?,
        created_at: row.get(4)?,
        undone_at: row.get(5)?,
    })
}

fn sanitize(message: &str) -> String {
    message.chars().take(240).collect()
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup() -> (tempfile::TempDir, Database, CsvTaskService) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("t.db")).unwrap();
        TaskService::new(db.clone()).ensure_seed_data().unwrap();
        let svc = CsvTaskService::new(db.clone());
        (dir, db, svc)
    }

    #[test]
    fn export_roundtrip_preview_and_import() {
        let (_dir, db, svc) = setup();
        TaskService::new(db)
            .create_task(CreateTaskInput {
                title: "Buy milk".into(),
                notes: Some("2L".into()),
                priority: Some(TaskPriority::High),
                list_id: None,
                due_date: Some("2026-08-20".into()),
                due_time: None,
                tag_names: Some(vec!["errands".into()]),
            })
            .unwrap();

        let csv = svc.export_csv().unwrap();
        assert!(csv.contains("title"));
        assert!(csv.contains("Buy milk"));

        let preview = svc.preview(&csv, None).unwrap();
        assert!(preview.row_count >= 1);
        assert!(preview.duplicate_count >= 1);
        assert_eq!(preview.error_count, 0);
        assert!(preview.mapping.title.is_some());

        let (_fresh_dir, _fresh_db, fresh) = setup();
        let result = fresh
            .import(CsvImportInput {
                csv,
                skip_duplicates: false,
                mapping: None,
            })
            .unwrap();
        assert!(result.created >= 1);

        let batches = fresh.list_batches().unwrap();
        assert_eq!(batches.len(), 1);
        let undo = fresh.undo_batch(batches[0].id).unwrap();
        assert!(undo.deleted >= 1);
    }

    #[test]
    fn chinese_headers_and_invalid_status() {
        let (_dir, _db, svc) = setup();
        let csv = "标题,状态\n周报,不明\n";
        let preview = svc.preview(csv, None).unwrap();
        assert_eq!(preview.error_count, 1);
        assert!(preview.errors[0].message.contains("状态"));
    }

    #[test]
    fn empty_title_is_error() {
        let (_dir, _db, svc) = setup();
        let csv = "title,notes\n,x\n";
        let preview = svc.preview(csv, None).unwrap();
        assert_eq!(preview.error_count, 1);
    }
}
