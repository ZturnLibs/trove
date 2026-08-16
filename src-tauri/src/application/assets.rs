use crate::domain::{new_id, stamp, Asset, DomainError, EntityId, SystemClock};
use crate::infrastructure::db::Database;
use image::{imageops::FilterType, ImageBuffer, Rgba, RgbaImage};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const THUMB_MAX: u32 = 240;

pub struct StoredImage {
    pub asset: Asset,
    pub png_bytes: Vec<u8>,
    pub thumb_png: Vec<u8>,
}

pub struct AssetStore {
    db: Database,
    root: PathBuf,
    clock: SystemClock,
}

impl AssetStore {
    pub fn new(db: Database, root: PathBuf) -> Self {
        Self {
            db,
            root,
            clock: SystemClock,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn hash_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    pub fn store_rgba_image(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<StoredImage, DomainError> {
        if width == 0 || height == 0 {
            return Err(DomainError::Validation("无效图片尺寸".into()));
        }
        let expected = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        if rgba.len() < expected {
            return Err(DomainError::Validation("图片像素数据不完整".into()));
        }

        let img: RgbaImage =
            ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba[..expected].to_vec())
                .ok_or_else(|| DomainError::Internal("无法构建图片缓冲".into()))?;

        let mut png_bytes = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut png_bytes);
            img.write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| DomainError::Internal(format!("encode png: {e}")))?;
        }
        let hash = Self::hash_bytes(&png_bytes);

        if let Some(existing) = self.find_by_hash(&hash)? {
            let thumb_png = self.read_thumb_bytes(&existing)?;
            return Ok(StoredImage {
                png_bytes,
                thumb_png,
                asset: existing,
            });
        }

        let thumb = resize_thumb(&img);
        let mut thumb_png = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut thumb_png);
            thumb
                .write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| DomainError::Internal(format!("encode thumb: {e}")))?;
        }

        let rel = format!("clipboard/{hash}.png");
        let thumb_rel = format!("clipboard/thumbs/{hash}.png");
        let abs = self.root.join(&rel);
        let thumb_abs = self.root.join(&thumb_rel);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).map_err(io_err)?;
        }
        if let Some(parent) = thumb_abs.parent() {
            std::fs::create_dir_all(parent).map_err(io_err)?;
        }
        std::fs::write(&abs, &png_bytes).map_err(io_err)?;
        std::fs::write(&thumb_abs, &thumb_png).map_err(io_err)?;

        let id = new_id();
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO assets (
                id, kind, content_hash, relative_path, thumb_path, mime_type, byte_size,
                width, height, created_at, updated_at, revision, deleted_at
             ) VALUES (?1, 'image', ?2, ?3, ?4, 'image/png', ?5, ?6, ?7, ?8, ?8, 1, NULL)",
            params![
                id.to_string(),
                hash,
                rel,
                thumb_rel,
                png_bytes.len() as i64,
                width as i64,
                height as i64,
                now
            ],
        )
        .map_err(internal)?;

        Ok(StoredImage {
            png_bytes,
            thumb_png,
            asset: self.get(id)?,
        })
    }

    pub fn find_by_hash(&self, hash: &str) -> Result<Option<Asset>, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, kind, content_hash, relative_path, thumb_path, mime_type, byte_size,
                    width, height, created_at, updated_at, revision
             FROM assets WHERE content_hash = ?1 AND deleted_at IS NULL LIMIT 1",
            [hash],
            map_asset,
        )
        .optional()
        .map_err(internal)
    }

    pub fn get(&self, id: EntityId) -> Result<Asset, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, kind, content_hash, relative_path, thumb_path, mime_type, byte_size,
                    width, height, created_at, updated_at, revision
             FROM assets WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_asset,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("资源不存在".into()))
    }

    pub fn absolute_path(&self, asset: &Asset) -> PathBuf {
        self.root.join(&asset.relative_path)
    }

    pub fn read_bytes(&self, asset: &Asset) -> Result<Vec<u8>, DomainError> {
        std::fs::read(self.absolute_path(asset)).map_err(io_err)
    }

    pub fn read_thumb_bytes(&self, asset: &Asset) -> Result<Vec<u8>, DomainError> {
        let Some(rel) = asset.thumb_path.as_ref() else {
            return Ok(Vec::new());
        };
        let path = self.root.join(rel);
        if !path.exists() {
            return Ok(Vec::new());
        }
        std::fs::read(path).map_err(io_err)
    }

    pub fn thumb_base64(&self, asset: &Asset) -> Result<Option<String>, DomainError> {
        let bytes = self.read_thumb_bytes(asset)?;
        if bytes.is_empty() {
            return Ok(None);
        }
        Ok(Some(format!(
            "data:image/png;base64,{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
        )))
    }

    pub fn collect_garbage(&self, retention_days: u32) -> Result<GcSummary, DomainError> {
        let candidates = self.gc_candidates(retention_days)?;
        let mut removed = 0usize;
        let mut freed_bytes = 0i64;
        let conn = self.connect()?;
        for (id, rel, thumb_rel, byte_size) in candidates {
            conn.execute("DELETE FROM assets WHERE id = ?1", [&id])
                .map_err(internal)?;
            let _ = std::fs::remove_file(self.root.join(&rel));
            if let Some(thumb_rel) = thumb_rel {
                let _ = std::fs::remove_file(self.root.join(&thumb_rel));
            }
            removed += 1;
            freed_bytes += byte_size;
        }
        Ok(GcSummary {
            removed,
            freed_bytes,
        })
    }

    pub fn gc_preview(&self, retention_days: u32) -> Result<GcPreview, DomainError> {
        let candidates = self.gc_candidates(retention_days)?;
        let candidate_bytes = candidates.iter().map(|(_, _, _, size)| size).sum();
        Ok(GcPreview {
            candidate_count: candidates.len(),
            candidate_bytes,
            retention_days,
        })
    }

    fn gc_candidates(
        &self,
        retention_days: u32,
    ) -> Result<Vec<(String, String, Option<String>, i64)>, DomainError> {
        let cutoff = (chrono::Local::now() - chrono::Duration::days(retention_days as i64))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT a.id, a.relative_path, a.thumb_path, a.byte_size FROM assets a
                 WHERE a.deleted_at IS NULL
                   AND a.created_at < ?1
                   AND NOT EXISTS (SELECT 1 FROM entity_links el
                                   WHERE el.target_type = 'asset' AND el.target_id = a.id)
                   AND NOT EXISTS (SELECT 1 FROM clipboard_items ci
                                   WHERE ci.asset_id = a.id AND ci.deleted_at IS NULL)",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([&cutoff], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(internal)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(internal)?);
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GcSummary {
    pub removed: usize,
    pub freed_bytes: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GcPreview {
    pub candidate_count: usize,
    pub candidate_bytes: i64,
    pub retention_days: u32,
}

fn resize_thumb(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let max = w.max(h);
    if max <= THUMB_MAX {
        return img.clone();
    }
    let scale = THUMB_MAX as f32 / max as f32;
    let nw = ((w as f32) * scale).round().max(1.0) as u32;
    let nh = ((h as f32) * scale).round().max(1.0) as u32;
    image::imageops::resize(img, nw, nh, FilterType::Triangle)
}

fn map_asset(row: &rusqlite::Row<'_>) -> Result<Asset, rusqlite::Error> {
    Ok(Asset {
        id: row.get::<_, String>(0)?.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        kind: row.get(1)?,
        content_hash: row.get(2)?,
        relative_path: row.get(3)?,
        thumb_path: row.get(4)?,
        mime_type: row.get(5)?,
        byte_size: row.get(6)?,
        width: row.get(7)?,
        height: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        revision: row.get(11)?,
    })
}

fn io_err(err: std::io::Error) -> DomainError {
    DomainError::Internal(err.to_string())
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store(dir: &std::path::Path) -> AssetStore {
        let db = Database::open(dir.join("a.db")).unwrap();
        AssetStore::new(db, dir.join("assets"))
    }

    fn rgba() -> Vec<u8> {
        vec![
            255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ]
    }

    fn age_asset(db_path: &std::path::Path, asset_id: &str) {
        let db = Database::open(db_path).unwrap();
        let conn = db.connect().unwrap();
        conn.execute(
            "UPDATE assets SET created_at = '2000-01-01T00:00:00' WHERE id = ?1",
            [asset_id],
        )
        .unwrap();
    }

    #[test]
    fn dedupes_identical_png() {
        let dir = tempdir().unwrap();
        let store = store(dir.path());
        let a = store.store_rgba_image(2, 2, &rgba()).unwrap();
        let b = store.store_rgba_image(2, 2, &rgba()).unwrap();
        assert_eq!(a.asset.id, b.asset.id);
        assert!(store.absolute_path(&a.asset).exists());
    }

    #[test]
    fn gc_removes_orphan_asset_past_retention() {
        let dir = tempdir().unwrap();
        let store = store(dir.path());
        let a = store.store_rgba_image(2, 2, &rgba()).unwrap();
        age_asset(&dir.path().join("a.db"), &a.asset.id.to_string());
        let abs = store.absolute_path(&a.asset);
        assert!(abs.exists());

        let summary = store.collect_garbage(30).unwrap();
        assert_eq!(summary.removed, 1);
        assert!(summary.freed_bytes > 0);
        assert!(store.get(a.asset.id).is_err());
        assert!(!abs.exists());
    }

    #[test]
    fn gc_keeps_asset_within_retention() {
        let dir = tempdir().unwrap();
        let store = store(dir.path());
        let a = store.store_rgba_image(2, 2, &rgba()).unwrap();
        let summary = store.collect_garbage(30).unwrap();
        assert_eq!(summary.removed, 0);
        assert!(store.get(a.asset.id).is_ok());
    }

    #[test]
    fn gc_keeps_asset_referenced_by_entity_link() {
        let dir = tempdir().unwrap();
        let store = store(dir.path());
        let a = store.store_rgba_image(2, 2, &rgba()).unwrap();
        age_asset(&dir.path().join("a.db"), &a.asset.id.to_string());

        let db = Database::open(dir.path().join("a.db")).unwrap();
        let conn = db.connect().unwrap();
        conn.execute(
            "INSERT INTO entity_links
                (id, source_type, source_id, target_type, target_id, link_kind, created_at)
             VALUES ('l1', 'memory', 'm1', 'asset', ?1, 'attachment', '2026-01-01T00:00:00')",
            [a.asset.id.to_string()],
        )
        .unwrap();

        let summary = store.collect_garbage(30).unwrap();
        assert_eq!(summary.removed, 0);
        assert!(store.get(a.asset.id).is_ok());
    }

    #[test]
    fn gc_keeps_asset_referenced_by_active_clipboard_item() {
        let dir = tempdir().unwrap();
        let store = store(dir.path());
        let a = store.store_rgba_image(2, 2, &rgba()).unwrap();
        age_asset(&dir.path().join("a.db"), &a.asset.id.to_string());

        let db = Database::open(dir.path().join("a.db")).unwrap();
        let conn = db.connect().unwrap();
        conn.execute(
            "INSERT INTO clipboard_items
                (id, content, content_hash, source_app, favorite, use_count, last_used_at,
                 created_at, updated_at, revision, deleted_at, kind, asset_id)
             VALUES ('c1', '[图片]', 'h', NULL, 0, 0, NULL,
                     '2026-01-01T00:00:00', '2026-01-01T00:00:00', 1, NULL, 'image', ?1)",
            [a.asset.id.to_string()],
        )
        .unwrap();

        let summary = store.collect_garbage(30).unwrap();
        assert_eq!(summary.removed, 0);
        assert!(store.get(a.asset.id).is_ok());
    }
}
