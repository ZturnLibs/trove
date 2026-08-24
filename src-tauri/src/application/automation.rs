use crate::application::memories::MemoryService;
use crate::application::tasks::TaskService;
use crate::domain::{
    evaluate_conditions, trigger_matches, validate_rule_definition, AutomationAction,
    AutomationDryRunResult, AutomationEntityType, AutomationEvent, AutomationEventKind,
    AutomationRun, AutomationRunStatus, AutomationRule, AutomationRuleDefinition,
    AutomationTrigger, ClipboardItem, CreateAutomationRuleInput, DomainError, EntityId, Memory,
    Reminder, ReminderOccurrence, SystemClock, Task, UpdateAutomationRuleInput,
    UpdateMemoryInput, UpdateTaskInput,
};
use crate::domain::{new_id, stamp};
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;
use rusqlite::{params, Connection, OptionalExtension};
use std::cell::RefCell;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

thread_local! {
    static AUTOMATION_DEPTH: RefCell<u32> = const { RefCell::new(0) };
}

pub struct AutomationService {
    db: Database,
    clock: SystemClock,
}

#[derive(Debug, Clone)]
pub struct AutomationRunOutcome {
    pub rule_id: EntityId,
    pub rule_name: String,
    pub status: AutomationRunStatus,
    pub actions_applied: Vec<AutomationAction>,
    pub error_summary: Option<String>,
}

impl AutomationService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn create(&self, input: CreateAutomationRuleInput) -> Result<AutomationRule, DomainError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("规则名称不能为空".into()));
        }
        validate_rule_definition(&input.definition)?;

        let id = new_id();
        let now = stamp(&self.clock);
        let definition_json =
            serde_json::to_string(&input.definition).map_err(|e| internal(e.to_string()))?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO automation_rules (id, name, enabled, definition_json, created_at, updated_at, revision, deleted_at)
             VALUES (?1, ?2, 1, ?3, ?4, ?5, 1, NULL)",
            params![id.to_string(), name, definition_json, now, now],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn list(&self) -> Result<Vec<AutomationRule>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, enabled, definition_json, created_at, updated_at, revision
                 FROM automation_rules WHERE deleted_at IS NULL
                 ORDER BY updated_at DESC",
            )
            .map_err(internal)?;
        let rows = stmt.query_map([], map_rule).map_err(internal)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(internal)?);
        }
        Ok(out)
    }

    pub fn get(&self, id: EntityId) -> Result<AutomationRule, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, name, enabled, definition_json, created_at, updated_at, revision
             FROM automation_rules WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_rule,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("规则不存在".into()))
    }

    pub fn update(&self, input: UpdateAutomationRuleInput) -> Result<AutomationRule, DomainError> {
        let _ = self.get(input.id)?;
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("规则名称不能为空".into()));
        }
        validate_rule_definition(&input.definition)?;

        let now = stamp(&self.clock);
        let definition_json =
            serde_json::to_string(&input.definition).map_err(|e| internal(e.to_string()))?;
        let conn = self.connect()?;
        conn.execute(
            "UPDATE automation_rules SET
                name = ?1,
                enabled = ?2,
                definition_json = ?3,
                updated_at = ?4,
                revision = revision + 1
             WHERE id = ?5 AND deleted_at IS NULL",
            params![
                name,
                input.enabled as i32,
                definition_json,
                now,
                input.id.to_string()
            ],
        )
        .map_err(internal)?;
        self.get(input.id)
    }

    pub fn set_enabled(&self, id: EntityId, enabled: bool) -> Result<AutomationRule, DomainError> {
        let rule = self.get(id)?;
        self.update(UpdateAutomationRuleInput {
            id,
            name: rule.name,
            enabled,
            definition: rule.definition,
        })
    }

    pub fn delete(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE automation_rules SET deleted_at = ?1, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }

    pub fn list_runs(&self, rule_id: Option<EntityId>, limit: i64) -> Result<Vec<AutomationRun>, DomainError> {
        let limit = limit.clamp(1, 200);
        let conn = self.connect()?;
        if let Some(rule_id) = rule_id {
            let mut stmt = conn
                .prepare(
                    "SELECT id, rule_id, rule_name, entity_type, entity_id, status,
                            actions_applied_json, error_summary, dry_run, created_at
                     FROM automation_runs WHERE rule_id = ?1
                     ORDER BY created_at DESC LIMIT ?2",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map(params![rule_id.to_string(), limit], map_run)
                .map_err(internal)?;
            collect_runs(rows)
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, rule_id, rule_name, entity_type, entity_id, status,
                            actions_applied_json, error_summary, dry_run, created_at
                     FROM automation_runs
                     ORDER BY created_at DESC LIMIT ?1",
                )
                .map_err(internal)?;
            let rows = stmt.query_map([limit], map_run).map_err(internal)?;
            collect_runs(rows)
        }
    }

    pub fn dry_run(
        &self,
        rule_id: EntityId,
        event: AutomationEvent,
    ) -> Result<AutomationDryRunResult, DomainError> {
        let rule = self.get(rule_id)?;
        Ok(self.evaluate_rule(&rule, &event))
    }

    fn evaluate_rule(&self, rule: &AutomationRule, event: &AutomationEvent) -> AutomationDryRunResult {
        if !rule.enabled {
            return AutomationDryRunResult {
                rule_id: rule.id,
                rule_name: rule.name.clone(),
                matched: false,
                actions: vec![],
                skip_reason: Some("规则已暂停".into()),
            };
        }
        if !trigger_matches(&rule.definition.trigger, event) {
            return AutomationDryRunResult {
                rule_id: rule.id,
                rule_name: rule.name.clone(),
                matched: false,
                actions: vec![],
                skip_reason: Some("触发器不匹配".into()),
            };
        }
        if !evaluate_conditions(&rule.definition.conditions, event) {
            return AutomationDryRunResult {
                rule_id: rule.id,
                rule_name: rule.name.clone(),
                matched: false,
                actions: vec![],
                skip_reason: Some("条件不满足".into()),
            };
        }
        AutomationDryRunResult {
            rule_id: rule.id,
            rule_name: rule.name.clone(),
            matched: true,
            actions: rule.definition.actions.clone(),
            skip_reason: None,
        }
    }

    pub fn run_for_event(
        &self,
        app: &AppHandle,
        settings: &SettingsService,
        tasks: &TaskService,
        memories: &MemoryService,
        event: AutomationEvent,
        dry_run: bool,
    ) -> Result<Vec<AutomationRunOutcome>, DomainError> {
        if !dry_run {
            let enabled = settings.get()?.automation_enabled;
            if !enabled {
                return Ok(vec![]);
            }
            let depth = AUTOMATION_DEPTH.with(|c| *c.borrow());
            if depth > 0 {
                return Ok(vec![]);
            }
        }

        let rules = self.list()?;
        let mut outcomes = Vec::new();

        let execute = || -> Result<Vec<AutomationRunOutcome>, DomainError> {
            for rule in rules {
                let evaluation = self.evaluate_rule(&rule, &event);
                if !evaluation.matched {
                    if dry_run {
                        outcomes.push(AutomationRunOutcome {
                            rule_id: evaluation.rule_id,
                            rule_name: evaluation.rule_name,
                            status: AutomationRunStatus::Skipped,
                            actions_applied: vec![],
                            error_summary: evaluation.skip_reason,
                        });
                    }
                    continue;
                }

                if dry_run {
                    self.record_run(
                        &rule,
                        &event,
                        AutomationRunStatus::DryRun,
                        &evaluation.actions,
                        None,
                        true,
                    )?;
                    outcomes.push(AutomationRunOutcome {
                        rule_id: rule.id,
                        rule_name: rule.name.clone(),
                        status: AutomationRunStatus::DryRun,
                        actions_applied: evaluation.actions.clone(),
                        error_summary: None,
                    });
                    continue;
                }

                match self.execute_actions(app, tasks, memories, &event, &evaluation.actions) {
                    Ok(applied) => {
                        self.record_run(
                            &rule,
                            &event,
                            AutomationRunStatus::Success,
                            &applied,
                            None,
                            false,
                        )?;
                        outcomes.push(AutomationRunOutcome {
                            rule_id: rule.id,
                            rule_name: rule.name.clone(),
                            status: AutomationRunStatus::Success,
                            actions_applied: applied,
                            error_summary: None,
                        });
                    }
                    Err(err) => {
                        let summary = sanitize_error(&err.to_string());
                        self.record_run(
                            &rule,
                            &event,
                            AutomationRunStatus::Failed,
                            &[],
                            Some(summary.clone()),
                            false,
                        )?;
                        outcomes.push(AutomationRunOutcome {
                            rule_id: rule.id,
                            rule_name: rule.name.clone(),
                            status: AutomationRunStatus::Failed,
                            actions_applied: vec![],
                            error_summary: Some(summary),
                        });
                    }
                }
            }
            Ok(outcomes)
        };

        if dry_run {
            execute()
        } else {
            AUTOMATION_DEPTH.with(|cell| {
                *cell.borrow_mut() += 1;
                let result = execute();
                *cell.borrow_mut() -= 1;
                result
            })
        }
    }

    fn execute_actions(
        &self,
        app: &AppHandle,
        tasks: &TaskService,
        memories: &MemoryService,
        event: &AutomationEvent,
        actions: &[AutomationAction],
    ) -> Result<Vec<AutomationAction>, DomainError> {
        let mut applied = Vec::new();
        for action in actions {
            self.execute_action(app, tasks, memories, event, action)?;
            applied.push(action.clone());
        }
        Ok(applied)
    }

    fn execute_action(
        &self,
        app: &AppHandle,
        tasks: &TaskService,
        memories: &MemoryService,
        event: &AutomationEvent,
        action: &AutomationAction,
    ) -> Result<(), DomainError> {
        match action {
            AutomationAction::SetPriority { priority } => {
                ensure_entity(event, AutomationEntityType::Task)?;
                let task = tasks.get_task(event.entity_id)?;
                let updated = tasks.update_task(build_task_update(&task, |t| {
                    t.priority = *priority;
                }))?;
                emit_entity_updated(app, "task", updated.id, updated.revision);
            }
            AutomationAction::MoveToList { list_id } => {
                ensure_entity(event, AutomationEntityType::Task)?;
                let task = tasks.get_task(event.entity_id)?;
                let updated = tasks.update_task(build_task_update(&task, |t| {
                    t.list_id = *list_id;
                }))?;
                emit_entity_updated(app, "task", updated.id, updated.revision);
            }
            AutomationAction::AddTag { tag_name } => {
                ensure_entity(event, AutomationEntityType::Task)?;
                let task = tasks.get_task(event.entity_id)?;
                let mut tags = task.tag_names.clone();
                if !tags.iter().any(|t| t.eq_ignore_ascii_case(tag_name)) {
                    tags.push(tag_name.trim().to_string());
                }
                let updated = tasks.update_task(build_task_update(&task, |t| {
                    t.tag_names = tags.clone();
                }))?;
                emit_entity_updated(app, "task", updated.id, updated.revision);
            }
            AutomationAction::PinMemory => {
                ensure_entity(event, AutomationEntityType::Memory)?;
                let memory = memories.get(event.entity_id)?;
                let updated = memories.update(build_memory_update(&memory, true))?;
                emit_entity_updated(app, "memory", updated.id, updated.revision);
            }
            AutomationAction::Notify { title, body } => {
                app.notification()
                    .builder()
                    .title(title)
                    .body(body)
                    .show()
                    .map_err(|e| DomainError::Internal(format!("通知失败: {e}")))?;
            }
        }
        Ok(())
    }

    fn record_run(
        &self,
        rule: &AutomationRule,
        event: &AutomationEvent,
        status: AutomationRunStatus,
        actions: &[AutomationAction],
        error_summary: Option<String>,
        dry_run: bool,
    ) -> Result<(), DomainError> {
        let id = new_id();
        let now = stamp(&self.clock);
        let actions_json =
            serde_json::to_string(actions).map_err(|e| internal(e.to_string()))?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO automation_runs (
                id, rule_id, rule_name, entity_type, entity_id, status,
                actions_applied_json, error_summary, dry_run, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id.to_string(),
                rule.id.to_string(),
                rule.name,
                event.entity_type.as_str(),
                event.entity_id.to_string(),
                status.as_str(),
                actions_json,
                error_summary,
                dry_run as i32,
                now,
            ],
        )
        .map_err(internal)?;
        Ok(())
    }
}

fn build_task_update(task: &Task, mutate: impl FnOnce(&mut UpdateTaskInput)) -> UpdateTaskInput {
    let mut input = UpdateTaskInput {
        id: task.id,
        title: task.title.clone(),
        notes: task.notes.clone(),
        priority: task.priority,
        list_id: task.list_id,
        due_date: task.due_date.clone(),
        due_time: task.due_time.clone(),
        tag_names: task.tag_names.clone(),
    };
    mutate(&mut input);
    input
}

fn build_memory_update(memory: &Memory, pinned: bool) -> UpdateMemoryInput {
    UpdateMemoryInput {
        id: memory.id,
        title: memory.title.clone(),
        body: memory.body.clone(),
        pinned,
        archived: memory.archived,
        quick_insert: memory.quick_insert,
        trigger_word: memory.trigger_word.clone(),
        sensitive: memory.sensitive,
        tag_names: memory.tag_names.clone(),
    }
}

fn ensure_entity(event: &AutomationEvent, expected: AutomationEntityType) -> Result<(), DomainError> {
    if event.entity_type != expected {
        return Err(DomainError::Validation(format!(
            "动作不适用于 {} 类型",
            event.entity_type.as_str()
        )));
    }
    Ok(())
}

fn emit_entity_updated(app: &AppHandle, entity_type: &str, entity_id: EntityId, revision: i64) {
    let _ = app.emit(
        "domain://changed",
        serde_json::json!({
            "entityType": entity_type,
            "entityId": entity_id.to_string(),
            "change": "updated",
            "revision": revision,
        }),
    );
}

fn sanitize_error(message: &str) -> String {
    const MAX: usize = 240;
    let trimmed = message.trim();
    if trimmed.chars().count() <= MAX {
        trimmed.to_string()
    } else {
        trimmed.chars().take(MAX).collect::<String>() + "…"
    }
}

fn collect_runs(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> Result<AutomationRun, rusqlite::Error>>,
) -> Result<Vec<AutomationRun>, DomainError> {
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(internal)?);
    }
    Ok(out)
}

fn map_rule(row: &rusqlite::Row<'_>) -> Result<AutomationRule, rusqlite::Error> {
    let definition_raw: String = row.get(3)?;
    let definition: AutomationRuleDefinition =
        serde_json::from_str(&definition_raw).unwrap_or_else(|_| AutomationRuleDefinition {
            trigger: AutomationTrigger::TaskCreated,
            conditions: vec![],
            actions: vec![],
        });
    Ok(AutomationRule {
        id: parse_uuid_row(row.get::<_, String>(0)?)?,
        name: row.get(1)?,
        enabled: row.get::<_, i32>(2)? != 0,
        definition,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        revision: row.get(6)?,
    })
}

fn map_run(row: &rusqlite::Row<'_>) -> Result<AutomationRun, rusqlite::Error> {
    let actions_raw: Option<String> = row.get(6)?;
    let actions: Vec<AutomationAction> = actions_raw
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_default();
    let entity_type = AutomationEntityType::parse(row.get::<_, String>(3)?.as_str())
        .unwrap_or(AutomationEntityType::Task);
    let status = AutomationRunStatus::parse(row.get::<_, String>(5)?.as_str())
        .unwrap_or(AutomationRunStatus::Failed);
    Ok(AutomationRun {
        id: parse_uuid_row(row.get::<_, String>(0)?)?,
        rule_id: parse_uuid_row(row.get::<_, String>(1)?)?,
        rule_name: row.get(2)?,
        entity_type,
        entity_id: parse_uuid_row(row.get::<_, String>(4)?)?,
        status,
        actions_applied: actions,
        error_summary: row.get(7)?,
        dry_run: row.get::<_, i32>(8)? != 0,
        created_at: row.get(9)?,
    })
}

fn parse_uuid_row(value: String) -> Result<EntityId, rusqlite::Error> {
    value.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

pub fn event_from_task_created(task: &Task) -> AutomationEvent {
    AutomationEvent {
        kind: AutomationEventKind::TaskCreated,
        entity_type: AutomationEntityType::Task,
        entity_id: task.id,
        title: task.title.clone(),
        body: task.notes.clone(),
        list_id: Some(task.list_id),
        tag_names: task.tag_names.clone(),
        priority: Some(task.priority),
        source_app: None,
        added_tag: None,
        target_list_id: None,
    }
}

pub fn event_from_task_moved(task: &Task, _previous_list_id: EntityId) -> AutomationEvent {
    AutomationEvent {
        kind: AutomationEventKind::TaskMovedToList,
        entity_type: AutomationEntityType::Task,
        entity_id: task.id,
        title: task.title.clone(),
        body: task.notes.clone(),
        list_id: Some(task.list_id),
        tag_names: task.tag_names.clone(),
        priority: Some(task.priority),
        source_app: None,
        added_tag: None,
        target_list_id: Some(task.list_id),
    }
}

pub fn event_from_task_tag_added(task: &Task, tag: &str) -> AutomationEvent {
    AutomationEvent {
        kind: AutomationEventKind::TaskTagAdded,
        entity_type: AutomationEntityType::Task,
        entity_id: task.id,
        title: task.title.clone(),
        body: task.notes.clone(),
        list_id: Some(task.list_id),
        tag_names: task.tag_names.clone(),
        priority: Some(task.priority),
        source_app: None,
        added_tag: Some(tag.to_string()),
        target_list_id: None,
    }
}

pub fn event_from_reminder_created(reminder: &Reminder) -> AutomationEvent {
    AutomationEvent {
        kind: AutomationEventKind::ReminderCreated,
        entity_type: AutomationEntityType::Reminder,
        entity_id: reminder.id,
        title: reminder.title.clone(),
        body: reminder.notes.clone(),
        list_id: None,
        tag_names: vec![],
        priority: None,
        source_app: None,
        added_tag: None,
        target_list_id: None,
    }
}

pub fn event_from_reminder_fired(occ: &ReminderOccurrence) -> AutomationEvent {
    AutomationEvent {
        kind: AutomationEventKind::ReminderFired,
        entity_type: AutomationEntityType::Reminder,
        entity_id: occ.reminder_id,
        title: occ.title.clone(),
        body: String::new(),
        list_id: None,
        tag_names: vec![],
        priority: None,
        source_app: None,
        added_tag: None,
        target_list_id: None,
    }
}

pub fn event_from_memory_created(memory: &Memory) -> AutomationEvent {
    AutomationEvent {
        kind: AutomationEventKind::MemoryCreated,
        entity_type: AutomationEntityType::Memory,
        entity_id: memory.id,
        title: memory.title.clone(),
        body: memory.body.clone(),
        list_id: None,
        tag_names: memory.tag_names.clone(),
        priority: None,
        source_app: None,
        added_tag: None,
        target_list_id: None,
    }
}

pub fn event_from_clipboard_favorited(item: &ClipboardItem) -> AutomationEvent {
    AutomationEvent {
        kind: AutomationEventKind::ClipboardFavorited,
        entity_type: AutomationEntityType::Clipboard,
        entity_id: item.id,
        title: item.content.chars().take(120).collect(),
        body: item.content.clone(),
        list_id: None,
        tag_names: vec![],
        priority: None,
        source_app: item.source_app.clone(),
        added_tag: None,
        target_list_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TaskPriority;
    use tempfile::tempdir;

    fn sample_definition() -> AutomationRuleDefinition {
        AutomationRuleDefinition {
            trigger: AutomationTrigger::TaskCreated,
            conditions: vec![AutomationCondition::TitleContains {
                text: "urgent".into(),
                case_insensitive: true,
            }],
            actions: vec![AutomationAction::SetPriority {
                priority: TaskPriority::High,
            }],
        }
    }

    use crate::domain::AutomationCondition;

    #[test]
    fn create_list_delete_rule() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("t.db")).unwrap();
        let svc = AutomationService::new(db);

        let created = svc
            .create(CreateAutomationRuleInput {
                name: "高优先级".into(),
                definition: sample_definition(),
            })
            .unwrap();
        assert!(created.enabled);
        assert_eq!(created.name, "高优先级");

        let listed = svc.list().unwrap();
        assert_eq!(listed.len(), 1);

        svc.set_enabled(created.id, false).unwrap();
        let updated = svc.get(created.id).unwrap();
        assert!(!updated.enabled);

        svc.delete(created.id).unwrap();
        assert!(svc.list().unwrap().is_empty());
    }

    #[test]
    fn dry_run_does_not_require_global_enabled() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("t.db")).unwrap();
        let svc = AutomationService::new(db);
        let rule = svc
            .create(CreateAutomationRuleInput {
                name: "test".into(),
                definition: sample_definition(),
            })
            .unwrap();
        let event = AutomationEvent {
            kind: AutomationEventKind::TaskCreated,
            entity_type: AutomationEntityType::Task,
            entity_id: new_id(),
            title: "URGENT fix".into(),
            body: String::new(),
            list_id: None,
            tag_names: vec![],
            priority: None,
            source_app: None,
            added_tag: None,
            target_list_id: None,
        };
        let result = svc.dry_run(rule.id, event).unwrap();
        assert!(result.matched);
        assert_eq!(result.actions.len(), 1);
    }
}
