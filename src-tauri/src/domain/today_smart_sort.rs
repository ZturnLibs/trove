use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{EntityId, Task, TaskPriority};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodaySortSuggestion {
    pub task_id: EntityId,
    pub rank: i32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodaySortSuggestions {
    pub enabled: bool,
    pub suggestions: Vec<TodaySortSuggestion>,
}

#[derive(Debug, Clone)]
struct SortCandidate<'a> {
    task: &'a Task,
    defer_count: i64,
    reminder_at: Option<String>,
}

fn priority_score(priority: TaskPriority) -> i32 {
    match priority {
        TaskPriority::High => 3,
        TaskPriority::Medium => 2,
        TaskPriority::Low => 1,
        TaskPriority::None => 0,
    }
}

fn due_time_sort_key(due_time: Option<&str>) -> (i32, String) {
    match due_time {
        Some(t) if t.len() >= 5 => (0, t.to_string()),
        _ => (1, String::new()),
    }
}

fn build_reason(task: &Task, defer_count: i64, reminder_at: Option<&str>) -> String {
    if defer_count >= 2 {
        return format!("已延期 {defer_count} 次");
    }
    if task.priority == TaskPriority::High {
        return "高优先级".into();
    }
    if let Some(at) = reminder_at {
        if at.len() >= 16 {
            return format!("今天 {} 提醒", &at[11..16]);
        }
    }
    if let Some(t) = task.due_time.as_deref() {
        if !t.is_empty() {
            return format!("今天 {t} 到期");
        }
    }
    if defer_count == 1 {
        return "已延期 1 次".into();
    }
    if task.priority == TaskPriority::Medium {
        return "中优先级".into();
    }
    "今天到期".into()
}

fn sort_key(candidate: &SortCandidate<'_>) -> (i32, String, i32, i64, String, i64, String, String) {
    let (due_bucket, due_time) = due_time_sort_key(candidate.task.due_time.as_deref());
    let priority = priority_score(candidate.task.priority);
    let reminder = candidate
        .reminder_at
        .clone()
        .unwrap_or_else(|| "9999".into());
    (
        due_bucket,
        due_time,
        -priority,
        -candidate.defer_count,
        reminder,
        candidate.task.sort_order.to_bits() as i64,
        candidate.task.created_at.clone(),
        candidate.task.id.to_string(),
    )
}

/// Deterministic sort for today's due bucket; higher urgency sorts first (lower rank).
pub fn compute_today_sort_suggestions(
    tasks: &[Task],
    defer_counts: &HashMap<EntityId, i64>,
    reminder_times: &HashMap<EntityId, String>,
) -> Vec<TodaySortSuggestion> {
    if tasks.len() <= 1 {
        return tasks
            .iter()
            .enumerate()
            .map(|(index, task)| TodaySortSuggestion {
                task_id: task.id,
                rank: index as i32,
                reason: build_reason(
                    task,
                    *defer_counts.get(&task.id).unwrap_or(&0),
                    reminder_times.get(&task.id).map(String::as_str),
                ),
            })
            .collect();
    }

    let mut candidates: Vec<SortCandidate<'_>> = tasks
        .iter()
        .map(|task| SortCandidate {
            task,
            defer_count: *defer_counts.get(&task.id).unwrap_or(&0),
            reminder_at: reminder_times.get(&task.id).cloned(),
        })
        .collect();

    candidates.sort_by(|a, b| {
        let ka = sort_key(a);
        let kb = sort_key(b);
        ka.cmp(&kb)
    });

    candidates
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| TodaySortSuggestion {
            task_id: candidate.task.id,
            rank: index as i32,
            reason: build_reason(
                candidate.task,
                candidate.defer_count,
                candidate.reminder_at.as_deref(),
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ListKind, TaskStatus, TaskWorkflowState, new_entity_id, new_id, stamp, SystemClock,
    };

    fn sample_task(id: EntityId, priority: TaskPriority, due_time: Option<&str>) -> Task {
        Task {
            id,
            title: "t".into(),
            notes: String::new(),
            status: TaskStatus::Todo,
            priority,
            list_id: new_entity_id(),
            list_name: "inbox".into(),
            list_kind: ListKind::Inbox,
            due_date: Some("2026-08-16".into()),
            due_time: due_time.map(str::to_string),
            completed_at: None,
            sort_order: 0.0,
            series_id: None,
            tag_ids: vec![],
            tag_names: vec![],
            workflow_state: TaskWorkflowState::Active,
            available_at: None,
            waiting_for: None,
            follow_up_date: None,
            created_at: stamp(&SystemClock),
            updated_at: stamp(&SystemClock),
            revision: 1,
        }
    }

    #[test]
    fn high_priority_and_due_time_ordering() {
        let t1 = sample_task(new_id(), TaskPriority::None, Some("18:00"));
        let t2 = sample_task(new_id(), TaskPriority::High, None);
        let t3 = sample_task(new_id(), TaskPriority::None, Some("09:00"));
        let t1_id = t1.id;
        let t2_id = t2.id;
        let t3_id = t3.id;
        let counts = HashMap::new();
        let reminders = HashMap::new();
        let out = compute_today_sort_suggestions(&[t1, t2, t3], &counts, &reminders);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].task_id, t3_id);
        assert!(out[0].reason.contains("09:00"));
        assert_eq!(out[1].task_id, t1_id);
        assert_eq!(out[2].task_id, t2_id);
        assert_eq!(out[2].reason, "高优先级");
    }

    #[test]
    fn defer_count_reason() {
        let t1 = sample_task(new_id(), TaskPriority::None, None);
        let mut counts = HashMap::new();
        counts.insert(t1.id, 2);
        let out = compute_today_sort_suggestions(&[t1], &counts, &HashMap::new());
        assert_eq!(out[0].reason, "已延期 2 次");
    }

    #[test]
    fn deterministic_tie_breaker() {
        let t1 = sample_task(new_id(), TaskPriority::None, None);
        let mut t2 = sample_task(new_id(), TaskPriority::None, None);
        t2.sort_order = 1.0;
        let t1_id = t1.id;
        let t2_id = t2.id;
        let out = compute_today_sort_suggestions(&[t2, t1], &HashMap::new(), &HashMap::new());
        assert_eq!(out[0].task_id, t1_id);
        assert_eq!(out[1].task_id, t2_id);
    }
}
