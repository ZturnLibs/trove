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
        let expected = (width as usize).saturating_mul(height as usize).saturating_mul(4);
        if rgba.len() < expected {
            return Err(DomainError::Validation("图片像素数据不完整".into()));
        }

        let img: RgbaImage = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba[..expected].to_vec())
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

    #[test]
    fn dedupes_identical_png() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("a.db")).unwrap();
        let store = AssetStore::new(db, dir.path().join("assets"));
        let rgba = vec![255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255];
        let a = store.store_rgba_image(2, 2, &rgba).unwrap();
        let b = store.store_rgba_image(2, 2, &rgba).unwrap();
        assert_eq!(a.asset.id, b.asset.id);
        assert!(store.absolute_path(&a.asset).exists());
    }
}
