//! Unified rules for whether a task appears in active list views.

use chrono::NaiveDate;

use super::{TaskStatus, TaskWorkflowState};

/// Fields needed for activity predicates (subset of Task).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskActivityView {
    pub status: TaskStatus,
    pub workflow_state: TaskWorkflowState,
    pub available_at: Option<NaiveDate>,
    pub follow_up_date: Option<NaiveDate>,
}

impl TaskActivityView {
    pub fn from_parts(
        status: TaskStatus,
        workflow_state: TaskWorkflowState,
        available_at: Option<NaiveDate>,
        follow_up_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            status,
            workflow_state,
            available_at,
            follow_up_date,
        }
    }
}

/// Todo tasks that belong in inbox / task lists / smart lists (not deferred, not waiting).
pub fn is_active_list_task(view: &TaskActivityView, local_date: NaiveDate) -> bool {
    if view.status != TaskStatus::Todo {
        return false;
    }
    if view.workflow_state == TaskWorkflowState::Waiting {
        return false;
    }
    is_available_by_date(view.available_at, local_date)
}

/// Deferred: todo with available_at strictly in the future.
pub fn is_deferred_task(view: &TaskActivityView, local_date: NaiveDate) -> bool {
    view.status == TaskStatus::Todo
        && view.workflow_state == TaskWorkflowState::Active
        && view
            .available_at
            .is_some_and(|d| d > local_date)
}

/// Waiting tasks whose follow-up date is due (for Today "等待跟进" section).
pub fn is_waiting_follow_up_due(view: &TaskActivityView, local_date: NaiveDate) -> bool {
    view.status == TaskStatus::Todo
        && view.workflow_state == TaskWorkflowState::Waiting
        && view
            .follow_up_date
            .is_some_and(|d| d <= local_date)
        && is_available_by_date(view.available_at, local_date)
}

fn is_available_by_date(available_at: Option<NaiveDate>, local_date: NaiveDate) -> bool {
    available_at.is_none_or(|d| d <= local_date)
}

/// Search queries must not hide deferred/waiting tasks.
pub fn should_apply_active_list_filter(search: Option<&str>) -> bool {
    search.map(|s| s.trim().is_empty()).unwrap_or(true)
}

pub fn parse_workflow_state(value: &str) -> Result<TaskWorkflowState, String> {
    TaskWorkflowState::parse(value).map_err(|e| e.to_string())
}

pub fn parse_optional_date(value: Option<&str>) -> Result<Option<NaiveDate>, String> {
    match value {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| format!("date must be YYYY-MM-DD: {s}")),
    }
}

/// Block save when due date is before defer display date.
pub fn validate_due_vs_available(
    due_date: Option<&str>,
    available_at: Option<&str>,
) -> Result<(), String> {
    let due = match due_date.filter(|s| !s.is_empty()) {
        Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .map_err(|_| "dueDate must be YYYY-MM-DD".to_string())?,
        None => return Ok(()),
    };
    let avail = match available_at.filter(|s| !s.is_empty()) {
        Some(a) => NaiveDate::parse_from_str(a, "%Y-%m-%d")
            .map_err(|_| "availableAt must be YYYY-MM-DD".to_string())?,
        None => return Ok(()),
    };
    if due < avail {
        return Err(
            "截止日期早于推迟显示日，任务会在此之前到期。请调整截止日期或推迟日。".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn active() -> TaskActivityView {
        TaskActivityView::from_parts(
            TaskStatus::Todo,
            TaskWorkflowState::Active,
            None,
            None,
        )
    }

    #[test]
    fn active_list_excludes_deferred() {
        let view = TaskActivityView::from_parts(
            TaskStatus::Todo,
            TaskWorkflowState::Active,
            Some(d("2026-08-20")),
            None,
        );
        assert!(!is_active_list_task(&view, d("2026-08-15")));
        assert!(is_deferred_task(&view, d("2026-08-15")));
        assert!(is_active_list_task(&view, d("2026-08-20")));
    }

    #[test]
    fn active_list_excludes_waiting() {
        let view = TaskActivityView::from_parts(
            TaskStatus::Todo,
            TaskWorkflowState::Waiting,
            None,
            Some(d("2026-08-16")),
        );
        assert!(!is_active_list_task(&view, d("2026-08-16")));
        assert!(is_waiting_follow_up_due(&view, d("2026-08-16")));
    }

    #[test]
    fn waiting_without_follow_up_not_in_follow_up_section() {
        let view = TaskActivityView::from_parts(
            TaskStatus::Todo,
            TaskWorkflowState::Waiting,
            None,
            None,
        );
        assert!(!is_waiting_follow_up_due(&view, d("2026-08-16")));
    }

    #[test]
    fn search_skips_active_filter() {
        assert!(!should_apply_active_list_filter(Some("hello")));
        assert!(should_apply_active_list_filter(Some("  ")));
        assert!(should_apply_active_list_filter(None));
    }

    #[test]
    fn due_before_available_rejected() {
        assert!(validate_due_vs_available(Some("2026-08-10"), Some("2026-08-20")).is_err());
        assert!(validate_due_vs_available(Some("2026-08-20"), Some("2026-08-10")).is_ok());
    }
}
