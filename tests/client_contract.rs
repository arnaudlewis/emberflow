use emberflow::mcp::server::{start_stdio_server, StdioTransportConfig};
use emberflow::mcp::surface::{
    EmberFlowSurface, TrackBriefSectionInput, TrackMetadataInput, TrackPlanItemInput,
    TrackPlanPhaseInput,
};
use serde_json::json;
use std::collections::BTreeSet;
use tempfile::tempdir;

fn canonical_track_metadata_input(track_id: &str) -> TrackMetadataInput {
    TrackMetadataInput {
        track_id: track_id.to_string(),
        track_type: "feature".to_string(),
        status: "in-progress".to_string(),
        description: "Resume work through EmberFlow".to_string(),
        branch: "feature/track-surface".to_string(),
        spec_ref: Some("emberflow/specs/emberflow-client-contract.spec".to_string()),
    }
}

fn brief_sections() -> Vec<TrackBriefSectionInput> {
    vec![
        TrackBriefSectionInput {
            section_key: "objective".to_string(),
            section_text: "Keep the client contract stable".to_string(),
            position: 0,
        },
        TrackBriefSectionInput {
            section_key: "context".to_string(),
            section_text: "The client should resume from EmberFlow canonical state".to_string(),
            position: 1,
        },
        TrackBriefSectionInput {
            section_key: "next_step".to_string(),
            section_text: "Display the current EmberFlow state before resuming work".to_string(),
            position: 2,
        },
    ]
}

fn canonical_plan() -> Vec<TrackPlanPhaseInput> {
    vec![TrackPlanPhaseInput {
        phase_id: "phase-1".to_string(),
        title: "Bootstrap and resume".to_string(),
        position: 0,
        items: vec![TrackPlanItemInput {
            item_id: "phase-1/item-1".to_string(),
            title: "Initialize before tracked work".to_string(),
            position: Some(0),
        }],
    }]
}

fn seed_workspace(path: &std::path::Path) {
    let surface = EmberFlowSurface::from_workspace_root(path).unwrap();
    surface
        .create_track("track-001", "Client contract bootstrap", "in-progress")
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
}

// @minter:integration client-initializes-before-tracked-work client-reads-self-describing-emberflow-bootstrap client-loads-minimal-context-before-resume durable-mutations-use-emberflow-surface client-may-list-readable-resources-after-bootstrap client-may-read-dynamic-knowledge-views-before-resume client-may-read-display-ready-transparency-resource transparency-may-include-enriched-task-visibility
#[test]
fn client_contract_bootstraps_and_resumes_via_emberflow_surface() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());

    let session = start_stdio_server(StdioTransportConfig {
        cwd: Some(tmp.path().to_path_buf()),
        workspace_root: None,
        state_path: None,
    })
    .unwrap();

    let init = session.initialize().unwrap();
    let capabilities: BTreeSet<_> = init.capabilities.into_iter().collect();
    assert_eq!(
        capabilities,
        [
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
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        init.workspace_db.project_root,
        tmp.path().display().to_string()
    );
    assert_eq!(
        init.system_role,
        "canonical tracked runtime and visibility layer"
    );
    assert_eq!(init.source_of_truth, "emberflow-canonical-state");
    assert_eq!(init.projected_files, "derived-only");
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
    assert!(resource_uris.contains(&"emberflow://workspace/overview"));
    assert!(resource_uris.contains(&"emberflow://tracks/{trackId}/record"));
    assert!(resource_uris.contains(&"emberflow://tracks/{trackId}/resume"));
    assert!(resource_uris.contains(&"emberflow://tracks/{trackId}/transparency"));
    assert!(resource_uris.contains(&"emberflow://tracks/{trackId}/context"));
    assert!(resource_uris.contains(&"emberflow://tracks/{trackId}/brief"));
    assert!(resource_uris.contains(&"emberflow://tracks/{trackId}/plan"));
    assert!(resource_uris.contains(&"emberflow://tracks/{trackId}/runtime"));
    assert!(resource_uris.contains(&"emberflow://tracks/{trackId}/events"));
    assert!(resource_uris.contains(&"emberflow://tasks/{taskId}/visibility"));
    assert!(resource_uris.contains(&"emberflow://tasks/{taskId}/events"));
    assert!(resource_uris.contains(&"emberflow://protocol/client-contract"));

    let overview = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://workspace/overview"}),
        )
        .unwrap();
    assert_eq!(
        overview
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("source"))
            .and_then(|value| value.as_str()),
        Some("emberflow-canonical-state")
    );

    let resources = session.call("resources/list", json!({})).unwrap();
    let listed_resources = resources
        .get("resources")
        .and_then(|value| value.as_array())
        .unwrap();
    assert!(listed_resources
        .iter()
        .any(|value| value.get("uri").and_then(|uri| uri.as_str())
            == Some("emberflow://workspace/overview")));
    assert!(listed_resources
        .iter()
        .all(|value| value.get("uriTemplate").is_none()));

    let resume = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/resume"}),
        )
        .unwrap();
    assert_eq!(
        resume
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("trackId"))
            .and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert!(resume
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("summarySections"))
        .is_some());

    let initial_transparency = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/transparency"}),
        )
        .unwrap();
    assert_eq!(
        initial_transparency
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("trackId"))
            .and_then(|value| value.as_str()),
        Some("track-001")
    );

    let context = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/context"}),
        )
        .unwrap();
    assert_eq!(
        context
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("trackId"))
            .and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert!(context
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("metadata"))
        .is_some());
    assert!(context
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("brief"))
        .is_some());
    assert!(context
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("plan"))
        .is_some());

    let task = session
        .call(
            "emberflow-task-create",
            json!({
                "taskId": "task-001",
                "trackId": "track-001",
                "title": "Initialize tracked work before resume",
                "status": "running",
                "phase": "planning",
                "executor": "assistant",
                "execution": "interactive session",
                "intentSummary": "Initialize tracked work before resume",
                "agentType": "dev-implementer"
            }),
        )
        .unwrap();
    assert_eq!(
        task.get("id").and_then(|value| value.as_str()),
        Some("task-001")
    );
    assert_eq!(
        task.get("executor").and_then(|value| value.as_str()),
        Some("assistant")
    );
    assert_eq!(
        task.get("execution").and_then(|value| value.as_str()),
        Some("interactive session")
    );
    assert_eq!(
        task.get("intentSummary").and_then(|value| value.as_str()),
        Some("Initialize tracked work before resume")
    );

    session
        .call(
            "emberflow-task-claim",
            json!({
                "taskId": "task-001",
                "holder": "assistant",
                "durationSecs": 300
            }),
        )
        .unwrap();

    let event = session
        .call(
            "emberflow-event-record",
            json!({
                "eventId": "event-001",
                "trackId": "track-001",
                "taskId": "task-001",
                "kind": "progress",
                "payload": {
                    "summary": "Initialized tracked work through EmberFlow",
                    "recommended_next_step": "Display the current EmberFlow state to the user"
                }
            }),
        )
        .unwrap();
    assert_eq!(
        event.get("id").and_then(|value| value.as_str()),
        Some("event-001")
    );
    assert_eq!(
        event.get("kind").and_then(|value| value.as_str()),
        Some("progress")
    );

    let runtime_status = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/runtime"}),
        )
        .unwrap();
    assert_eq!(
        runtime_status
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("trackId"))
            .and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert!(runtime_status
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("statusLine"))
        .is_some());
    assert!(runtime_status
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("source"))
        .is_some());
    assert_eq!(
        runtime_status
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("executor"))
            .and_then(|value| value.as_str()),
        Some("assistant")
    );
    assert_eq!(
        runtime_status
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("execution"))
            .and_then(|value| value.as_str()),
        Some("interactive session")
    );
    assert_eq!(
        runtime_status
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("intentSummary"))
            .and_then(|value| value.as_str()),
        Some("Initialize tracked work before resume")
    );
    assert_eq!(
        runtime_status
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("next"))
            .and_then(|value| value.as_str()),
        Some("Display the current EmberFlow state to the user")
    );
    assert_eq!(
        runtime_status
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("phase"))
            .and_then(|value| value.as_str()),
        Some("planning")
    );

    let updated_transparency = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/transparency"}),
        )
        .unwrap();
    assert_eq!(
        updated_transparency
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("taskStatus"))
            .and_then(|value| value.as_str()),
        Some("running")
    );
}

// @minter:integration client-may-list-readable-resources-after-bootstrap client-may-read-dynamic-knowledge-views-before-resume durable-mutations-use-emberflow-surface
#[test]
fn client_contract_hides_archived_tracks_from_active_discovery_but_allows_direct_reads() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());

    let session = start_stdio_server(StdioTransportConfig {
        cwd: Some(tmp.path().to_path_buf()),
        workspace_root: None,
        state_path: None,
    })
    .unwrap();

    session
        .call(
            "emberflow-track-metadata-upsert",
            json!({
                "trackId": "track-001",
                "trackType": "feature",
                "status": "review",
                "description": "Resume work through EmberFlow",
                "branch": "feature/track-surface",
                "specRef": "emberflow/specs/emberflow-client-contract.spec"
            }),
        )
        .unwrap();

    session
        .call(
            "emberflow-track-archive",
            json!({
                "trackId": "track-001"
            }),
        )
        .unwrap();

    let overview = session
        .call(
            "read-resource",
            json!({
                "uri": "emberflow://workspace/overview"
            }),
        )
        .unwrap();
    assert!(overview["resource"]["content"]["tracks"]
        .as_array()
        .unwrap()
        .is_empty());

    let resources = session.call("resources/list", json!({})).unwrap();
    let listed_resources = resources
        .get("resources")
        .and_then(|value| value.as_array())
        .unwrap();
    assert!(listed_resources.iter().all(|value| {
        value.get("uri").and_then(|uri| uri.as_str()) != Some("emberflow://tracks/track-001/record")
    }));

    let archived_record = session
        .call(
            "read-resource",
            json!({
                "uri": "emberflow://tracks/track-001/record"
            }),
        )
        .unwrap();
    assert_eq!(
        archived_record["resource"]["content"]["status"].as_str(),
        Some("archived")
    );
}

// @minter:integration missing-workspace-root-fails-explicitly emberflow-unavailable-never-produces-guessed-canonical-state
#[test]
fn client_contract_reports_missing_workspace_root_explicitly() {
    let broken = tempdir().unwrap();
    let missing = broken.path().join("missing-repo");
    let error = start_stdio_server(StdioTransportConfig {
        cwd: Some(missing),
        workspace_root: None,
        state_path: None,
    })
    .unwrap_err();

    assert_eq!(error.source, "workspace-resolution");
    assert!(error.message.contains("workspace"));
    assert!(error.message.contains("EmberFlow"));
}

#[test]
fn client_contract_hides_archived_tracks_from_active_discovery_but_keeps_direct_reads() {
    let tmp = tempdir().unwrap();
    let surface = EmberFlowSurface::from_workspace_root(tmp.path()).unwrap();

    surface
        .create_track("track-active", "Active track", "in-progress")
        .unwrap();
    surface
        .create_track("track-archived", "Archived track", "done")
        .unwrap();

    let session = start_stdio_server(StdioTransportConfig {
        cwd: Some(tmp.path().to_path_buf()),
        workspace_root: None,
        state_path: None,
    })
    .unwrap();

    let resources = session.call("resources/list", json!({})).unwrap();
    let listed_resources = resources
        .get("resources")
        .and_then(|value| value.as_array())
        .unwrap();
    assert!(listed_resources.iter().any(|value| {
        value.get("uri").and_then(|uri| uri.as_str())
            == Some("emberflow://tracks/track-active/record")
    }));
    assert!(listed_resources.iter().all(|value| {
        value.get("uri").and_then(|uri| uri.as_str())
            != Some("emberflow://tracks/track-archived/record")
    }));

    let overview = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://workspace/overview"}),
        )
        .unwrap();
    let overview_tracks = overview
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("tracks"))
        .and_then(|value| value.as_array())
        .unwrap();
    assert_eq!(overview_tracks.len(), 1);
    assert_eq!(
        overview_tracks[0]
            .get("trackId")
            .and_then(|value| value.as_str()),
        Some("track-active")
    );

    let archived = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-archived/record"}),
        )
        .unwrap();
    assert_eq!(
        archived
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("archived")
    );
}
