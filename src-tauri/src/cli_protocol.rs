use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::application::workbench_actions;
use crate::domain::{ActionDispatchOptions, ActionOutcome, ActionSource, WorkbenchAction};

pub const TROVE_ACTION_PREFIX: &str = "trove-action:";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliDispatchRequest {
    pub action: WorkbenchAction,
    pub options: ActionDispatchOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_path: Option<String>,
}

impl CliDispatchRequest {
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(format!("{TROVE_ACTION_PREFIX}{json}"))
    }

    pub fn decode(raw: &str) -> Result<Self, String> {
        let json = raw
            .strip_prefix(TROVE_ACTION_PREFIX)
            .ok_or_else(|| "缺少 trove-action: 前缀".to_string())?;
        serde_json::from_str(json).map_err(|e| format!("JSON 无效: {e}"))
    }

    pub fn cli_options(dry_run: bool) -> ActionDispatchOptions {
        ActionDispatchOptions {
            source: ActionSource::Cli,
            dry_run,
        }
    }
}

pub fn dry_run_outcome(action: &WorkbenchAction) -> ActionOutcome {
    ActionOutcome::DryRun {
        description: crate::domain::workbench_action_description(action),
    }
}

pub fn is_cli_action_arg(arg: &str) -> bool {
    arg.starts_with(TROVE_ACTION_PREFIX)
}

pub fn handle_cli_dispatch(app: &AppHandle, raw: &str) {
    let request = match CliDispatchRequest::decode(raw) {
        Ok(req) => req,
        Err(err) => {
            tracing::warn!(error = %err, "rejected cli dispatch");
            return;
        }
    };

    let state = app.try_state::<AppState>().map(|s| s.inner());
    let outcome = match workbench_actions::dispatch(
        app,
        state,
        request.action,
        request.options,
    ) {
        Ok(outcome) => outcome,
        Err(err) => ActionOutcome::Rejected {
            reason: err.to_string(),
        },
    };

    if let Some(path) = request.response_path {
        match serde_json::to_string(&outcome) {
            Ok(json) => {
                if let Err(err) = std::fs::write(&path, json) {
                    tracing::warn!(path = %path, error = %err, "cli response write failed");
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, "cli response serialize failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CreateTaskInput;

    #[test]
    fn roundtrip_encode_decode() {
        let req = CliDispatchRequest {
            action: WorkbenchAction::Navigate {
                path: "/today".into(),
            },
            options: CliDispatchRequest::cli_options(false),
            response_path: Some("/tmp/out.json".into()),
        };
        let encoded = req.encode().expect("encode");
        let decoded = CliDispatchRequest::decode(&encoded).expect("decode");
        assert!(matches!(
            decoded.action,
            WorkbenchAction::Navigate { path } if path == "/today"
        ));
    }

    #[test]
    fn dry_run_describes_task_create() {
        let action = WorkbenchAction::CreateTask {
            input: CreateTaskInput {
                title: "CLI task".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            },
            confirmed: true,
        };
        let outcome = dry_run_outcome(&action);
        assert!(matches!(outcome, ActionOutcome::DryRun { .. }));
    }
}
