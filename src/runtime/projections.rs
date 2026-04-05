use super::layout::PROJECTED_RUNTIME_STATUS_PATH;
use super::store::{EventRecord, TaskRecord, TrackRecord};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProjection {
    pub event_id: String,
    pub kind: String,
    pub format: String,
    pub summary: String,
    pub line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProjection {
    pub event_id: String,
    pub kind: String,
    pub target_path: String,
    pub line_format: String,
    pub line: String,
    pub status: String,
    pub phase: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackProjection {
    pub event_id: String,
    pub track_id: String,
    pub kind: String,
    pub summary: String,
    pub durable_change: String,
    pub status: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ProjectionEngine;

impl ProjectionEngine {
    pub fn project_user_view(&self, event: &EventRecord) -> UserProjection {
        let payload = event.payload.clone();
        let summary = self.summary_for_user(&event.kind, &payload);
        UserProjection {
            event_id: event.id.clone(),
            kind: event.kind.clone(),
            format: "plain-text".to_string(),
            summary: summary.clone(),
            line: summary,
        }
    }

    pub fn project_runtime_view(
        &self,
        event: &EventRecord,
        task: Option<&TaskRecord>,
    ) -> RuntimeProjection {
        let payload = event.payload.clone();
        let phase = payload
            .get("phase")
            .and_then(Value::as_str)
            .or_else(|| {
                if event.kind == "progress" {
                    task.map(|task| task.phase.as_str())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| self.default_phase(&event.kind));
        let status = payload
            .get("status")
            .and_then(Value::as_str)
            .or_else(|| {
                if event.kind == "progress" {
                    task.map(|task| task.status.as_str())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| self.default_status(&event.kind));
        let details = self.details(&payload);
        let line = format!("phase: {phase} | status: {status} | details: {details}");
        RuntimeProjection {
            event_id: event.id.clone(),
            kind: event.kind.clone(),
            target_path: PROJECTED_RUNTIME_STATUS_PATH.to_string(),
            line_format: "phase: ... | status: ... | details: ...".to_string(),
            line,
            status: status.to_string(),
            phase: phase.to_string(),
            details,
        }
    }

    pub fn project_track_view(&self, event: &EventRecord, track: &TrackRecord) -> TrackProjection {
        let summary = self.details(&event.payload);
        let (status, durable_change) = self.track_projection(&event.kind, &track.status);
        TrackProjection {
            event_id: event.id.clone(),
            track_id: track.id.clone(),
            kind: event.kind.clone(),
            summary,
            durable_change,
            status,
        }
    }

    fn details(&self, payload: &Value) -> String {
        [
            "summary",
            "current_action",
            "understanding",
            "goal",
            "recommendation",
            "details",
        ]
        .iter()
        .find_map(|key| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "update".to_string())
    }

    fn default_status(&self, kind: &str) -> &'static str {
        match kind {
            "assign" => "queued",
            "ack" => "running",
            "progress" => "running",
            "blocker" => "blocked",
            "handoff" => "awaiting-review",
            "close" => "done",
            _ => "running",
        }
    }

    fn default_phase(&self, kind: &str) -> &'static str {
        match kind {
            "assign" => "planning",
            "ack" => "planning",
            "progress" => "implementing",
            "blocker" => "implementing",
            "handoff" => "verifying",
            "close" => "verifying",
            _ => "planning",
        }
    }

    fn summary_for_user(&self, kind: &str, payload: &Value) -> String {
        let summary = self.details(payload);
        let agent = payload.get("agent").and_then(Value::as_str);
        let phase = payload
            .get("phase")
            .and_then(Value::as_str)
            .unwrap_or_else(|| self.default_phase(kind));
        let status = payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_else(|| self.default_status(kind));
        let prefix = match kind {
            "assign" => "Delegation",
            "ack" => "Ack",
            "progress" => "Progress",
            "blocker" => "Blocker",
            "handoff" => "Handoff",
            "close" => "Close",
            _ => "Update",
        };

        let mut parts = vec![prefix.to_string()];
        if let Some(agent) = agent {
            parts.push(agent.to_string());
        }
        parts.push(format!("{status} / {phase}"));
        parts.push(summary);
        parts.join(" — ")
    }

    fn track_projection(&self, kind: &str, current_status: &str) -> (Option<String>, String) {
        match kind {
            "blocker" => (Some("blocked".to_string()), "status:blocked".to_string()),
            "handoff" => (Some("review".to_string()), "status:review".to_string()),
            "close" => (Some("done".to_string()), "status:done".to_string()),
            _ => (Some(current_status.to_string()), "none".to_string()),
        }
    }
}
