use emberflow::runtime::service::{
    EmberFlowRuntime, TaskInput, TrackBriefSectionInput, TrackMetadataInput, TrackPlanItemInput,
    TrackPlanPhaseInput,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn projected_runtime() -> (tempfile::TempDir, EmberFlowRuntime) {
    let tmp = tempdir().unwrap();
    std::fs::write(
        tmp.path().join("emberflow.config.json"),
        r#"{"mode":"projected"}"#,
    )
    .unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();
    (tmp, runtime)
}

fn canonical_track_metadata_input(track_id: &str, description: &str) -> TrackMetadataInput {
    TrackMetadataInput {
        track_id: track_id.to_string(),
        track_type: "feature".to_string(),
        status: "in-progress".to_string(),
        description: description.to_string(),
        branch: "feature/runtime-v1".to_string(),
        spec_ref: Some("emberflow/specs/emberflow-v1.spec".to_string()),
    }
}

fn brief_sections() -> Vec<TrackBriefSectionInput> {
    vec![
        TrackBriefSectionInput {
            section_key: "objective".to_string(),
            section_text: "Persist projected track state".to_string(),
            position: 0,
        },
        TrackBriefSectionInput {
            section_key: "context".to_string(),
            section_text: "Keep SQLite canonical and project filesystem views from it".to_string(),
            position: 1,
        },
    ]
}

fn canonical_plan() -> Vec<TrackPlanPhaseInput> {
    vec![TrackPlanPhaseInput {
        phase_id: "phase-1".to_string(),
        title: "Projection persistence".to_string(),
        position: 0,
        items: vec![TrackPlanItemInput {
            item_id: "phase-1/item-1".to_string(),
            title: "Mark projected files dirty".to_string(),
            position: Some(0),
        }],
    }]
}

fn projected_root(workspace: &tempfile::TempDir) -> PathBuf {
    workspace.path().join(".emberflow")
}

fn projected_path(workspace: &tempfile::TempDir, relative: &str) -> PathBuf {
    workspace.path().join(relative)
}

fn write_text(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn assert_no_temp_artifacts(root: &Path) {
    fn walk(path: &Path) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            assert!(
                !name.contains(".tmp"),
                "unexpected temporary projection artifact: {}",
                entry.path().display()
            );
            let path = entry.path();
            if path.is_dir() {
                walk(&path);
            }
        }
    }

    if root.exists() {
        walk(root);
    }
}

// @minter:integration projected-mode-materializes-track-filesystem-view
#[test]
fn projected_mode_materializes_track_filesystem_view() {
    let (workspace, runtime) = projected_runtime();
    let track_id = "track-001";
    let task_id = "task-001";
    let description = "Build EmberFlow runtime with track projection engine";

    let targets = runtime
        .projected_track_filesystem_targets(track_id)
        .unwrap();

    assert_eq!(targets.mode, "projected");
    assert_eq!(targets.runtime_status_path, ".emberflow/context/status.md");
    assert_eq!(targets.track_list_path, ".emberflow/tracks/tracks.md");
    assert_eq!(targets.track_directory_path, ".emberflow/tracks/track-001/");
    assert_eq!(
        targets.metadata_path,
        ".emberflow/tracks/track-001/metadata.json"
    );
    assert_eq!(targets.brief_path, ".emberflow/tracks/track-001/brief.md");
    assert_eq!(targets.plan_path, ".emberflow/tracks/track-001/plan.md");
    assert_eq!(targets.summary_path, ".emberflow/tracks/track-001/index.md");

    let track = runtime
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    let metadata = runtime
        .upsert_track_metadata(canonical_track_metadata_input(track_id, description))
        .unwrap();
    let brief = runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    let plan = runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();
    let task = runtime
        .create_task(TaskInput {
            task_id: task_id.to_string(),
            track_id: Some(track_id.to_string()),
            title: "Persist the first event".to_string(),
            status: "running".to_string(),
            phase: "implementing".to_string(),
            executor: None,
            agent_instance_id: None,
            execution: None,
            intent_summary: None,
        })
        .unwrap();
    runtime.claim_task(task_id, "assistant", Some(300)).unwrap();
    let runtime_state = runtime
        .record_runtime_state(
            track_id,
            task_id,
            "progress",
            serde_json::json!({
                "summary": "Writing the first event",
                "phase": "implementing",
                "status": "running",
            }),
        )
        .unwrap();

    assert_eq!(track.id, track_id);
    assert_eq!(metadata.description, description);
    assert_eq!(brief.sections.len(), 2);
    assert_eq!(plan.phases.len(), 1);
    assert_eq!(task.id, task_id);
    assert_eq!(runtime_state.store, "canonical");

    let runtime_status_path = projected_path(&workspace, ".emberflow/context/status.md");
    let tracks_path = projected_path(&workspace, ".emberflow/tracks/tracks.md");
    let metadata_path = projected_path(&workspace, ".emberflow/tracks/track-001/metadata.json");
    let brief_path = projected_path(&workspace, ".emberflow/tracks/track-001/brief.md");
    let plan_path = projected_path(&workspace, ".emberflow/tracks/track-001/plan.md");
    let index_path = projected_path(&workspace, ".emberflow/tracks/track-001/index.md");

    assert_eq!(
        fs::read_to_string(&runtime_status_path).unwrap(),
        "phase: implementing | status: running | details: Writing the first event"
    );
    assert!(fs::read_to_string(&tracks_path)
        .unwrap()
        .contains("Build EmberFlow runtime with track projection engine"));
    assert!(fs::read_to_string(&metadata_path)
        .unwrap()
        .contains("\"track_id\":\"track-001\""));
    assert!(fs::read_to_string(&brief_path)
        .unwrap()
        .contains("Keep SQLite canonical and project filesystem views from it"));
    assert!(fs::read_to_string(&plan_path)
        .unwrap()
        .contains("Projection persistence"));
    assert!(fs::read_to_string(&index_path)
        .unwrap()
        .contains("# Track: track-001"));
    assert!(runtime
        .dirty_projection_targets(Some(track_id))
        .unwrap()
        .is_empty());
    assert_no_temp_artifacts(&projected_root(&workspace));
}

// @minter:integration successful-projection-refresh-clears-dirty-targets
#[test]
fn successful_projection_refresh_clears_dirty_targets() {
    let (workspace, runtime) = projected_runtime();
    let track_id = "track-001";
    let original_description = "Build EmberFlow runtime with track projection engine";

    runtime
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            original_description,
        ))
        .unwrap();
    runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    assert!(runtime
        .dirty_projection_targets(Some(track_id))
        .unwrap()
        .is_empty());
    assert!(fs::read_to_string(projected_path(
        &workspace,
        ".emberflow/tracks/track-001/index.md"
    ))
    .unwrap()
    .contains("# Track: track-001"));
    assert!(
        fs::read_to_string(projected_path(&workspace, ".emberflow/tracks/tracks.md"))
            .unwrap()
            .contains(original_description)
    );
    assert_no_temp_artifacts(&projected_root(&workspace));
}

// @minter:integration canonical-write-attempts-immediate-projection-refresh
#[test]
fn canonical_write_attempts_immediate_projection_refresh() {
    let (workspace, runtime) = projected_runtime();
    let track_id = "track-001";
    let updated_description = "Build EmberFlow runtime and integrate canonical projections";

    runtime
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            "Build EmberFlow runtime with track projection engine",
        ))
        .unwrap();
    runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    let tracks_path = projected_path(&workspace, ".emberflow/tracks/tracks.md");
    write_text(&tracks_path, "stale projection");

    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            updated_description,
        ))
        .unwrap();

    let dirty = runtime.dirty_projection_targets(Some(track_id)).unwrap();
    assert!(dirty.is_empty());
    let tracks_md = fs::read_to_string(&tracks_path).unwrap();
    assert!(tracks_md.contains(updated_description));
    assert!(!tracks_md.contains("stale projection"));
    assert!(fs::read_to_string(projected_path(
        &workspace,
        ".emberflow/tracks/track-001/metadata.json"
    ))
    .unwrap()
    .contains(updated_description));
    assert!(fs::read_to_string(projected_path(
        &workspace,
        ".emberflow/tracks/track-001/index.md"
    ))
    .unwrap()
    .contains(updated_description));
    assert_no_temp_artifacts(&projected_root(&workspace));
}

// @minter:integration projected-filesystem-views-are-rendered-from-canonical-sqlite-state
#[test]
fn projected_filesystem_views_are_rendered_from_canonical_sqlite_state() {
    let (workspace, runtime) = projected_runtime();
    let track_id = "track-001";

    runtime
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            "Build EmberFlow runtime with track projection engine",
        ))
        .unwrap();
    runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    let metadata_path = projected_path(&workspace, ".emberflow/tracks/track-001/metadata.json");
    write_text(&metadata_path, r#"{"stale":true}"#);
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            "Build EmberFlow runtime from canonical SQLite state",
        ))
        .unwrap();

    let metadata_json = fs::read_to_string(&metadata_path).unwrap();
    assert!(metadata_json.contains("Build EmberFlow runtime from canonical SQLite state"));
    assert!(!metadata_json.contains("stale"));
    assert!(fs::read_to_string(projected_path(
        &workspace,
        ".emberflow/tracks/track-001/index.md"
    ))
    .unwrap()
    .contains("Build EmberFlow runtime from canonical SQLite state"));
}

// @minter:integration projected-filesystem-refresh-is-atomic
#[test]
fn projected_filesystem_refresh_is_atomic() {
    let (workspace, runtime) = projected_runtime();
    let track_id = "track-001";

    runtime
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            "Build EmberFlow runtime with track projection engine",
        ))
        .unwrap();
    runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    let brief_path = projected_path(&workspace, ".emberflow/tracks/track-001/brief.md");
    let original = fs::read_to_string(&brief_path).unwrap();
    assert!(original.contains("Persist projected track state"));

    runtime
        .replace_track_brief(
            track_id,
            vec![TrackBriefSectionInput {
                section_key: "objective".to_string(),
                section_text: "Write projection files atomically".to_string(),
                position: 0,
            }],
        )
        .unwrap();

    let updated = fs::read_to_string(&brief_path).unwrap();
    assert!(updated.contains("Write projection files atomically"));
    assert!(!updated.contains("Persist projected track state"));
    assert_no_temp_artifacts(&projected_root(&workspace));
}

// @minter:integration projection-failure-preserves-canonical-write
#[test]
fn projection_failure_preserves_canonical_write() {
    let (workspace, runtime) = projected_runtime();
    let track_id = "track-001";
    let projection_dir = projected_path(&workspace, ".emberflow/tracks");
    let updated_description = "Build EmberFlow runtime and preserve SQLite on failure";

    runtime
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            "Build EmberFlow runtime with track projection engine",
        ))
        .unwrap();
    runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    fs::remove_dir_all(&projection_dir).unwrap();
    write_text(&projection_dir, "blocked");
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            updated_description,
        ))
        .unwrap();
    let failure = runtime.refresh_dirty_projection_targets().unwrap_err();

    assert!(failure.to_string().contains("filesystem"));
    let context = runtime.store.get_track_metadata(track_id).unwrap();
    assert_eq!(context.description, updated_description);
    let dirty = runtime.dirty_projection_targets(Some(track_id)).unwrap();
    let dirty_paths: BTreeSet<_> = dirty.into_iter().map(|target| target.target_path).collect();
    assert!(!dirty_paths.is_empty());
}

// @minter:integration dirty-projections-retry-on-later-access
#[test]
fn dirty_projections_retry_on_later_access() {
    let (workspace, runtime) = projected_runtime();
    let track_id = "track-001";
    let projection_dir = projected_path(&workspace, ".emberflow/tracks");

    runtime
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            "Build EmberFlow runtime with track projection engine",
        ))
        .unwrap();
    runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    fs::remove_dir_all(&projection_dir).unwrap();
    write_text(&projection_dir, "blocked");
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            "Build EmberFlow runtime and retry later",
        ))
        .unwrap();
    assert!(runtime.refresh_dirty_projection_targets().is_err());
    let before = runtime.dirty_projection_targets(Some(track_id)).unwrap();
    assert!(!before.is_empty());

    fs::remove_file(&projection_dir).unwrap();
    runtime.load_track_context(track_id).unwrap();

    let after = runtime.dirty_projection_targets(Some(track_id)).unwrap();
    assert!(after.is_empty());
    assert!(fs::read_to_string(projected_path(
        &workspace,
        ".emberflow/tracks/track-001/metadata.json"
    ))
    .unwrap()
    .contains("Build EmberFlow runtime and retry later"));
}

// @minter:integration canonical-mode-does-not-materialize-projected-files
#[test]
fn canonical_mode_does_not_materialize_projected_files() {
    let tmp = tempdir().unwrap();
    let runtime = EmberFlowRuntime::from_workspace_root(tmp.path()).unwrap();
    let track_id = "track-001";

    runtime
        .create_track(track_id, "Build EmberFlow V1", "in-progress")
        .unwrap();
    runtime
        .upsert_track_metadata(canonical_track_metadata_input(
            track_id,
            "Build EmberFlow runtime with track projection engine",
        ))
        .unwrap();
    runtime
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    runtime
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    assert!(
        !projected_path(&tmp, ".emberflow/context/status.md").exists(),
        "canonical mode should not materialize status.md"
    );
    assert!(
        !projected_path(&tmp, ".emberflow/tracks").exists(),
        "canonical mode should not materialize track projections"
    );

    let dirty = runtime.dirty_projection_targets(Some(track_id)).unwrap();
    assert!(dirty.is_empty());
}
