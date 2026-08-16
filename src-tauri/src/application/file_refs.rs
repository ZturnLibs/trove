use std::path::{Path, PathBuf};
use std::process::Command;

use crate::application::links::EntityLinkService;
use crate::domain::{
    new_id, stamp, DomainError, EntityId, FileReference, LinkedFileReference, SystemClock,
    LINK_KIND_ATTACHMENT,
};
use crate::infrastructure::db::Database;
use crate::platform::file_bookmark::{create_bookmark, resolve_bookmark};
use rusqlite::{params, OptionalExtension};

pub struct FileReferenceService {
    db: Database,
    links: EntityLinkService,
    clock: SystemClock,
}

impl FileReferenceService {
    pub fn new(db: Database) -> Self {
        Self {
            links: EntityLinkService::new(db.clone()),
            db,
            clock: SystemClock,
        }
    }

    pub fn pick_and_create(&self) -> Result<Option<FileReference>, DomainError> {
        let picked = rfd::FileDialog::new().pick_file();
        let Some(path) = picked else {
            return Ok(None);
        };
        self.create_from_path(&path).map(Some)
    }

    pub fn create_from_path(&self, path: &Path) -> Result<FileReference, DomainError> {
        let path_hint = path.to_string_lossy().to_string();
        if path_hint.is_empty() {
            return Err(DomainError::Validation("无效文件路径".into()));
        }
        let display_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("文件")
            .to_string();
        let mime_type = mime_guess::from_path(path).first_raw().map(str::to_string);
        let byte_size = std::fs::metadata(path).ok().map(|m| m.len() as i64);
        let accessible = path.exists();
        let bookmark = create_bookmark(&path_hint).map(|b| b.bytes);

        let id = new_id();
        let now = stamp(&self.clock);
        let conn = self.db.connect().map_err(internal)?;
        conn.execute(
            "INSERT INTO file_references (
                id, display_name, path_hint, mime_type, byte_size, bookmark,
                accessible, created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1, NULL)",
            params![
                id.to_string(),
                display_name,
                path_hint,
                mime_type,
                byte_size,
                bookmark,
                if accessible { 1 } else { 0 },
                now,
            ],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn get(&self, id: EntityId) -> Result<FileReference, DomainError> {
        let conn = self.db.connect().map_err(internal)?;
        conn.query_row(
            "SELECT id, display_name, path_hint, mime_type, byte_size, accessible,
                    created_at, updated_at, revision
             FROM file_references WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_row,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("文件引用不存在".into()))
    }

    pub fn attach(
        &self,
        source_type: &str,
        source_id: EntityId,
        file_id: EntityId,
    ) -> Result<LinkedFileReference, DomainError> {
        let file = self.get(file_id)?;
        let link = self.links.link(
            source_type,
            source_id,
            "file_ref",
            file_id,
            LINK_KIND_ATTACHMENT,
        )?;
        Ok(LinkedFileReference {
            link_id: link.id,
            file,
        })
    }

    pub fn list_for_entity(
        &self,
        source_type: &str,
        source_id: EntityId,
    ) -> Result<Vec<LinkedFileReference>, DomainError> {
        let links = self
            .links
            .list_outgoing(source_type, source_id)?
            .into_iter()
            .filter(|l| l.target_type == "file_ref" && l.link_kind == LINK_KIND_ATTACHMENT)
            .collect::<Vec<_>>();
        let mut out = Vec::with_capacity(links.len());
        for link in links {
            if let Ok(file) = self.get(link.target_id) {
                out.push(LinkedFileReference {
                    link_id: link.id,
                    file,
                });
            }
        }
        Ok(out)
    }

    pub fn resolve_path(&self, id: EntityId) -> Result<PathBuf, DomainError> {
        let conn = self.db.connect().map_err(internal)?;
        let (path_hint, bookmark): (String, Option<Vec<u8>>) = conn
            .query_row(
                "SELECT path_hint, bookmark FROM file_references WHERE id = ?1 AND deleted_at IS NULL",
                [id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(internal)?
            .ok_or_else(|| DomainError::NotFound("文件引用不存在".into()))?;

        if let Some(data) = bookmark {
            if let Some(resolved) = resolve_bookmark(&data) {
                let path = PathBuf::from(resolved);
                if path.exists() {
                    return Ok(path);
                }
            }
        }
        let path = PathBuf::from(path_hint);
        if path.exists() {
            Ok(path)
        } else {
            Err(DomainError::NotFound("文件不可访问，请重新选择文件".into()))
        }
    }

    pub fn refresh_accessibility(&self, id: EntityId) -> Result<FileReference, DomainError> {
        let accessible = self.resolve_path(id).is_ok();
        let conn = self.db.connect().map_err(internal)?;
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE file_references SET accessible = ?1, updated_at = ?2, revision = revision + 1
             WHERE id = ?3 AND deleted_at IS NULL",
            params![if accessible { 1 } else { 0 }, now, id.to_string()],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn open(&self, id: EntityId) -> Result<(), DomainError> {
        open_path(&self.resolve_path(id)?)
    }

    pub fn reveal(&self, id: EntityId) -> Result<(), DomainError> {
        reveal_path(&self.resolve_path(id)?)
    }

    pub fn relink(&self, id: EntityId) -> Result<Option<FileReference>, DomainError> {
        let picked = rfd::FileDialog::new().pick_file();
        let Some(path) = picked else {
            return Ok(None);
        };
        let path_hint = path.to_string_lossy().to_string();
        let display_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("文件")
            .to_string();
        let byte_size = std::fs::metadata(&path).ok().map(|m| m.len() as i64);
        let bookmark = create_bookmark(&path_hint).map(|b| b.bytes);
        let accessible = path.exists();
        let now = stamp(&self.clock);
        let conn = self.db.connect().map_err(internal)?;
        conn.execute(
            "UPDATE file_references SET display_name = ?1, path_hint = ?2, byte_size = ?3,
             bookmark = ?4, accessible = ?5, updated_at = ?6, revision = revision + 1
             WHERE id = ?7 AND deleted_at IS NULL",
            params![
                display_name,
                path_hint,
                byte_size,
                bookmark,
                if accessible { 1 } else { 0 },
                now,
                id.to_string(),
            ],
        )
        .map_err(internal)?;
        self.get(id).map(Some)
    }
}

fn open_path(path: &Path) -> Result<(), DomainError> {
    #[cfg(target_os = "macos")]
    {
        if Command::new("open").arg(path).status().map_err(internal)?.success() {
            return Ok(());
        }
    }
    #[cfg(target_os = "windows")]
    {
        if Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .status()
            .map_err(internal)?
            .success()
        {
            return Ok(());
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = path;
    Err(DomainError::Internal("无法打开文件".into()))
}

fn reveal_path(path: &Path) -> Result<(), DomainError> {
    #[cfg(target_os = "macos")]
    {
        if Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .status()
            .map_err(internal)?
            .success()
        {
            return Ok(());
        }
    }
    #[cfg(target_os = "windows")]
    {
        if Command::new("explorer")
            .args(["/select,", &path.to_string_lossy()])
            .status()
            .map_err(internal)?
            .success()
        {
            return Ok(());
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = path;
    Err(DomainError::Internal("无法在文件管理器中显示".into()))
}

fn map_row(row: &rusqlite::Row<'_>) -> Result<FileReference, rusqlite::Error> {
    Ok(FileReference {
        id: row.get::<_, String>(0)?.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        display_name: row.get(1)?,
        path_hint: row.get(2)?,
        mime_type: row.get(3)?,
        byte_size: row.get(4)?,
        accessible: row.get::<_, i64>(5)? == 1,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        revision: row.get(8)?,
    })
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}
