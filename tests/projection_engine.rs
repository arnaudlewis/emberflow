use emberflow::runtime::projections::ProjectionEngine;
use emberflow::runtime::store::{EventRecord, TaskRecord, TrackRecord};

// @minter:unit progress-projects-to-plain-text-chat
#[test]
fn progress_projects_to_plain_text_chat() {
    let engine = ProjectionEngine;
    let event = EventRecord {
        id: "event-001".to_string(),
        track_id: Some("track-001".to_string()),
        task_id: Some("task-001".to_string()),
        kind: "progress".to_string(),
        payload: serde_json::json!({
            "track_id": "track-001",
            "task_id": "task-001",
            "agent": "dev-implementer",
            "status": "running",
            "phase": "implementing",
            "summary": "Writing the first event",
            "completed": ["Read the spec"],
            "current_action": "Writing the first event",
            "next_update_by": "2026-04-02T10:00:00Z",
            "confidence": 0.81
        }),
        created_at: "now".to_string(),
    };

    let projection = engine.project_user_view(&event);

    assert_eq!(projection.event_id, "event-001");
    assert_eq!(projection.kind, "progress");
    assert_eq!(projection.format, "plain-text");
    assert!(!projection.summary.is_empty());
}

// @minter:unit progress-projects-to-runtime-status-line
#[test]
fn progress_projects_to_runtime_status_line() {
    let engine = ProjectionEngine;
    let event = EventRecord {
        id: "event-001".to_string(),
        track_id: Some("track-001".to_string()),
        task_id: Some("task-001".to_string()),
        kind: "progress".to_string(),
        payload: serde_json::json!({
            "track_id": "track-001",
            "task_id": "task-001",
            "agent": "dev-implementer",
            "status": "running",
            "phase": "implementing",
            "summary": "Writing the first event",
            "completed": ["Read the spec"],
            "current_action": "Writing the first event",
            "next_update_by": "2026-04-02T10:00:00Z",
            "confidence": 0.81
        }),
        created_at: "now".to_string(),
    };

    let projection = engine.project_runtime_view(&event, None);

    assert_eq!(projection.event_id, "event-001");
    assert_eq!(projection.target_path, ".emberflow/context/status.md");
    assert_eq!(
        projection.line_format,
        "phase: ... | status: ... | details: ..."
    );
}

// @minter:unit progress-runtime-inherits-canonical-task-state
#[test]
fn progress_runtime_inherits_canonical_task_state() {
    let engine = ProjectionEngine;
    let task = TaskRecord {
        id: "task-plan-review".to_string(),
        track_id: Some("track-001".to_string()),
        plan_item_id: None,
        title: "Review and approve the execution plan".to_string(),
        status: "need-input".to_string(),
        phase: "planning".to_string(),
        executor: Some("assistant".to_string()),
        agent_instance_id: None,
        execution: Some("interactive session".to_string()),
        intent_summary: Some("Review the plan before implementation".to_string()),
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
        lease_holder: None,
        lease_expires_at: None,
    };
    let event = EventRecord {
        id: "event-001".to_string(),
        track_id: Some("track-001".to_string()),
        task_id: Some(task.id.clone()),
        kind: "progress".to_string(),
        payload: serde_json::json!({
            "summary": "Waiting for plan approval"
        }),
        created_at: "now".to_string(),
    };

    let projection = engine.project_runtime_view(&event, Some(&task));

    assert_eq!(projection.status, "need-input");
    assert_eq!(projection.phase, "planning");
    assert_eq!(
        projection.line,
        "phase: planning | status: need-input | details: Waiting for plan approval"
    );
}

// @minter:unit progress-runtime-respects-explicit-overrides
#[test]
fn progress_runtime_respects_explicit_overrides() {
    let engine = ProjectionEngine;
    let task = TaskRecord {
        id: "task-plan-review".to_string(),
        track_id: Some("track-001".to_string()),
        plan_item_id: None,
        title: "Review and approve the execution plan".to_string(),
        status: "need-input".to_string(),
        phase: "planning".to_string(),
        executor: Some("assistant".to_string()),
        agent_instance_id: None,
        execution: Some("interactive session".to_string()),
        intent_summary: Some("Review the plan before implementation".to_string()),
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
        lease_holder: None,
        lease_expires_at: None,
    };
    let event = EventRecord {
        id: "event-002".to_string(),
        track_id: Some("track-001".to_string()),
        task_id: Some(task.id.clone()),
        kind: "progress".to_string(),
        payload: serde_json::json!({
            "status": "running",
            "phase": "implementing",
            "summary": "Implementing the approved plan"
        }),
        created_at: "now".to_string(),
    };

    let projection = engine.project_runtime_view(&event, Some(&task));

    assert_eq!(projection.status, "running");
    assert_eq!(projection.phase, "implementing");
}

// @minter:unit transient-progress-does-not-mutate-track
#[test]
fn transient_progress_does_not_mutate_track() {
    let engine = ProjectionEngine;
    let track = TrackRecord {
        id: "track-001".to_string(),
        title: "Build EmberFlow V1".to_string(),
        status: "in-progress".to_string(),
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    let event = EventRecord {
        id: "event-001".to_string(),
        track_id: Some("track-001".to_string()),
        task_id: Some("task-001".to_string()),
        kind: "progress".to_string(),
        payload: serde_json::json!({"summary": "Writing the first event"}),
        created_at: "now".to_string(),
    };

    let projection = engine.project_track_view(&event, &track);

    assert_eq!(projection.track_id, "track-001");
    assert_eq!(projection.event_id, "event-001");
    assert_eq!(projection.durable_change, "none");
    assert_eq!(track.status, "in-progress");
}

// @minter:unit blocker-projects-durable-track-state
#[test]
fn blocker_projects_durable_track_state() {
    let engine = ProjectionEngine;
    let track = TrackRecord {
        id: "track-001".to_string(),
        title: "Build EmberFlow V1".to_string(),
        status: "in-progress".to_string(),
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    let event = EventRecord {
        id: "event-002".to_string(),
        track_id: Some("track-001".to_string()),
        task_id: Some("task-001".to_string()),
        kind: "blocker".to_string(),
        payload: serde_json::json!({
            "summary": "Need a product decision",
            "what_i_tried": ["Read the specs"],
            "needs_from_decider": "confirm the projection shape",
            "recommendation": "Keep the current model"
        }),
        created_at: "now".to_string(),
    };

    let projection = engine.project_track_view(&event, &track);

    assert_eq!(projection.track_id, "track-001");
    assert_eq!(projection.status.as_deref(), Some("blocked"));
    assert!(!projection.summary.is_empty());
}

// @minter:unit handoff-projects-track-to-review
#[test]
fn handoff_projects_track_to_review() {
    let engine = ProjectionEngine;
    let track = TrackRecord {
        id: "track-001".to_string(),
        title: "Build EmberFlow V1".to_string(),
        status: "in-progress".to_string(),
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    let event = EventRecord {
        id: "event-003".to_string(),
        track_id: Some("track-001".to_string()),
        task_id: Some("task-001".to_string()),
        kind: "handoff".to_string(),
        payload: serde_json::json!({
            "summary": "Ready for validation",
            "files_changed": ["emberflow/runtime/store.rs"],
            "checks_performed": ["unit tests"],
            "known_limits": ["No transport wiring yet"],
            "recommended_next_step": "Review the V1 surface"
        }),
        created_at: "now".to_string(),
    };

    let projection = engine.project_track_view(&event, &track);

    assert_eq!(projection.track_id, "track-001");
    assert_eq!(projection.status.as_deref(), Some("review"));
    assert!(!projection.summary.is_empty());
}
