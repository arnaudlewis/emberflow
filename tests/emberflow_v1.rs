use emberflow::runtime::service::{
    EmberFlowRuntime, TaskInput, TrackBriefSectionInput, TrackMetadataInput, TrackPlanItemInput,
    TrackPlanPhaseInput,
};
use tempfile::tempdir;

fn canonical_track_metadata_input(track_id: &str) -> TrackMetadataInput {
    TrackMetadataInput {
        track_id: track_id.to_string(),
        track_type: "feature".to_string(),
        status: "in-progress".to_string(),
        description: "Build EmberFlow runtime with track projection engine".to_string(),
        branch: "feature/runtime-v1".to_string(),
        spec_ref: Some("emberflow/specs/emberflow-v1.spec".to_string()),
    }
}

fn brief_sections() -> Vec<TrackBriefSectionInput> {
    vec![
        TrackBriefSectionInput {
            section_key: "objective".to_string(),
            section_text: "Finish the canonical SQLite track model".to_string(),
            position: 0,
        },
        TrackBriefSectionInput {
            section_key: "context".to_string(),
            section_text: "The track resume context must survive workspace restarts".to_string(),
            position: 1,
        },
        TrackBriefSectionInput {
            section_key: "decisions".to_string(),
            section_text: "SQLite is canonical; projections are derived".to_string(),
            position: 2,
        },
        TrackBriefSectionInput {
            section_key: "next_step".to_string(),
            section_text: "Store the canonical plan in SQLite".to_string(),
            position: 3,
        },
    ]
}

fn canonical_plan() -> Vec<TrackPlanPhaseInput> {
    vec![
        TrackPlanPhaseInput {
            phase_id: "phase-1".to_string(),
            title: "Canonical schema design".to_string(),
            position: 0,
            items: vec![
                TrackPlanItemInput {
                    item_id: "phase-1/item-1".to_string(),
                    title: "Define canonical metadata tables".to_string(),
                    position: Some(0),
                },
                TrackPlanItemInput {
                    item_id: "phase-1/item-2".to_string(),
                    title: "Define ordered brief sections".to_string(),
                    position: Some(1),
                },
            ],
        },
        TrackPlanPhaseInput {
            phase_id: "phase-2".to_string(),
            title: "Canonical read/write path".to_string(),
            position: 1,
            items: vec![TrackPlanItemInput {
                item_id: "phase-2/item-1".to_string(),
                title: "Persist and read one canonical track".to_string(),
                position: Some(0),
            }],
        },
    ]
}

// @minter:integration mcp-initializes-with-workspace-db-metadata
#[test]
fn emberflow_v1_initializes_workspace_db_metadata() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();

    let init = runtime.initialize().unwrap();

    assert_eq!(
        init.workspace_db.project_root,
        tmp.path().display().to_string()
    );
    assert_eq!(
        init.workspace_db.state_root,
        tmp.path().join(".emberflow").display().to_string()
    );
    assert_eq!(
        init.workspace_db.default_path,
        tmp.path()
            .join(".emberflow/emberflow.db")
            .display()
            .to_string()
    );
    assert_eq!(init.workspace_db.projection_mode, "canonical");
    assert_eq!(init.workspace_db.mode, "sqlite");
    assert_eq!(init.workspace_db.initialization, "ready");
}

// @minter:integration emberflow-v1-persists-canonical-runtime-state emberflow-v1-surfaces-canonical-task-visibility
#[test]
fn emberflow_v1_persists_canonical_runtime_state() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();

    // Create track+task and claim before recording
    runtime
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    runtime
        .create_task(TaskInput {
            task_id: "task-001".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Persist the first event".to_string(),
            status: "running".to_string(),
            phase: "implementing".to_string(),
            executor: Some("assistant".to_string()),
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Writing the first event".to_string()),
        })
        .unwrap();
    runtime
        .claim_task("task-001", "assistant", Some(300))
        .unwrap();

    let result = runtime
        .record_runtime_state(
            "track-001",
            "task-001",
            "progress",
            serde_json::json!({
                "summary": "Writing the first event",
                "executor": "assistant",
                "execution": "interactive session",
                "intent_summary": "Writing the first event"
            }),
        )
        .unwrap();

    assert_eq!(result.track_id, "track-001");
    assert_eq!(result.task_id, "task-001");
    assert_eq!(result.event_kind, "progress");
    assert_eq!(result.store, "canonical");

    let runtime_status = runtime.get_runtime_status("track-001").unwrap();
    assert_eq!(runtime_status.executor.as_deref(), Some("assistant"));
    assert_eq!(
        runtime_status.execution.as_deref(),
        Some("interactive session")
    );
    assert_eq!(
        runtime_status.intent_summary.as_deref(),
        Some("Writing the first event")
    );
}

// @minter:integration emberflow-v1-persists-canonical-track-context
#[test]
fn emberflow_v1_persists_canonical_track_context() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    runtime
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();
    runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    let context = runtime.load_track_context(track_id).unwrap();

    assert_eq!(context.track_id, track_id);
    assert_eq!(context.metadata.track_id, track_id);
    assert_eq!(context.brief.sections.len(), 4);
    assert_eq!(context.plan.phases.len(), 2);
}

// @minter:integration emberflow-v1-treats-context-status-as-projection
#[test]
fn emberflow_v1_treats_context_status_as_projection() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();
    runtime
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    runtime
        .create_task(TaskInput {
            task_id: "task-001".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Persist the first event".to_string(),
            status: "running".to_string(),
            phase: "implementing".to_string(),
            executor: Some("dev-implementer".to_string()),
            agent_instance_id: None,
            execution: None,
            intent_summary: None,
        })
        .unwrap();
    runtime
        .claim_task("task-001", "dev-implementer", Some(300))
        .unwrap();
    runtime
        .record_runtime_state(
            "track-001",
            "task-001",
            "progress",
            serde_json::json!({
                "agent": "dev-implementer",
                "executor": "dev-implementer",
                "status": "running",
                "phase": "implementing",
                "summary": "Writing the first event",
                "completed": ["Read the spec"],
                "current_action": "Writing the first event",
                "next_update_by": "2026-04-02T10:00:00Z",
                "confidence": 0.81
            }),
        )
        .unwrap();

    let projection = runtime.project_runtime_status("track-001").unwrap();

    assert_eq!(projection.track_id, "track-001");
    assert_eq!(projection.target_path, ".emberflow/context/status.md");
    assert_eq!(projection.source, "canonical-event-store");
}

// @minter:integration emberflow-v1-exposes-queryable-runtime-tools
#[test]
fn emberflow_v1_exposes_queryable_runtime_tools() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();

    let tool_names = runtime.available_tools();

    assert!(tool_names.contains(&"emberflow-track-create"));
    assert!(tool_names.contains(&"emberflow-track-metadata-upsert"));
    assert!(tool_names.contains(&"emberflow-track-brief-replace"));
    assert!(tool_names.contains(&"emberflow-track-plan-replace"));
    assert!(tool_names.contains(&"emberflow-track-archive"));
    assert!(tool_names.contains(&"emberflow-track-delete"));
    assert!(tool_names.contains(&"emberflow-task-create"));
    assert!(tool_names.contains(&"emberflow-event-record"));
}

#[test]
fn emberflow_v1_auto_archives_tracks_on_durable_completion_transition() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();
    let track_id = "track-complete";

    runtime
        .create_track(track_id, "Finish EmberFlow runtime", "in-progress")
        .unwrap();
    runtime
        .upsert_track_metadata(TrackMetadataInput {
            track_id: track_id.to_string(),
            track_type: "feature".to_string(),
            status: "done".to_string(),
            description: "Finish EmberFlow runtime".to_string(),
            branch: "feature/example-branch".to_string(),
            spec_ref: Some("emberflow/specs/emberflow-v1.spec".to_string()),
        })
        .unwrap();

    let track = runtime.read_track(track_id).unwrap();
    assert_eq!(track.status, "archived");

    let overview = runtime.read_workspace_overview().unwrap();
    assert!(overview.tracks.iter().all(|item| item.track_id != track_id));
}

// @minter:integration emberflow-v1-exposes-queryable-runtime-tools
#[test]
fn emberflow_v1_keeps_archived_tracks_readable_but_not_active() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();

    runtime
        .create_track("track-active", "Active track", "in-progress")
        .unwrap();
    runtime
        .create_track("track-archived", "Archived track", "review")
        .unwrap();
    runtime.archive_track("track-archived").unwrap();

    let overview = runtime.read_workspace_overview().unwrap();
    let ids: Vec<_> = overview
        .tracks
        .iter()
        .map(|track| track.track_id.as_str())
        .collect();
    assert_eq!(ids, vec!["track-active"]);

    let archived = runtime.read_track("track-archived").unwrap();
    assert_eq!(archived.status, "archived");
}

// @minter:integration mcp-manual-archive-requires-terminal-track-status
#[test]
fn emberflow_v1_rejects_manual_archive_for_active_tracks() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();

    runtime
        .create_track("track-active", "Active track", "in-progress")
        .unwrap();

    let err = runtime.archive_track("track-active").unwrap_err();
    assert!(err.to_string().contains("review"));
    assert!(err.to_string().contains("done"));
    assert!(err.to_string().contains("in-progress"));

    let track = runtime.read_track("track-active").unwrap();
    assert_eq!(track.status, "in-progress");
}

// @minter:integration emberflow-v1-rejects-runtime-writes-outside-the-protocol
#[test]
fn emberflow_v1_rejects_runtime_writes_outside_the_protocol() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();

    let err = runtime
        .record_runtime_state(
            "track-001",
            "task-001",
            "retry",
            serde_json::json!({"summary": "Unsupported runtime write"}),
        )
        .unwrap_err();
    assert!(err.to_string().contains("unsupported"));

    assert!(runtime.read_track("track-001").is_err());
    assert!(runtime
        .list_events(Some("track-001"), Some("task-001"))
        .unwrap()
        .items
        .is_empty());
}
