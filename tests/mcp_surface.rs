use emberflow::mcp::surface::{
    EmberFlowSurface, TrackBriefSectionInput, TrackMetadataInput, TrackPlanItemInput,
    TrackPlanPhaseInput,
};
use emberflow::runtime::store::TaskInput;
use tempfile::tempdir;

fn task_input(
    task_id: &str,
    track_id: Option<&str>,
    title: &str,
    status: &str,
    phase: &str,
) -> TaskInput {
    TaskInput {
        task_id: task_id.to_string(),
        track_id: track_id.map(str::to_string),
        title: title.to_string(),
        status: status.to_string(),
        phase: phase.to_string(),
        executor: None,
        agent_instance_id: None,
        execution: None,
        intent_summary: None,
    }
}

fn canonical_track_metadata_input(track_id: &str) -> TrackMetadataInput {
    TrackMetadataInput {
        track_id: track_id.to_string(),
        track_type: "feature".to_string(),
        status: "in-progress".to_string(),
        description: "Expose canonical track context through EmberFlow MCP".to_string(),
        branch: "feature/mcp-surface".to_string(),
        spec_ref: Some("emberflow/specs/emberflow-mcp-surface.spec".to_string()),
    }
}

fn updated_canonical_track_metadata_input(track_id: &str) -> TrackMetadataInput {
    TrackMetadataInput {
        track_id: track_id.to_string(),
        track_type: "feature".to_string(),
        status: "in-progress".to_string(),
        description: "Expose canonical track context through EmberFlow MCP, updated".to_string(),
        branch: "feature/mcp-surface-v2".to_string(),
        spec_ref: Some("emberflow/specs/emberflow-mcp-surface.v2.spec".to_string()),
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

const EXPECTED_INITIALIZE_CAPABILITIES: &[&str] = &[
    "emberflow-track-create",
    "emberflow-track-metadata-upsert",
    "emberflow-track-brief-replace",
    "emberflow-track-plan-replace",
    "emberflow-track-archive",
    "emberflow-track-delete",
    "emberflow-task-create",
    "emberflow-event-record",
    "emberflow-task-claim",
    "emberflow-task-release",
];

const EXPECTED_RESOURCE_URIS: &[&str] = &[
    "emberflow://workspace/overview",
    "emberflow://tracks/{trackId}/record",
    "emberflow://tracks/{trackId}/resume",
    "emberflow://tracks/{trackId}/transparency",
    "emberflow://tracks/{trackId}/context",
    "emberflow://tracks/{trackId}/brief",
    "emberflow://tracks/{trackId}/plan",
    "emberflow://tracks/{trackId}/runtime",
    "emberflow://tracks/{trackId}/events",
    "emberflow://tasks/{taskId}/visibility",
    "emberflow://tasks/{taskId}/events",
    "emberflow://protocol/client-contract",
];

const EXPECTED_TRACK_BOOTSTRAP_SECTIONS: &[&str] = &[
    "objective",
    "context",
    "decisions",
    "non_goals",
    "current_state",
    "workspace_branch_pr_context",
    "next_step",
];

// @minter:integration mcp-initializes-with-capability-discovery
#[test]
fn mcp_initializes_with_capability_discovery() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    let init = surface.initialize().unwrap();

    assert_eq!(init.capabilities, EXPECTED_INITIALIZE_CAPABILITIES);
}

// @minter:integration mcp-initializes-with-self-description mcp-initializes-with-resource-discovery
#[test]
fn mcp_initializes_with_self_description() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    let init = surface.initialize().unwrap();

    assert_eq!(
        init.system_role,
        "canonical tracked runtime and visibility layer"
    );
    assert_eq!(init.source_of_truth, "emberflow-canonical-state");
    assert_eq!(init.projected_files, "derived-only");
    assert_eq!(
        init.preferred_client_sequence,
        vec![
            "initialize",
            "list_resources",
            "read_resource",
            "mutate_via_emberflow_mcp",
        ]
    );
    assert_eq!(
        init.knowledge_views
            .iter()
            .map(|view| view.name)
            .collect::<Vec<_>>(),
        vec![
            "workspace-overview",
            "track-record",
            "track-resume",
            "track-transparency",
            "track-context",
            "track-brief",
            "track-plan",
            "track-runtime",
            "track-events",
            "task-visibility",
            "task-events",
            "client-contract",
        ]
    );
    let resource_uris: Vec<_> = init
        .resource_views
        .iter()
        .map(|view| view.uri_template)
        .collect();
    assert_eq!(resource_uris, EXPECTED_RESOURCE_URIS);
}

// @minter:integration mcp-initializes-with-track-bootstrap
#[test]
fn mcp_initializes_with_track_bootstrap() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    let init = surface.initialize().unwrap();

    assert_eq!(init.track_bootstrap.brief_artifact, "brief.md");
    assert_eq!(
        init.track_bootstrap.required_sections,
        EXPECTED_TRACK_BOOTSTRAP_SECTIONS
    );
    assert_eq!(init, surface.initialize().unwrap());
}

// @minter:integration mcp-initializes-with-workspace-db-metadata
#[test]
fn mcp_initializes_with_workspace_db_metadata() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    let init = surface.initialize().unwrap();

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

// @minter:integration mcp-initializes-with-canonical-mode-by-default
#[test]
fn mcp_initializes_with_canonical_mode_by_default() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    let init = surface.initialize().unwrap();

    assert_eq!(init.workspace_db.projection_mode, "canonical");
}

// @minter:integration mcp-initializes-with-configured-projection-mode
#[test]
fn mcp_initializes_with_configured_projection_mode() {
    let tmp = tempdir().unwrap();
    std::fs::write(
        tmp.path().join("emberflow.config.json"),
        r#"{"mode":"projected"}"#,
    )
    .unwrap();

    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let init = surface.initialize().unwrap();

    assert_eq!(init.workspace_db.projection_mode, "projected");
    assert_eq!(
        init.workspace_db.default_path,
        tmp.path()
            .join(".emberflow/emberflow.db")
            .display()
            .to_string()
    );
}

// @minter:integration mcp-records-canonical-event
#[test]
fn mcp_records_canonical_event() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    surface
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Persist the first event",
            "running",
            "implementing",
        ))
        .unwrap();
    surface
        .claim_task("task-001", "assistant", Some(300))
        .unwrap();

    let event = surface
        .record_event(
            "event-001",
            Some("track-001"),
            Some("task-001"),
            "progress",
            serde_json::json!({"summary": "Writing the first event"}),
        )
        .unwrap();

    assert_eq!(event.id, "event-001");
    assert_eq!(event.kind, "progress");
    assert!(event.payload.is_object());
}

// @minter:integration mcp-creates-task-with-canonical-visibility
#[test]
fn mcp_returns_task_visibility_fields() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();

    let task = surface
        .create_task(TaskInput {
            task_id: "task-visibility".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Investigate the transport contract".to_string(),
            status: "running".to_string(),
            phase: "planning".to_string(),
            executor: Some("assistant".to_string()),
            agent_instance_id: Some("claude-1".to_string()),
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Investigate the transport contract end to end".to_string()),
        })
        .unwrap();

    assert_eq!(task.executor.as_deref(), Some("assistant"));
    assert_eq!(task.agent_instance_id.as_deref(), Some("claude-1"));
    assert_eq!(task.execution.as_deref(), Some("interactive session"));
    assert_eq!(
        task.intent_summary.as_deref(),
        Some("Investigate the transport contract end to end")
    );
}

// @minter:integration mcp-creates-task-with-generic-executor-fallback
#[test]
fn mcp_create_task_falls_back_to_generic_executor() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    let task = surface
        .create_task(TaskInput {
            task_id: "task-standalone".to_string(),
            track_id: None,
            title: "Inspect recent events".to_string(),
            status: "running".to_string(),
            phase: "verifying".to_string(),
            executor: None,
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Inspect recent events for the user".to_string()),
        })
        .unwrap();

    assert_eq!(task.executor.as_deref(), Some("assistant"));
    assert_eq!(task.execution.as_deref(), Some("interactive session"));
    assert_eq!(
        task.intent_summary.as_deref(),
        Some("Inspect recent events for the user")
    );
}

// @minter:integration mcp-reads-workspace-overview-resource mcp-reads-track-record-resource
#[test]
fn mcp_excludes_archived_tracks_from_active_overview_but_keeps_direct_reads() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-active", "Active Track", "in-progress")
        .unwrap();
    surface
        .create_track("track-archived", "Archived Track", "review")
        .unwrap();
    surface.archive_track("track-archived").unwrap();

    let overview = surface
        .read_resource("emberflow://workspace/overview")
        .unwrap();
    let track_ids: Vec<_> = overview
        .content
        .get("tracks")
        .and_then(|value| value.as_array())
        .unwrap()
        .iter()
        .filter_map(|value| value.get("trackId").and_then(|value| value.as_str()))
        .collect();

    assert_eq!(track_ids, vec!["track-active"]);

    let archived_record = surface
        .read_resource("emberflow://tracks/track-archived/record")
        .unwrap();
    assert_eq!(
        archived_record
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some("track-archived")
    );
    assert_eq!(
        archived_record
            .content
            .get("status")
            .and_then(|value| value.as_str()),
        Some("archived")
    );
}

// @minter:integration delete-track-removes-associated-runtime-state
#[test]
fn mcp_can_archive_and_delete_tracks_manually() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Manual lifecycle", "review")
        .unwrap();
    surface
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Persist the first event",
            "running",
            "implementing",
        ))
        .unwrap();

    let archived = surface.archive_track("track-001").unwrap();
    assert_eq!(archived.status, "archived");

    surface.delete_track("track-001").unwrap();

    let err = surface
        .read_resource("emberflow://tracks/track-001/record")
        .unwrap_err();
    assert!(err.to_string().contains("track-001"));

    let task_err = surface
        .read_resource("emberflow://tasks/task-001/visibility")
        .unwrap_err();
    assert!(task_err.to_string().contains("task-001"));

    assert!(surface.list_tasks().unwrap().is_empty());
}

// @minter:integration mcp-manual-archive-requires-terminal-track-status
#[test]
fn mcp_rejects_manual_archive_for_active_track() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Manual lifecycle", "in-progress")
        .unwrap();

    let err = surface.archive_track("track-001").unwrap_err();
    assert!(err.to_string().contains("review"));
    assert!(err.to_string().contains("done"));
    assert!(err.to_string().contains("in-progress"));

    let track = surface
        .read_resource("emberflow://tracks/track-001/record")
        .unwrap();
    assert_eq!(
        track.content.get("status").and_then(|value| value.as_str()),
        Some("in-progress")
    );
}

// @minter:integration mcp-records-canonical-event
#[test]
fn mcp_auto_archives_track_when_completion_event_is_recorded() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Completed Track", "in-progress")
        .unwrap();
    surface
        .record_event(
            "event-done",
            Some("track-001"),
            None,
            "close",
            serde_json::json!({
                "status": "done",
                "completed": true,
                "summary": "All work finished"
            }),
        )
        .unwrap();

    let archived = surface
        .read_resource("emberflow://tracks/track-001/record")
        .unwrap();
    assert_eq!(
        archived
            .content
            .get("status")
            .and_then(|value| value.as_str()),
        Some("archived")
    );

    let overview = surface
        .read_resource("emberflow://workspace/overview")
        .unwrap();
    assert!(overview
        .content
        .get("tracks")
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
}

#[test]
fn mcp_reads_track_record_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "in-progress")
        .unwrap();

    let track = surface
        .read_resource("emberflow://tracks/track-001/record")
        .unwrap();

    assert_eq!(track.uri, "emberflow://tracks/track-001/record");
    assert_eq!(track.name, "track-record");
    assert_eq!(
        track
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert_eq!(
        track.content.get("status").and_then(|value| value.as_str()),
        Some("in-progress")
    );
}

// @minter:integration mcp-reads-workspace-overview-resource
#[test]
fn mcp_reads_workspace_overview_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "in-progress")
        .unwrap();
    surface
        .upsert_track_metadata(canonical_track_metadata_input("track-001"))
        .unwrap();
    surface
        .create_task(TaskInput {
            task_id: "task-001".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Expose workspace overview".to_string(),
            status: "running".to_string(),
            phase: "planning".to_string(),
            executor: Some("assistant".to_string()),
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Expose workspace overview".to_string()),
        })
        .unwrap();

    let overview = surface
        .read_resource("emberflow://workspace/overview")
        .unwrap();
    assert_eq!(overview.uri, "emberflow://workspace/overview");
    assert_eq!(overview.name, "workspace-overview");
    assert_eq!(
        overview
            .content
            .get("tracks")
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        overview
            .content
            .get("tracks")
            .and_then(|value| value.as_array())
            .unwrap()[0]
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some("track-001")
    );
}

// @minter:integration mcp-reads-track-resume-resource
#[test]
fn mcp_reads_track_resume_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "in-progress")
        .unwrap();
    surface
        .upsert_track_metadata(canonical_track_metadata_input("track-001"))
        .unwrap();
    surface
        .replace_track_brief("track-001", brief_sections())
        .unwrap();
    surface
        .replace_track_plan("track-001", canonical_plan())
        .unwrap();
    surface
        .create_task(TaskInput {
            task_id: "task-001".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Resume tracked work".to_string(),
            status: "running".to_string(),
            phase: "planning".to_string(),
            executor: Some("assistant".to_string()),
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Resume tracked work".to_string()),
        })
        .unwrap();

    let resume = surface
        .read_resource("emberflow://tracks/track-001/resume")
        .unwrap();
    assert_eq!(resume.uri, "emberflow://tracks/track-001/resume");
    assert_eq!(resume.name, "track-resume");
    assert_eq!(
        resume
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert_eq!(
        resume
            .content
            .get("intentSummary")
            .and_then(|value| value.as_str()),
        Some("Resume tracked work")
    );
    assert_eq!(
        resume
            .content
            .get("summarySections")
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        resume
            .content
            .get("plan")
            .and_then(|value| value.get("phases"))
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        2
    );
}

// @minter:integration mcp-reads-track-events-resource
#[test]
fn mcp_reads_track_events_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    surface
        .record_event(
            "event-001",
            Some("track-001"),
            None,
            "progress",
            serde_json::json!({"summary": "Writing the first event"}),
        )
        .unwrap();
    surface
        .record_event(
            "event-002",
            Some("track-001"),
            None,
            "progress",
            serde_json::json!({"summary": "Persisting projections"}),
        )
        .unwrap();

    let feed = surface
        .read_resource("emberflow://tracks/track-001/events")
        .unwrap();

    assert_eq!(feed.uri, "emberflow://tracks/track-001/events");
    assert_eq!(feed.name, "track-events");
    assert_eq!(
        feed.content.get("trackId").and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert!(
        feed.content
            .get("items")
            .and_then(|value| value.as_array())
            .unwrap()
            .len()
            >= 2
    );
}

// @minter:integration mcp-reads-empty-track-events-resource
#[test]
fn mcp_reads_empty_track_events_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-empty", "Empty track", "planning")
        .unwrap();

    let feed = surface
        .read_resource("emberflow://tracks/track-empty/events")
        .unwrap();

    assert_eq!(
        feed.content.get("trackId").and_then(|value| value.as_str()),
        Some("track-empty")
    );
    assert!(feed
        .content
        .get("items")
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
}

// @minter:integration mcp-rejects-unsupported-event-kind
#[test]
fn mcp_rejects_unsupported_event_kind() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    let err = surface
        .record_event(
            "event-invalid",
            Some("track-001"),
            None,
            "retry",
            serde_json::json!({"summary": "Unsupported event"}),
        )
        .unwrap_err();
    assert!(err.to_string().contains("unsupported"));
}

// @minter:integration mcp-reads-track-runtime-resource mcp-reads-track-transparency-resource mcp-reads-task-visibility-resource
#[test]
fn mcp_reads_track_runtime_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    surface
        .create_task(TaskInput {
            task_id: "task-001".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Persist the first event".to_string(),
            status: "running".to_string(),
            phase: "implementing".to_string(),
            executor: Some("dev-implementer".to_string()),
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Writing the first event".to_string()),
        })
        .unwrap();
    surface
        .claim_task("task-001", "dev-implementer", Some(300))
        .unwrap();
    surface
        .record_event(
            "event-001",
            Some("track-001"),
            Some("task-001"),
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

    let runtime = surface
        .read_resource("emberflow://tracks/track-001/runtime")
        .unwrap();

    assert_eq!(runtime.uri, "emberflow://tracks/track-001/runtime");
    assert_eq!(runtime.name, "track-runtime");
    assert_eq!(
        runtime
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert_eq!(
        runtime
            .content
            .get("taskId")
            .and_then(|value| value.as_str()),
        Some("task-001")
    );
    assert_eq!(
        runtime
            .content
            .get("executor")
            .and_then(|value| value.as_str()),
        Some("dev-implementer")
    );
    assert_eq!(
        runtime
            .content
            .get("intentSummary")
            .and_then(|value| value.as_str()),
        Some("Writing the first event")
    );
    assert_eq!(
        runtime
            .content
            .get("source")
            .and_then(|value| value.as_str()),
        Some("derived-projection")
    );
    assert_eq!(
        runtime
            .content
            .get("phase")
            .and_then(|value| value.as_str()),
        Some("implementing")
    );
}

#[test]
fn mcp_reads_track_transparency_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    surface
        .create_task(TaskInput {
            task_id: "task-001".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Persist the first event".to_string(),
            status: "running".to_string(),
            phase: "implementing".to_string(),
            executor: Some("dev-implementer".to_string()),
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Writing the first event".to_string()),
        })
        .unwrap();
    surface
        .claim_task("task-001", "dev-implementer", Some(300))
        .unwrap();
    surface
        .record_event(
            "event-001",
            Some("track-001"),
            Some("task-001"),
            "progress",
            serde_json::json!({
                "agent": "dev-implementer",
                "executor": "dev-implementer",
                "status": "running",
                "phase": "implementing",
                "summary": "Writing the first event",
                "recommended_next_step": "Review the canonical transparency block"
            }),
        )
        .unwrap();

    let transparency = surface
        .read_resource("emberflow://tracks/track-001/transparency")
        .unwrap();

    assert_eq!(
        transparency.uri,
        "emberflow://tracks/track-001/transparency"
    );
    assert_eq!(transparency.name, "track-transparency");
    assert_eq!(
        transparency
            .content
            .get("source")
            .and_then(|value| value.as_str()),
        Some("emberflow-canonical-state")
    );
    assert_eq!(
        transparency
            .content
            .get("trackStatus")
            .and_then(|value| value.as_str()),
        Some("planning")
    );
    assert_eq!(
        transparency
            .content
            .get("taskStatus")
            .and_then(|value| value.as_str()),
        Some("running")
    );
    assert_eq!(
        transparency
            .content
            .get("phase")
            .and_then(|value| value.as_str()),
        Some("implementing")
    );
    assert_eq!(
        transparency
            .content
            .get("next")
            .and_then(|value| value.as_str()),
        Some("Review the canonical transparency block")
    );
}

// @minter:integration mcp-reads-track-context-resource
#[test]
fn mcp_reads_track_context_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();
    surface
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    surface
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    let context = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/context"))
        .unwrap();

    assert_eq!(
        context.uri,
        format!("emberflow://tracks/{track_id}/context")
    );
    assert_eq!(context.name, "track-context");
    assert_eq!(
        context
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        context
            .content
            .get("metadata")
            .and_then(|value| value.get("trackId"))
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        context
            .content
            .get("brief")
            .and_then(|value| value.get("sections"))
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        context
            .content
            .get("plan")
            .and_then(|value| value.get("phases"))
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        2
    );
}

// @minter:integration mcp-lists-readable-resource-catalog
#[test]
fn mcp_lists_readable_resource_catalog() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    let resources = surface.list_resources();
    let resource_uris: Vec<_> = resources.iter().map(|view| view.uri_template).collect();

    assert_eq!(resource_uris, EXPECTED_RESOURCE_URIS);
    assert!(resources
        .iter()
        .all(|view| view.mime_type == "application/json"));
}

// @minter:integration mcp-reads-workspace-overview-resource mcp-reads-track-record-resource
#[test]
fn mcp_archives_and_deletes_tracks_via_surface() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let archived_track_id = "track-archive";
    let deleted_track_id = "track-delete";

    surface
        .create_track(archived_track_id, "Archive this track", "review")
        .unwrap();
    surface
        .create_track(deleted_track_id, "Delete this track", "planning")
        .unwrap();

    let archived = surface.archive_track(archived_track_id).unwrap();
    assert_eq!(archived.status, "archived");

    let overview = surface
        .read_resource("emberflow://workspace/overview")
        .unwrap();
    let overview_tracks = overview
        .content
        .get("tracks")
        .and_then(|value| value.as_array())
        .unwrap();
    assert!(overview_tracks.iter().all(|track| {
        track.get("trackId").and_then(|value| value.as_str()) != Some(archived_track_id)
    }));

    let archived_record = surface
        .read_resource("emberflow://tracks/track-archive/record")
        .unwrap();
    assert_eq!(
        archived_record
            .content
            .get("status")
            .and_then(|value| value.as_str()),
        Some("archived")
    );

    surface.delete_track(deleted_track_id).unwrap();
    assert!(surface
        .read_resource("emberflow://tracks/track-delete/record")
        .is_err());
}

// @minter:integration mcp-reads-workspace-overview-resource mcp-reads-track-resume-resource mcp-reads-track-transparency-resource mcp-reads-track-plan-resource mcp-reads-track-brief-resource mcp-reads-track-runtime-resource mcp-reads-task-visibility-resource mcp-reads-client-contract-resource
#[test]
fn mcp_reads_readable_resource_views() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();
    surface
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    surface
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();
    surface
        .create_task(TaskInput {
            task_id: "task-resource".to_string(),
            track_id: Some(track_id.to_string()),
            title: "Inspect EmberFlow resource views".to_string(),
            status: "running".to_string(),
            phase: "planning".to_string(),
            executor: Some("assistant".to_string()),
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Inspect EmberFlow resource views".to_string()),
        })
        .unwrap();
    surface
        .claim_task("task-resource", "assistant", Some(300))
        .unwrap();
    surface
        .record_event(
            "event-resource-001",
            Some(track_id),
            Some("task-resource"),
            "progress",
            serde_json::json!({"summary": "Inspect EmberFlow resource views"}),
        )
        .unwrap();

    let overview = surface
        .read_resource("emberflow://workspace/overview")
        .unwrap();
    assert_eq!(overview.uri, "emberflow://workspace/overview");
    assert_eq!(overview.name, "workspace-overview");
    assert_eq!(overview.mime_type, "application/json");
    assert_eq!(
        overview
            .content
            .get("tracks")
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        1
    );

    let resume = surface
        .read_resource("emberflow://tracks/emberflow-runtime-split-20260402/resume")
        .unwrap();
    assert_eq!(
        resume
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        resume
            .content
            .get("executor")
            .and_then(|value| value.as_str()),
        Some("assistant")
    );
    assert_eq!(
        resume
            .content
            .get("summarySections")
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        4
    );

    let transparency = surface
        .read_resource("emberflow://tracks/emberflow-runtime-split-20260402/transparency")
        .unwrap();
    assert_eq!(
        transparency
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        transparency
            .content
            .get("taskStatus")
            .and_then(|value| value.as_str()),
        Some("running")
    );
    assert_eq!(
        transparency
            .content
            .get("phase")
            .and_then(|value| value.as_str()),
        Some("planning")
    );

    let plan = surface
        .read_resource("emberflow://tracks/emberflow-runtime-split-20260402/plan")
        .unwrap();
    assert_eq!(
        plan.content.get("trackId").and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        plan.content
            .get("phases")
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        2
    );

    let brief = surface
        .read_resource("emberflow://tracks/emberflow-runtime-split-20260402/brief")
        .unwrap();
    assert_eq!(
        brief
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        brief
            .content
            .get("sections")
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        4
    );

    let runtime = surface
        .read_resource("emberflow://tracks/emberflow-runtime-split-20260402/runtime")
        .unwrap();
    assert_eq!(
        runtime
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        runtime
            .content
            .get("executor")
            .and_then(|value| value.as_str()),
        Some("assistant")
    );
    assert_eq!(
        runtime
            .content
            .get("source")
            .and_then(|value| value.as_str()),
        Some("derived-projection")
    );
    assert_eq!(
        runtime
            .content
            .get("phase")
            .and_then(|value| value.as_str()),
        Some("planning")
    );

    let visibility = surface
        .read_resource("emberflow://tasks/task-resource/visibility")
        .unwrap();
    assert_eq!(
        visibility
            .content
            .get("taskId")
            .and_then(|value| value.as_str()),
        Some("task-resource")
    );
    assert_eq!(
        visibility
            .content
            .get("executor")
            .and_then(|value| value.as_str()),
        Some("assistant")
    );
    assert_eq!(
        visibility
            .content
            .get("trackStatus")
            .and_then(|value| value.as_str()),
        Some("in-progress")
    );

    let task_events = surface
        .read_resource("emberflow://tasks/task-resource/events")
        .unwrap();
    assert_eq!(
        task_events
            .content
            .get("taskId")
            .and_then(|value| value.as_str()),
        Some("task-resource")
    );
    assert!(!task_events
        .content
        .get("items")
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());

    let contract = surface
        .read_resource("emberflow://protocol/client-contract")
        .unwrap();
    assert_eq!(
        contract
            .content
            .get("source")
            .and_then(|value| value.as_str()),
        Some("emberflow-canonical-state")
    );
    assert!(contract
        .content
        .get("resources")
        .and_then(|value| value.as_array())
        .unwrap()
        .iter()
        .any(|value| value.as_str() == Some("emberflow://workspace/overview")));
    assert_eq!(
        contract
            .content
            .get("transparency")
            .and_then(|value| value.get("resource"))
            .and_then(|value| value.as_str()),
        Some("emberflow://tracks/{trackId}/transparency")
    );
}

// @minter:integration mcp-reads-empty-track-context-resource
#[test]
fn mcp_reads_empty_track_context_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    let context = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/context"))
        .unwrap();

    assert_eq!(
        context
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert!(context
        .content
        .get("brief")
        .and_then(|value| value.get("sections"))
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
    assert!(context
        .content
        .get("plan")
        .and_then(|value| value.get("phases"))
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
}

// @minter:integration mcp-reads-track-brief-resource
#[test]
fn mcp_reads_track_brief_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";
    let sections = brief_sections();

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();
    surface
        .replace_track_brief(
            track_id,
            vec![
                sections[3].clone(),
                sections[0].clone(),
                sections[1].clone(),
            ],
        )
        .unwrap();

    let brief = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/brief"))
        .unwrap();

    assert_eq!(
        brief
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        brief
            .content
            .get("sections")
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        3
    );
}

// @minter:integration mcp-reads-empty-track-brief-resource
#[test]
fn mcp_reads_empty_track_brief_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    let brief = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/brief"))
        .unwrap();

    assert_eq!(
        brief
            .content
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert!(brief
        .content
        .get("sections")
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
}

// @minter:integration mcp-reads-track-plan-resource
#[test]
fn mcp_reads_track_plan_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();
    surface
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    let plan = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/plan"))
        .unwrap();

    assert_eq!(
        plan.content.get("trackId").and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert_eq!(
        plan.content
            .get("phases")
            .and_then(|value| value.as_array())
            .unwrap()
            .len(),
        2
    );
}

// @minter:integration mcp-reads-empty-track-plan-resource
#[test]
fn mcp_reads_empty_track_plan_resource() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    let plan = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/plan"))
        .unwrap();

    assert_eq!(
        plan.content.get("trackId").and_then(|value| value.as_str()),
        Some(track_id)
    );
    assert!(plan
        .content
        .get("phases")
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
}

// @minter:integration mcp-upserts-canonical-track-metadata
#[test]
fn mcp_upserts_canonical_track_metadata() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    let metadata = surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    assert_eq!(metadata.track_id, track_id);
    assert_eq!(metadata.track_type, "feature");
    assert_eq!(metadata.status, "in-progress");
    assert_eq!(metadata.branch, "feature/mcp-surface");
    assert_eq!(
        metadata.spec_ref.as_deref(),
        Some("emberflow/specs/emberflow-mcp-surface.spec")
    );
}

// @minter:integration mcp-updates-canonical-track-metadata
#[test]
fn mcp_updates_canonical_track_metadata() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    let metadata = surface
        .upsert_track_metadata(updated_canonical_track_metadata_input(track_id))
        .unwrap();

    assert_eq!(metadata.track_id, track_id);
    assert_eq!(
        metadata.description,
        "Expose canonical track context through EmberFlow MCP, updated"
    );
    assert_eq!(metadata.branch, "feature/mcp-surface-v2");
    assert_eq!(
        metadata.spec_ref.as_deref(),
        Some("emberflow/specs/emberflow-mcp-surface.v2.spec")
    );

    let context = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/context"))
        .unwrap();
    assert_eq!(
        context
            .content
            .get("metadata")
            .and_then(|value| value.get("description"))
            .and_then(|value| value.as_str()),
        Some(metadata.description.as_str())
    );
    assert!(context
        .content
        .get("brief")
        .and_then(|value| value.get("sections"))
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
    assert!(context
        .content
        .get("plan")
        .and_then(|value| value.get("phases"))
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
}

// @minter:integration mcp-rejects-invalid-track-brief-section
#[test]
fn mcp_rejects_invalid_track_brief_section() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    let err = surface
        .replace_track_brief(
            track_id,
            vec![TrackBriefSectionInput {
                section_key: String::new(),
                section_text: "Broken brief section".to_string(),
                position: 0,
            }],
        )
        .unwrap_err();

    assert!(err.to_string().contains("section key"));
    assert!(err.to_string().contains("present"));
    let brief = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/brief"))
        .unwrap();
    assert!(brief
        .content
        .get("sections")
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
}

// @minter:integration mcp-rejects-invalid-track-plan-item-placement
#[test]
fn mcp_rejects_invalid_track_plan_item_placement() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    let err = surface
        .replace_track_plan(
            track_id,
            vec![TrackPlanPhaseInput {
                phase_id: "phase-1".to_string(),
                title: "Canonical schema design".to_string(),
                position: 0,
                items: vec![TrackPlanItemInput {
                    item_id: "phase-1/item-1".to_string(),
                    title: "Define canonical metadata tables".to_string(),
                    position: None,
                }],
            }],
        )
        .unwrap_err();

    assert!(err.to_string().contains("stable placement"));
    assert!(err.to_string().contains("item"));
    let plan = surface
        .read_resource(&format!("emberflow://tracks/{track_id}/plan"))
        .unwrap();
    assert!(plan
        .content
        .get("phases")
        .and_then(|value| value.as_array())
        .unwrap()
        .is_empty());
}

// @minter:integration mcp-replaces-canonical-track-brief
#[test]
fn mcp_replaces_canonical_track_brief() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    let brief = surface
        .replace_track_brief(track_id, brief_sections())
        .unwrap();

    assert_eq!(brief.track_id, track_id);
    assert_eq!(brief.sections.len(), 4);
    assert_eq!(brief.sections[0].section_key, "objective");
    assert_eq!(brief.sections[3].section_key, "next_step");
}

// @minter:integration mcp-replaces-canonical-track-plan
#[test]
fn mcp_replaces_canonical_track_plan() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    surface
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();

    let plan = surface
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    assert_eq!(plan.track_id, track_id);
    assert_eq!(plan.phases.len(), 2);
    assert_eq!(plan.phases[0].phase_id, "phase-1");
    assert_eq!(plan.phases[1].items[0].item_id, "phase-2/item-1");
}

// @minter:unit claim-task-acquires-exclusive-lease
// @minter:unit new-event-kinds-claim-release-lease-expired
#[test]
fn surface_claim_task_succeeds() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_db_path(tmp.path().join("emberflow.db")).unwrap();
    surface
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    surface
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    let lease = surface
        .claim_task("task-001", "agent-a", Some(300))
        .unwrap();

    assert_eq!(lease.holder, "agent-a");
    assert!(!lease.acquired_at.is_empty());
    assert!(lease.expires_at.is_some());
}

// @minter:unit release-clears-lease
#[test]
fn surface_release_task_succeeds() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_db_path(tmp.path().join("emberflow.db")).unwrap();
    surface
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    surface
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    surface
        .claim_task("task-001", "agent-a", Some(300))
        .unwrap();
    surface.release_task("task-001", "agent-a").unwrap();
}

// @minter:unit different-holder-claim-rejected
#[test]
fn surface_claim_conflict_returns_error() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_db_path(tmp.path().join("emberflow.db")).unwrap();
    surface
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    surface
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    surface
        .claim_task("task-001", "agent-a", Some(300))
        .unwrap();
    let err = surface
        .claim_task("task-001", "agent-b", Some(300))
        .unwrap_err();

    assert!(err.to_string().contains("already held"), "error was: {err}");
}

// @minter:unit event-record-gated-by-lease
#[test]
fn surface_event_record_gated_by_lease() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_db_path(tmp.path().join("emberflow.db")).unwrap();
    surface
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    surface
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    // agent-a holds the lease
    surface
        .claim_task("task-001", "agent-a", Some(300))
        .unwrap();

    // agent-b tries to record an event for the task
    let err = surface
        .record_event(
            "event-001",
            Some("track-001"),
            Some("task-001"),
            "progress",
            serde_json::json!({ "executor": "agent-b", "message": "unauthorized update" }),
        )
        .unwrap_err();

    assert!(
        err.to_string().contains("lease held by"),
        "error was: {err}"
    );
}

// @minter:unit task-event-requires-active-lease
#[test]
fn surface_unclaimed_task_event_is_rejected() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_db_path(tmp.path().join("emberflow.db")).unwrap();
    surface
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    surface
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    // No lease — any write targeting the task must be rejected
    let err = surface
        .record_event(
            "event-001",
            Some("track-001"),
            Some("task-001"),
            "progress",
            serde_json::json!({ "executor": "agent-x", "message": "free update" }),
        )
        .unwrap_err();

    assert!(
        err.to_string().contains("no active lease"),
        "error was: {err}"
    );
}

// @minter:unit lease-state-in-transparency
#[test]
fn surface_lease_state_in_task_visibility() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_db_path(tmp.path().join("emberflow.db")).unwrap();
    surface
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    surface
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    surface
        .claim_task("task-001", "agent-a", Some(3600))
        .unwrap();

    let resource = surface
        .read_resource("emberflow://tasks/task-001/visibility")
        .unwrap();

    let content = &resource.content;
    assert_eq!(content["leaseHolder"], "agent-a");
    assert!(
        !content["leaseExpiresAt"].is_null(),
        "leaseExpiresAt should be present"
    );
}
