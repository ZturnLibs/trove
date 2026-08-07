use crate::domain::{
    format_local_datetime, local_now_naive, local_today, new_id, next_after, parse_local_datetime,
    stamp, CreateReminderInput, DomainError, EntityId, OccurrenceStatus, Reminder,
    ReminderOccurrence, SnoozePreset, SystemClock, TodayReminderItem, UpdateReminderInput,
};
use crate::infrastructure::db::Database;
use chrono::{Duration, Local, NaiveDateTime};
use rusqlite::{params, Connection, OptionalExtension};

pub struct ReminderService {
    db: Database,
    clock: SystemClock,
}

impl ReminderService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn create(&self, input: CreateReminderInput) -> Result<Reminder, DomainError> {
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(DomainError::Validation("标题不能为空".into()));
        }
        let fire_at = parse_local_datetime(&input.fire_at)?;
        if let Some(ref rule) = input.recurrence {
            rule.validate()?;
        }
        let timezone = input
            .timezone
            .unwrap_or_else(|| Local::now().offset().to_string());
        let timezone = if timezone.is_empty() {
            "local".into()
        } else {
            timezone
        };

        let id = new_id();
        let now = stamp(&self.clock);
        let recurrence_json = input.recurrence.as_ref().map(|r| r.to_json()).transpose()?;

        let conn = self.connect()?;
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "INSERT INTO reminders (
                id, title, notes, task_id, recurrence_json, timezone, next_fire_at, end_at,
                enabled, created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?9, 1, NULL)",
            params![
                id.to_string(),
                title,
                input.notes.unwrap_or_default(),
                input.task_id.map(|v| v.to_string()),
                recurrence_json,
                timezone,
                format_local_datetime(fire_at),
                input.end_at,
                now,
            ],
        )
        .map_err(internal)?;

        self.ensure_occurrence(&tx, id, fire_at)?;
        tx.commit().map_err(internal)?;
        self.get(id)
    }

    pub fn update(&self, input: UpdateReminderInput) -> Result<Reminder, DomainError> {
        let _ = self.get(input.id)?;
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(DomainError::Validation("标题不能为空".into()));
        }
        let fire_at = parse_local_datetime(&input.fire_at)?;
        if let Some(ref rule) = input.recurrence {
            rule.validate()?;
        }
        let recurrence_json = input.recurrence.as_ref().map(|r| r.to_json()).transpose()?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE reminders SET
                title = ?1,
                notes = ?2,
                recurrence_json = ?3,
                next_fire_at = ?4,
                end_at = ?5,
                enabled = ?6,
                updated_at = ?7,
                revision = revision + 1
             WHERE id = ?8 AND deleted_at IS NULL",
            params![
                title,
                input.notes,
                recurrence_json,
                format_local_datetime(fire_at),
                input.end_at,
                if input.enabled { 1 } else { 0 },
                now,
                input.id.to_string(),
            ],
        )
        .map_err(internal)?;

        // Cancel future pending/scheduled occurrences and recreate for new fire time.
        tx.execute(
            "UPDATE reminder_occurrences SET status = 'cancelled', updated_at = ?1, revision = revision + 1
             WHERE reminder_id = ?2 AND status IN ('pending', 'scheduled', 'snoozed')",
            params![now, input.id.to_string()],
        )
        .map_err(internal)?;
        if input.enabled {
            self.ensure_occurrence(&tx, input.id, fire_at)?;
        }
        tx.commit().map_err(internal)?;
        self.get(input.id)
    }

    pub fn get(&self, id: EntityId) -> Result<Reminder, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, title, notes, task_id, recurrence_json, timezone, next_fire_at, end_at,
                    enabled, created_at, updated_at, revision
             FROM reminders WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_reminder_row,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("提醒不存在".into()))
    }

    pub fn list_for_task(&self, task_id: EntityId) -> Result<Vec<Reminder>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, notes, task_id, recurrence_json, timezone, next_fire_at, end_at,
                        enabled, created_at, updated_at, revision
                 FROM reminders
                 WHERE task_id = ?1 AND deleted_at IS NULL
                 ORDER BY next_fire_at ASC",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([task_id.to_string()], map_reminder_row)
            .map_err(internal)?;
        collect(rows)
    }

    pub fn list_all(&self) -> Result<Vec<Reminder>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, notes, task_id, recurrence_json, timezone, next_fire_at, end_at,
                        enabled, created_at, updated_at, revision
                 FROM reminders
                 WHERE deleted_at IS NULL
                 ORDER BY next_fire_at ASC",
            )
            .map_err(internal)?;
        let rows = stmt.query_map([], map_reminder_row).map_err(internal)?;
        collect(rows)
    }

    pub fn delete(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE reminders SET deleted_at = ?1, updated_at = ?1, enabled = 0, revision = revision + 1
             WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        conn.execute(
            "UPDATE reminder_occurrences SET status = 'cancelled', updated_at = ?1, revision = revision + 1
             WHERE reminder_id = ?2 AND status IN ('pending', 'scheduled', 'snoozed')",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }

    pub fn today_items(&self) -> Result<Vec<TodayReminderItem>, DomainError> {
        let today = local_today(&self.clock);
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT o.id, o.reminder_id, o.scheduled_at, o.status, o.needs_schedule,
                        o.system_notification_id, o.actioned_at, o.snooze_until,
                        o.created_at, o.updated_at, o.revision,
                        r.title, r.notes, r.task_id, r.recurrence_json, r.timezone,
                        r.next_fire_at, r.end_at, r.enabled, r.created_at, r.updated_at, r.revision
                 FROM reminder_occurrences o
                 JOIN reminders r ON r.id = o.reminder_id
                 WHERE r.deleted_at IS NULL
                   AND o.status IN ('pending', 'scheduled', 'snoozed')
                   AND substr(o.scheduled_at, 1, 10) = ?1
                 ORDER BY o.scheduled_at ASC",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([&today], |row| {
                let occurrence = ReminderOccurrence {
                    id: parse_id(row.get(0)?)?,
                    reminder_id: parse_id(row.get(1)?)?,
                    scheduled_at: row.get(2)?,
                    status: OccurrenceStatus::parse(&row.get::<_, String>(3)?).map_err(map_sql)?,
                    needs_schedule: row.get::<_, i64>(4)? == 1,
                    system_notification_id: row.get(5)?,
                    actioned_at: row.get(6)?,
                    snooze_until: row.get(7)?,
                    title: row.get(11)?,
                    task_id: row
                        .get::<_, Option<String>>(13)?
                        .map(parse_id)
                        .transpose()?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    revision: row.get(10)?,
                };
                let reminder = Reminder {
                    id: parse_id(row.get(1)?)?,
                    title: row.get(11)?,
                    notes: row.get(12)?,
                    task_id: row
                        .get::<_, Option<String>>(13)?
                        .map(parse_id)
                        .transpose()?,
                    recurrence: row
                        .get::<_, Option<String>>(14)?
                        .map(|s| crate::domain::RecurrenceRule::from_json(&s))
                        .transpose()
                        .map_err(map_sql)?,
                    timezone: row.get(15)?,
                    next_fire_at: row.get(16)?,
                    end_at: row.get(17)?,
                    enabled: row.get::<_, i64>(18)? == 1,
                    created_at: row.get(19)?,
                    updated_at: row.get(20)?,
                    revision: row.get(21)?,
                };
                Ok(TodayReminderItem {
                    occurrence,
                    reminder,
                })
            })
            .map_err(internal)?;
        collect(rows)
    }

    pub fn due_occurrences(
        &self,
        now: NaiveDateTime,
    ) -> Result<Vec<ReminderOccurrence>, DomainError> {
        let now_s = format_local_datetime(now);
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT o.id, o.reminder_id, o.scheduled_at, o.status, o.needs_schedule,
                        o.system_notification_id, o.actioned_at, o.snooze_until,
                        o.created_at, o.updated_at, o.revision,
                        r.title, r.task_id
                 FROM reminder_occurrences o
                 JOIN reminders r ON r.id = o.reminder_id
                 WHERE r.deleted_at IS NULL AND r.enabled = 1
                   AND o.status IN ('pending', 'scheduled', 'snoozed')
                   AND (
                     (o.status = 'snoozed' AND o.snooze_until IS NOT NULL AND o.snooze_until <= ?1)
                     OR (o.status IN ('pending', 'scheduled') AND o.scheduled_at <= ?1)
                   )
                 ORDER BY o.scheduled_at ASC
                 LIMIT 50",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([&now_s], |row| {
                Ok(ReminderOccurrence {
                    id: parse_id(row.get(0)?)?,
                    reminder_id: parse_id(row.get(1)?)?,
                    scheduled_at: row.get(2)?,
                    status: OccurrenceStatus::parse(&row.get::<_, String>(3)?).map_err(map_sql)?,
                    needs_schedule: row.get::<_, i64>(4)? == 1,
                    system_notification_id: row.get(5)?,
                    actioned_at: row.get(6)?,
                    snooze_until: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    revision: row.get(10)?,
                    title: row.get(11)?,
                    task_id: row
                        .get::<_, Option<String>>(12)?
                        .map(parse_id)
                        .transpose()?,
                })
            })
            .map_err(internal)?;
        collect(rows)
    }

    pub fn mark_notified(
        &self,
        occurrence_id: EntityId,
        system_id: Option<i32>,
    ) -> Result<(), DomainError> {
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE reminder_occurrences SET
                status = 'scheduled',
                needs_schedule = 0,
                system_notification_id = ?1,
                updated_at = ?2,
                revision = revision + 1
             WHERE id = ?3",
            params![system_id, now, occurrence_id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }

    pub fn complete_occurrence(
        &self,
        occurrence_id: EntityId,
    ) -> Result<ReminderOccurrence, DomainError> {
        let occ = self.get_occurrence(occurrence_id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE reminder_occurrences SET
                status = 'actioned', actioned_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2",
            params![now, occurrence_id.to_string()],
        )
        .map_err(internal)?;

        let reminder = self.get_in_tx(&tx, occ.reminder_id)?;
        if let Some(rule) = reminder.recurrence.clone() {
            let current = parse_local_datetime(&occ.scheduled_at)?;
            if let Some(next) = next_after(&rule, current)? {
                tx.execute(
                    "UPDATE reminders SET next_fire_at = ?1, updated_at = ?2, revision = revision + 1
                     WHERE id = ?3",
                    params![
                        format_local_datetime(next),
                        now,
                        reminder.id.to_string()
                    ],
                )
                .map_err(internal)?;
                self.ensure_occurrence(&tx, reminder.id, next)?;
            } else {
                tx.execute(
                    "UPDATE reminders SET enabled = 0, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
                    params![now, reminder.id.to_string()],
                )
                .map_err(internal)?;
            }
        }
        tx.commit().map_err(internal)?;
        self.get_occurrence(occurrence_id)
    }

    pub fn snooze_occurrence(
        &self,
        occurrence_id: EntityId,
        preset: SnoozePreset,
    ) -> Result<ReminderOccurrence, DomainError> {
        let _ = self.get_occurrence(occurrence_id)?;
        let until = match preset {
            SnoozePreset::Minutes10 => local_now_naive() + Duration::minutes(10),
            SnoozePreset::Hour1 => local_now_naive() + Duration::hours(1),
            SnoozePreset::Tomorrow => {
                let now = local_now_naive();
                NaiveDateTime::new(
                    now.date() + Duration::days(1),
                    chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                )
            }
        };
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE reminder_occurrences SET
                status = 'snoozed',
                snooze_until = ?1,
                needs_schedule = 1,
                updated_at = ?2,
                revision = revision + 1
             WHERE id = ?3",
            params![format_local_datetime(until), now, occurrence_id.to_string()],
        )
        .map_err(internal)?;
        self.get_occurrence(occurrence_id)
    }

    /// On startup: one-shot missed -> mark for single catch-up; recurring missed many -> only latest.
    pub fn reconcile_on_startup(&self) -> Result<usize, DomainError> {
        let now = local_now_naive();
        let now_s = format_local_datetime(now);
        let stamp_now = stamp(&self.clock);
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction().map_err(internal)?;

        // Recurring: keep only latest overdue pending/scheduled per reminder; older -> inferred_missed
        let mut stmt = tx
            .prepare(
                "SELECT o.id, o.reminder_id, o.scheduled_at
                 FROM reminder_occurrences o
                 JOIN reminders r ON r.id = o.reminder_id
                 WHERE r.deleted_at IS NULL AND r.recurrence_json IS NOT NULL
                   AND o.status IN ('pending', 'scheduled')
                   AND o.scheduled_at < ?1
                 ORDER BY o.reminder_id, o.scheduled_at DESC",
            )
            .map_err(internal)?;
        let rows: Vec<(String, String, String)> = stmt
            .query_map([&now_s], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(internal)?
            .collect::<Result<_, _>>()
            .map_err(internal)?;
        drop(stmt);

        let mut seen = std::collections::HashSet::new();
        let mut catch_up = 0usize;
        for (id, reminder_id, _) in rows {
            if seen.insert(reminder_id) {
                catch_up += 1;
                continue;
            }
            tx.execute(
                "UPDATE reminder_occurrences SET status = 'inferred_missed', updated_at = ?1, revision = revision + 1
                 WHERE id = ?2",
                params![stamp_now, id],
            )
            .map_err(internal)?;
        }

        // One-shot overdue remain pending for a single catch-up notification.
        let oneshot: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM reminder_occurrences o
                 JOIN reminders r ON r.id = o.reminder_id
                 WHERE r.deleted_at IS NULL AND r.recurrence_json IS NULL
                   AND o.status IN ('pending', 'scheduled')
                   AND o.scheduled_at < ?1",
                [&now_s],
                |row| row.get(0),
            )
            .map_err(internal)?;
        catch_up += oneshot as usize;

        tx.commit().map_err(internal)?;
        Ok(catch_up)
    }

    pub fn get_occurrence(&self, id: EntityId) -> Result<ReminderOccurrence, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT o.id, o.reminder_id, o.scheduled_at, o.status, o.needs_schedule,
                    o.system_notification_id, o.actioned_at, o.snooze_until,
                    o.created_at, o.updated_at, o.revision, r.title, r.task_id
             FROM reminder_occurrences o
             JOIN reminders r ON r.id = o.reminder_id
             WHERE o.id = ?1",
            [id.to_string()],
            |row| {
                Ok(ReminderOccurrence {
                    id: parse_id(row.get(0)?)?,
                    reminder_id: parse_id(row.get(1)?)?,
                    scheduled_at: row.get(2)?,
                    status: OccurrenceStatus::parse(&row.get::<_, String>(3)?).map_err(map_sql)?,
                    needs_schedule: row.get::<_, i64>(4)? == 1,
                    system_notification_id: row.get(5)?,
                    actioned_at: row.get(6)?,
                    snooze_until: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    revision: row.get(10)?,
                    title: row.get(11)?,
                    task_id: row
                        .get::<_, Option<String>>(12)?
                        .map(parse_id)
                        .transpose()?,
                })
            },
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("提醒实例不存在".into()))
    }

    fn get_in_tx(&self, conn: &Connection, id: EntityId) -> Result<Reminder, DomainError> {
        conn.query_row(
            "SELECT id, title, notes, task_id, recurrence_json, timezone, next_fire_at, end_at,
                    enabled, created_at, updated_at, revision
             FROM reminders WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_reminder_row,
        )
        .map_err(internal)
    }

    fn ensure_occurrence(
        &self,
        conn: &Connection,
        reminder_id: EntityId,
        at: NaiveDateTime,
    ) -> Result<(), DomainError> {
        let id = new_id();
        let now = stamp(&self.clock);
        let scheduled = format_local_datetime(at);
        conn.execute(
            "INSERT OR IGNORE INTO reminder_occurrences (
                id, reminder_id, scheduled_at, status, needs_schedule,
                system_notification_id, actioned_at, snooze_until,
                created_at, updated_at, revision
             ) VALUES (?1, ?2, ?3, 'pending', 1, NULL, NULL, NULL, ?4, ?4, 1)",
            params![id.to_string(), reminder_id.to_string(), scheduled, now],
        )
        .map_err(internal)?;
        Ok(())
    }
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

fn map_sql(err: DomainError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(err.to_string())))
}

fn parse_id(value: String) -> Result<EntityId, rusqlite::Error> {
    value.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn map_reminder_row(row: &rusqlite::Row<'_>) -> Result<Reminder, rusqlite::Error> {
    Ok(Reminder {
        id: parse_id(row.get(0)?)?,
        title: row.get(1)?,
        notes: row.get(2)?,
        task_id: row.get::<_, Option<String>>(3)?.map(parse_id).transpose()?,
        recurrence: row
            .get::<_, Option<String>>(4)?
            .map(|s| crate::domain::RecurrenceRule::from_json(&s))
            .transpose()
            .map_err(map_sql)?,
        timezone: row.get(5)?,
        next_fire_at: row.get(6)?,
        end_at: row.get(7)?,
        enabled: row.get::<_, i64>(8)? == 1,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        revision: row.get(11)?,
    })
}

fn collect<T, E>(rows: impl IntoIterator<Item = Result<T, E>>) -> Result<Vec<T>, DomainError>
where
    E: std::fmt::Display,
{
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(internal)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RecurrenceFrequency, RecurrenceRule};
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    fn service() -> ReminderService {
        service_with_db().0
    }

    fn service_with_db() -> (ReminderService, Database) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("r.db")).unwrap();
        // seed inbox via tasks migration tables only — create list manually
        let conn = db.connect().unwrap();
        let now = stamp(&SystemClock);
        conn.execute(
            "INSERT INTO task_lists (id, name, kind, sort_order, created_at, updated_at, revision)
             VALUES ('11111111-1111-4111-8111-111111111111', '收件箱', 'inbox', 0, ?1, ?1, 1)",
            [now],
        )
        .unwrap();
        drop(conn);
        std::mem::forget(dir);
        (ReminderService::new(db.clone()), db)
    }

    #[test]
    fn create_oneshot_and_complete() {
        let svc = service();
        let fire = format_local_datetime(local_now_naive() + Duration::minutes(5));
        let reminder = svc
            .create(CreateReminderInput {
                title: "standup".into(),
                notes: None,
                task_id: None,
                fire_at: fire,
                recurrence: None,
                timezone: Some("Asia/Shanghai".into()),
                end_at: None,
            })
            .unwrap();
        let items = svc.today_items().unwrap();
        assert!(items.iter().any(|i| i.reminder.id == reminder.id));
        let occ_id = items
            .iter()
            .find(|i| i.reminder.id == reminder.id)
            .unwrap()
            .occurrence
            .id;
        svc.complete_occurrence(occ_id).unwrap();
        let occ = svc.get_occurrence(occ_id).unwrap();
        assert_eq!(occ.status, OccurrenceStatus::Actioned);
    }

    #[test]
    fn recurring_advances_on_complete() {
        let svc = service();
        let fire =
            NaiveDateTime::parse_from_str("2026-07-29T09:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        let reminder = svc
            .create(CreateReminderInput {
                title: "daily".into(),
                notes: None,
                task_id: None,
                fire_at: format_local_datetime(fire),
                recurrence: Some(RecurrenceRule {
                    version: 1,
                    frequency: RecurrenceFrequency::Daily,
                    interval: 1,
                    weekdays: None,
                    monthday: None,
                    timezone: "Asia/Shanghai".into(),
                    end_at: None,
                }),
                timezone: Some("Asia/Shanghai".into()),
                end_at: None,
            })
            .unwrap();
        let occ = svc
            .due_occurrences(fire + Duration::minutes(1))
            .unwrap()
            .into_iter()
            .find(|o| o.reminder_id == reminder.id)
            .unwrap();
        svc.complete_occurrence(occ.id).unwrap();
        let updated = svc.get(reminder.id).unwrap();
        assert_eq!(&updated.next_fire_at[..10], "2026-07-30");
    }

    #[test]
    fn list_all_returns_every_reminder_sorted_including_disabled() {
        let svc = service();
        let fire_later = format_local_datetime(local_now_naive() + Duration::days(2));
        let fire_sooner = format_local_datetime(local_now_naive() + Duration::days(1));
        let later = svc
            .create(CreateReminderInput {
                title: "later".into(),
                notes: None,
                task_id: None,
                fire_at: fire_later,
                recurrence: None,
                timezone: Some("Asia/Shanghai".into()),
                end_at: None,
            })
            .unwrap();
        let sooner = svc
            .create(CreateReminderInput {
                title: "sooner".into(),
                notes: None,
                task_id: None,
                fire_at: fire_sooner.clone(),
                recurrence: None,
                timezone: Some("Asia/Shanghai".into()),
                end_at: None,
            })
            .unwrap();
        // Disable the sooner reminder — it must still be listed.
        svc.update(UpdateReminderInput {
            id: sooner.id,
            title: sooner.title.clone(),
            notes: sooner.notes.clone(),
            fire_at: fire_sooner,
            recurrence: sooner.recurrence.clone(),
            enabled: false,
            end_at: sooner.end_at.clone(),
        })
        .unwrap();

        let all = svc.list_all().unwrap();
        assert_eq!(all.len(), 2);
        // Ordered by next_fire_at ascending.
        assert_eq!(all[0].id, sooner.id);
        assert_eq!(all[1].id, later.id);
        assert!(!all[0].enabled);
        assert!(all[1].enabled);
    }

    #[test]
    fn reconcile_counts_oneshot_overdue_and_dedupes_recurring() {
        let (svc, db) = service_with_db();
        let now = local_now_naive();
        let stamp_now = stamp(&SystemClock);

        // One-shot overdue — stays pending for catch-up.
        let oneshot = svc
            .create(CreateReminderInput {
                title: "past".into(),
                notes: None,
                task_id: None,
                fire_at: format_local_datetime(now - Duration::hours(2)),
                recurrence: None,
                timezone: Some("Asia/Shanghai".into()),
                end_at: None,
            })
            .unwrap();

        // Recurring with two overdue occurrences; only latest should count.
        let recurring = svc
            .create(CreateReminderInput {
                title: "daily".into(),
                notes: None,
                task_id: None,
                fire_at: format_local_datetime(
                    NaiveDateTime::parse_from_str("2026-07-28T09:00:00", "%Y-%m-%dT%H:%M:%S")
                        .unwrap(),
                ),
                recurrence: Some(RecurrenceRule {
                    version: 1,
                    frequency: RecurrenceFrequency::Daily,
                    interval: 1,
                    weekdays: None,
                    monthday: None,
                    timezone: "Asia/Shanghai".into(),
                    end_at: None,
                }),
                timezone: Some("Asia/Shanghai".into()),
                end_at: None,
            })
            .unwrap();
        let conn = db.connect().unwrap();
        conn.execute(
            "INSERT INTO reminder_occurrences (
                id, reminder_id, scheduled_at, status, needs_schedule,
                system_notification_id, actioned_at, snooze_until,
                created_at, updated_at, revision
             ) VALUES (?1, ?2, ?3, 'pending', 1, NULL, NULL, NULL, ?4, ?4, 1)",
            params![
                new_id().to_string(),
                recurring.id.to_string(),
                "2026-07-29T09:00:00",
                stamp_now,
            ],
        )
        .unwrap();
        drop(conn);

        let catch_up = svc.reconcile_on_startup().unwrap();
        assert_eq!(catch_up, 2);

        let oneshot_occ = svc
            .today_items()
            .unwrap()
            .into_iter()
            .find(|i| i.reminder.id == oneshot.id)
            .map(|i| i.occurrence)
            .or_else(|| {
                svc.due_occurrences(now)
                    .unwrap()
                    .into_iter()
                    .find(|o| o.reminder_id == oneshot.id)
            })
            .expect("oneshot occurrence");
        assert_eq!(oneshot_occ.status, OccurrenceStatus::Pending);

        let recurring_occs: Vec<_> = svc
            .due_occurrences(
                NaiveDateTime::parse_from_str("2026-07-29T09:00:00", "%Y-%m-%dT%H:%M:%S")
                    .unwrap(),
            )
            .unwrap()
            .into_iter()
            .filter(|o| o.reminder_id == recurring.id)
            .collect();
        assert_eq!(recurring_occs.len(), 1);
        assert_eq!(recurring_occs[0].scheduled_at, "2026-07-29T09:00:00");

        let missed: i64 = db
            .connect()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM reminder_occurrences
                 WHERE reminder_id = ?1 AND status = 'inferred_missed'",
                [recurring.id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(missed, 1);
    }

    #[test]
    fn snoozed_occurrence_not_due_until_snooze_until() {
        let svc = service();
        let fire = format_local_datetime(local_now_naive() - Duration::minutes(5));
        let reminder = svc
            .create(CreateReminderInput {
                title: "snooze me".into(),
                notes: None,
                task_id: None,
                fire_at: fire,
                recurrence: None,
                timezone: Some("Asia/Shanghai".into()),
                end_at: None,
            })
            .unwrap();
        let occ_id = svc
            .due_occurrences(local_now_naive())
            .unwrap()
            .into_iter()
            .find(|o| o.reminder_id == reminder.id)
            .unwrap()
            .id;

        let snoozed = svc
            .snooze_occurrence(occ_id, SnoozePreset::Minutes10)
            .unwrap();
        assert_eq!(snoozed.status, OccurrenceStatus::Snoozed);
        let until = parse_local_datetime(snoozed.snooze_until.as_ref().unwrap()).unwrap();

        assert!(
            svc.due_occurrences(until - Duration::minutes(1))
                .unwrap()
                .iter()
                .all(|o| o.id != occ_id)
        );
        assert!(
            svc.due_occurrences(until + Duration::seconds(1))
                .unwrap()
                .iter()
                .any(|o| o.id == occ_id)
        );
    }
}
