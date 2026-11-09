use crate::domain::{
    new_id, parse_uuid, stamp, validate_link_kind, DomainError, EntityId, EntityLink,
    LinkEntityType, SystemClock,
};
use crate::infrastructure::db::Database;
use rusqlite::{params, Connection, OptionalExtension};

pub struct EntityLinkService {
    db: Database,
    clock: SystemClock,
}

impl EntityLinkService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn link(
        &self,
        source_type: &str,
        source_id: EntityId,
        target_type: &str,
        target_id: EntityId,
        link_kind: &str,
    ) -> Result<EntityLink, DomainError> {
        LinkEntityType::parse(source_type)?;
        LinkEntityType::parse(target_type)?;
        validate_link_kind(link_kind)?;

        let conn = self.connect()?;
        let now = stamp(&self.clock);
        conn.execute(
            "INSERT OR IGNORE INTO entity_links
                (id, source_type, source_id, target_type, target_id, link_kind, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id().to_string(),
                source_type,
                source_id.to_string(),
                target_type,
                target_id.to_string(),
                link_kind,
                now
            ],
        )
        .map_err(internal)?;
        self.find_pair(
            &conn,
            source_type,
            source_id,
            target_type,
            target_id,
            link_kind,
        )?
        .ok_or_else(|| DomainError::Internal("关联写入后未能读取".into()))
    }

    pub fn unlink(&self, id: EntityId) -> Result<(), DomainError> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM entity_links WHERE id = ?1", [id.to_string()])
            .map_err(internal)?;
        Ok(())
    }

    pub fn purge_for_source(
        &self,
        source_type: &str,
        source_id: EntityId,
    ) -> Result<usize, DomainError> {
        let conn = self.connect()?;
        let count = conn
            .execute(
                "DELETE FROM entity_links WHERE source_type = ?1 AND source_id = ?2",
                params![source_type, source_id.to_string()],
            )
            .map_err(internal)?;
        Ok(count)
    }

    pub fn purge_incoming_for_target(
        &self,
        target_type: &str,
        target_id: EntityId,
    ) -> Result<usize, DomainError> {
        let conn = self.connect()?;
        let count = conn
            .execute(
                "DELETE FROM entity_links WHERE target_type = ?1 AND target_id = ?2",
                params![target_type, target_id.to_string()],
            )
            .map_err(internal)?;
        Ok(count)
    }

    pub fn list_outgoing(
        &self,
        source_type: &str,
        source_id: EntityId,
    ) -> Result<Vec<EntityLink>, DomainError> {
        let conn = self.connect()?;
        list(
            &conn,
            "source_type = ?1 AND source_id = ?2",
            &[source_type, &source_id.to_string()],
        )
    }

    pub fn list_incoming(
        &self,
        target_type: &str,
        target_id: EntityId,
    ) -> Result<Vec<EntityLink>, DomainError> {
        let conn = self.connect()?;
        list(
            &conn,
            "target_type = ?1 AND target_id = ?2",
            &[target_type, &target_id.to_string()],
        )
    }

    pub fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: EntityId,
    ) -> Result<Vec<EntityLink>, DomainError> {
        let conn = self.connect()?;
        list(
            &conn,
            "(source_type = ?1 AND source_id = ?2) OR (target_type = ?1 AND target_id = ?2)",
            &[entity_type, &entity_id.to_string()],
        )
    }

    pub fn is_referenced(
        &self,
        target_type: &str,
        target_id: EntityId,
    ) -> Result<bool, DomainError> {
        Ok(self.reference_count(target_type, target_id)? > 0)
    }

    pub fn reference_count(
        &self,
        target_type: &str,
        target_id: EntityId,
    ) -> Result<i64, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT COUNT(*) FROM entity_links WHERE target_type = ?1 AND target_id = ?2",
            params![target_type, target_id.to_string()],
            |row| row.get(0),
        )
        .map_err(internal)
    }

    fn find_pair(
        &self,
        conn: &Connection,
        source_type: &str,
        source_id: EntityId,
        target_type: &str,
        target_id: EntityId,
        link_kind: &str,
    ) -> Result<Option<EntityLink>, DomainError> {
        conn.query_row(
            "SELECT id, source_type, source_id, target_type, target_id, link_kind, created_at
             FROM entity_links
             WHERE source_type = ?1 AND source_id = ?2
               AND target_type = ?3 AND target_id = ?4 AND link_kind = ?5
             LIMIT 1",
            params![
                source_type,
                source_id.to_string(),
                target_type,
                target_id.to_string(),
                link_kind
            ],
            map_link,
        )
        .optional()
        .map_err(internal)
    }
}

fn list(
    conn: &Connection,
    where_clause: &str,
    args: &[&str],
) -> Result<Vec<EntityLink>, DomainError> {
    let sql = format!(
        "SELECT id, source_type, source_id, target_type, target_id, link_kind, created_at
         FROM entity_links WHERE {where_clause}"
    );
    let mut stmt = conn.prepare(&sql).map_err(internal)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter().copied()), map_link)
        .map_err(internal)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(internal)?);
    }
    Ok(out)
}

fn map_link(row: &rusqlite::Row<'_>) -> Result<EntityLink, rusqlite::Error> {
    Ok(EntityLink {
        id: row.get::<_, String>(0)?.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        source_type: row.get(1)?,
        source_id: parse_uuid(&row.get::<_, String>(2)?).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?,
        target_type: row.get(3)?,
        target_id: parse_uuid(&row.get::<_, String>(4)?).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?,
        link_kind: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn svc(dir: &std::path::Path) -> EntityLinkService {
        let db = Database::open(dir.join("links.db")).unwrap();
        EntityLinkService::new(db)
    }

    /// 附件链路回归：真实资产 → link(target=asset) → list_outgoing 命中，
    /// 且 assets.get(target) 可解析出缩略图（裸 base64，无 data URL 前缀）。
    /// 幽灵 target_id（如误传剪贴板条目 id）会被查询侧跳过——前端必须传资产 id。
    #[test]
    fn attach_asset_link_round_trip_resolves_asset() {
        use crate::application::assets::AssetStore;

        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("attach.db")).unwrap();
        let assets = AssetStore::new(db.clone(), dir.path().join("assets"));
        let links = EntityLinkService::new(db.clone());
        let task = uuid::Uuid::new_v4();

        let stored = assets
            .store_rgba_image(
                2,
                2,
                &[
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
                ],
            )
            .unwrap();
        links
            .link("task", task, "asset", stored.asset.id, "attachment")
            .unwrap();

        let outgoing = links.list_outgoing("task", task).unwrap();
        assert_eq!(outgoing.len(), 1);

        let asset = assets.get(stored.asset.id).unwrap();
        let thumb = assets.thumb_base64(&asset).unwrap();
        assert!(thumb.is_some());
        let thumb = thumb.unwrap();
        assert!(!thumb.starts_with("data:"));

        // 幽灵 target（不存在的资产 id）不可解析——查询侧会跳过此类链接。
        let ghost = uuid::Uuid::new_v4();
        assert!(assets.get(ghost).is_err());
    }

    #[test]
    fn link_is_idempotent() {
        let dir = tempdir().unwrap();
        let svc = svc(dir.path());
        let task = uuid::Uuid::new_v4();
        let asset = uuid::Uuid::new_v4();
        let a = svc
            .link("task", task, "asset", asset, "attachment")
            .unwrap();
        let b = svc
            .link("task", task, "asset", asset, "attachment")
            .unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(svc.reference_count("asset", asset).unwrap(), 1);
    }

    #[test]
    fn invalid_type_or_kind_rejected() {
        let dir = tempdir().unwrap();
        let svc = svc(dir.path());
        let id = uuid::Uuid::new_v4();
        assert!(svc.link("bogus", id, "asset", id, "attachment").is_err());
        assert!(svc.link("task", id, "asset", id, "bogus").is_err());
    }

    #[test]
    fn outgoing_incoming_unlink_purge() {
        let dir = tempdir().unwrap();
        let svc = svc(dir.path());
        let memory = uuid::Uuid::new_v4();
        let task = uuid::Uuid::new_v4();
        let asset = uuid::Uuid::new_v4();

        svc.link("memory", memory, "task", task, "converted_to")
            .unwrap();
        svc.link("memory", memory, "asset", asset, "attachment")
            .unwrap();

        let outgoing = svc.list_outgoing("memory", memory).unwrap();
        assert_eq!(outgoing.len(), 2);
        let incoming_asset = svc.list_incoming("asset", asset).unwrap();
        assert_eq!(incoming_asset.len(), 1);
        let incoming_task = svc.list_incoming("task", task).unwrap();
        assert_eq!(incoming_task.len(), 1);
        let both = svc.list_for_entity("memory", memory).unwrap();
        assert_eq!(both.len(), 2);
        assert!(svc.is_referenced("asset", asset).unwrap());

        let link_id = incoming_asset[0].id;
        svc.unlink(link_id).unwrap();
        assert!(!svc.is_referenced("asset", asset).unwrap());
        assert_eq!(svc.list_outgoing("memory", memory).unwrap().len(), 1);

        let purged = svc.purge_for_source("memory", memory).unwrap();
        assert_eq!(purged, 1);
        assert!(svc.list_outgoing("memory", memory).unwrap().is_empty());
    }
}
