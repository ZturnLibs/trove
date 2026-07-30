use crate::application::search::SearchService;
use crate::domain::DomainError;
use crate::infrastructure::db::Database;
use rusqlite::{types::ValueRef, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const EXPORT_FORMAT: &str = "workbench-export";
const EXPORT_VERSION: u32 = 1;

const EXPORT_TABLES: &[&str] = &[
    "settings",
    "smoke_notes",
    "task_lists",
    "tags",
    "task_series",
    "tasks",
    "task_tags",
    "reminders",
    "reminder_occurrences",
    "memories",
    "memory_tags",
    "assets",
    "derived_texts",
    "clipboard_items",
    "entity_links",
    "item_templates",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDocument {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub schema_version: i64,
    pub app_version: String,
    pub data: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub tables: usize,
    pub rows: usize,
}

pub struct DataPortService {
    db: Database,
    search: SearchService,
}

impl DataPortService {
    pub fn new(db: Database) -> Self {
        Self {
            search: SearchService::new(db.clone()),
            db,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn export_json(&self) -> Result<String, DomainError> {
        let conn = self.connect()?;
        let schema_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(internal)?;

        let mut data = Map::new();
        for table in EXPORT_TABLES {
            data.insert((*table).into(), Value::Array(dump_table(&conn, table)?));
        }

        let doc = ExportDocument {
            format: EXPORT_FORMAT.into(),
            version: EXPORT_VERSION,
            exported_at: chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
            schema_version,
            app_version: env!("CARGO_PKG_VERSION").into(),
            data,
        };
        serde_json::to_string_pretty(&doc).map_err(internal)
    }

    pub fn import_json(&self, raw: &str) -> Result<ImportResult, DomainError> {
        let doc: ExportDocument = serde_json::from_str(raw)
            .map_err(|e| DomainError::Validation(format!("无效的导出文件: {e}")))?;
        if doc.format != EXPORT_FORMAT {
            return Err(DomainError::Validation("不是工作台导出文件".into()));
        }
        if doc.version != EXPORT_VERSION {
            return Err(DomainError::Validation(format!(
                "不支持的导出版本: {}",
                doc.version
            )));
        }

        let conn = self.connect()?;
        let tx = conn.unchecked_transaction().map_err(internal)?;

        // Children first.
        for table in [
            "reminder_occurrences",
            "reminders",
            "memory_tags",
            "task_tags",
            "entity_links",
            "derived_texts",
            "clipboard_items",
            "assets",
            "memories",
            "item_templates",
            "tasks",
            "task_series",
            "tags",
            "task_lists",
            "smoke_notes",
            "settings",
        ] {
            tx.execute(&format!("DELETE FROM {table}"), [])
                .map_err(internal)?;
        }
        let _ = tx.execute_batch(
            "DELETE FROM search_index;
             DELETE FROM search_documents;",
        );

        let mut rows = 0usize;
        let mut tables = 0usize;
        for table in EXPORT_TABLES {
            let Some(value) = doc.data.get(*table) else {
                continue;
            };
            let Value::Array(items) = value else {
                return Err(DomainError::Validation(format!(
                    "表 {table} 数据格式错误"
                )));
            };
            tables += 1;
            for item in items {
                let Value::Object(obj) = item else {
                    continue;
                };
                insert_row(&tx, table, obj)?;
                rows += 1;
            }
        }

        tx.commit().map_err(internal)?;
        self.search.rebuild_all()?;
        Ok(ImportResult { tables, rows })
    }
}

fn dump_table(conn: &Connection, table: &str) -> Result<Vec<Value>, DomainError> {
    let mut stmt = conn
        .prepare(&format!("SELECT * FROM {table}"))
        .map_err(internal)?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut rows = stmt.query([]).map_err(internal)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(internal)? {
        let mut obj = Map::new();
        for (idx, name) in column_names.iter().enumerate() {
            obj.insert(name.clone(), sql_to_json(row.get_ref(idx).map_err(internal)?));
        }
        out.push(Value::Object(obj));
    }
    Ok(out)
}

fn insert_row(
    conn: &Connection,
    table: &str,
    obj: &Map<String, Value>,
) -> Result<(), DomainError> {
    if obj.is_empty() {
        return Ok(());
    }
    let columns: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        columns.join(", "),
        placeholders.join(", ")
    );
    let values: Vec<rusqlite::types::Value> = columns
        .iter()
        .map(|col| json_to_sql(obj.get(*col).unwrap_or(&Value::Null)))
        .collect();
    let params_as_refs: Vec<&dyn rusqlite::types::ToSql> =
        values.iter().map(|v| v as &dyn rusqlite::types::ToSql).collect();
    conn.execute(&sql, params_as_refs.as_slice())
        .map_err(|e| DomainError::Internal(format!("导入 {table} 失败: {e}")))?;
    Ok(())
}

fn sql_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => json!(v),
        ValueRef::Real(v) => json!(v),
        ValueRef::Text(v) => Value::String(String::from_utf8_lossy(v).into_owned()),
        ValueRef::Blob(v) => Value::String(format!("base64:{}", hex::encode(v))),
    }
}

fn json_to_sql(value: &Value) -> rusqlite::types::Value {
    match value {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(v) => rusqlite::types::Value::Integer(if *v { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Text(n.to_string())
            }
        }
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        other => rusqlite::types::Value::Text(other.to_string()),
    }
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::TaskService;
    use tempfile::tempdir;

    #[test]
    fn export_import_roundtrip() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("a.db")).unwrap();
        TaskService::new(db.clone()).ensure_seed_data().unwrap();
        let port = DataPortService::new(db.clone());
        {
            let conn = db.connect().unwrap();
            conn.execute(
                "INSERT INTO smoke_notes (id, body, created_at, updated_at, revision, deleted_at)
                 VALUES ('e1', 'export-me', 't', 't', 1, NULL)",
                [],
            )
            .unwrap();
        }

        let json = port.export_json().unwrap();
        assert!(json.contains("workbench-export"));

        let db2 = Database::open(dir.path().join("b.db")).unwrap();
        TaskService::new(db2.clone()).ensure_seed_data().unwrap();
        let port2 = DataPortService::new(db2.clone());
        let result = port2.import_json(&json).unwrap();
        assert!(result.rows > 0);

        let body: String = db2
            .connect()
            .unwrap()
            .query_row(
                "SELECT body FROM smoke_notes WHERE id = 'e1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(body, "export-me");
    }
}
