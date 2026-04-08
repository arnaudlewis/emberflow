use crate::error::{EmberFlowError, Result};

pub const TRACK_STATUSES: &[&str] = &[
    "planning",
    "in-progress",
    "blocked",
    "review",
    "done",
    "archived",
];

pub const TASK_STATUSES: &[&str] = &[
    "queued",
    "running",
    "need-input",
    "blocked",
    "awaiting-review",
    "done",
    "failed",
    "cancelled",
];

pub const PHASES: &[&str] = &[
    "exploring",
    "planning",
    "implementing",
    "reviewing",
    "verifying",
];

pub const RUNTIME_MESSAGES: &[&str] = &[
    "assign",
    "ack",
    "progress",
    "blocker",
    "handoff",
    "close",
    "claim",
    "release",
    "lease-expired",
];

pub const EMBERFLOW_TRACK_CREATE_TOOL: &str = "emberflow-track-create";
pub const EMBERFLOW_TRACK_METADATA_UPSERT_TOOL: &str = "emberflow-track-metadata-upsert";
pub const EMBERFLOW_TRACK_BRIEF_REPLACE_TOOL: &str = "emberflow-track-brief-replace";
pub const EMBERFLOW_TRACK_PLAN_REPLACE_TOOL: &str = "emberflow-track-plan-replace";
pub const EMBERFLOW_TRACK_ARCHIVE_TOOL: &str = "emberflow-track-archive";
pub const EMBERFLOW_TRACK_DELETE_TOOL: &str = "emberflow-track-delete";
pub const EMBERFLOW_TASK_CREATE_TOOL: &str = "emberflow-task-create";
pub const EMBERFLOW_EVENT_RECORD_TOOL: &str = "emberflow-event-record";
pub const EMBERFLOW_TASK_CLAIM_TOOL: &str = "emberflow-task-claim";
pub const EMBERFLOW_TASK_RELEASE_TOOL: &str = "emberflow-task-release";

pub const EMBERFLOW_STANDARD_TOOLS: &[&str] = &[
    EMBERFLOW_TRACK_CREATE_TOOL,
    EMBERFLOW_TRACK_METADATA_UPSERT_TOOL,
    EMBERFLOW_TRACK_BRIEF_REPLACE_TOOL,
    EMBERFLOW_TRACK_PLAN_REPLACE_TOOL,
    EMBERFLOW_TRACK_ARCHIVE_TOOL,
    EMBERFLOW_TRACK_DELETE_TOOL,
    EMBERFLOW_TASK_CREATE_TOOL,
    EMBERFLOW_EVENT_RECORD_TOOL,
    EMBERFLOW_TASK_CLAIM_TOOL,
    EMBERFLOW_TASK_RELEASE_TOOL,
];

pub fn validate_choice(value: &str, allowed: &[&str], field: &'static str) -> Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(EmberFlowError::UnsupportedValue {
            field,
            value: value.to_string(),
        })
    }
}
