use emberflow::runtime::store::{
    RuntimeStore, TaskInput, TrackBriefSectionInput, TrackMetadataInput, TrackPlanItemInput,
    TrackPlanPhaseInput,
};
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

// @minter:unit create-track-with-durable-status
#[test]
fn create_track_with_durable_status() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();

    let track = store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();

    assert_eq!(track.id, "track-001");
    assert_eq!(track.title, "Build EmberFlow V1");
    assert_eq!(track.status, "planning");
    assert!(!track.created_at.is_empty());
    assert!(!track.updated_at.is_empty());
}

// @minter:unit create-task-with-runtime-state create-task-without-visibility-metadata
#[test]
fn create_task_with_runtime_state() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();

    let task = store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Persist the first event",
            "queued",
            "planning",
        ))
        .unwrap();

    assert_eq!(task.id, "task-001");
    assert_eq!(task.track_id.as_deref(), Some("track-001"));
    assert_eq!(task.title, "Persist the first event");
    assert_eq!(task.status, "queued");
    assert_eq!(task.phase, "planning");
    assert_eq!(task.executor, None);
    assert_eq!(task.execution, None);
    assert_eq!(task.intent_summary, None);
}

// @minter:unit create-task-with-visibility-context
#[test]
fn create_task_with_visibility_context() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();

    let task = store
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

    let reopened = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    let persisted = reopened.get_task("task-visibility").unwrap();

    assert_eq!(persisted.executor.as_deref(), Some("assistant"));
    assert_eq!(persisted.agent_instance_id.as_deref(), Some("claude-1"));
    assert_eq!(persisted.execution.as_deref(), Some("interactive session"));
    assert_eq!(
        persisted.intent_summary.as_deref(),
        Some("Investigate the transport contract end to end")
    );
}

// @minter:unit create-plan-review-task-with-need-input
#[test]
fn create_plan_review_task_with_need_input() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();

    let task = store
        .create_task(TaskInput {
            task_id: "task-plan-review".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Review and approve the execution plan".to_string(),
            status: "need-input".to_string(),
            phase: "planning".to_string(),
            executor: Some("assistant".to_string()),
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Review the plan before implementation".to_string()),
        })
        .unwrap();

    assert_eq!(task.status, "need-input");
    assert_eq!(task.phase, "planning");
    assert_eq!(
        task.intent_summary.as_deref(),
        Some("Review the plan before implementation")
    );
}

// @minter:unit standalone-task-does-not-require-track create-task-without-visibility-metadata
#[test]
fn standalone_task_does_not_require_track() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();

    let task = store
        .create_task(task_input(
            "task-standalone",
            None,
            "Inspect recent events",
            "running",
            "verifying",
        ))
        .unwrap();

    assert_eq!(task.id, "task-standalone");
    assert_eq!(task.track_id, None);
    assert_eq!(task.title, "Inspect recent events");
    assert_eq!(task.status, "running");
    assert_eq!(task.phase, "verifying");
    assert_eq!(task.executor, None);
    assert_eq!(task.execution, None);
    assert_eq!(task.intent_summary, None);
}

// @minter:unit record-event-with-track-and-task-context
#[test]
fn record_event_with_track_and_task_context() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Persist the first event",
            "running",
            "implementing",
        ))
        .unwrap();

    let event = store
        .record_event(
            "event-001",
            Some("track-001"),
            Some("task-001"),
            "progress",
            serde_json::json!({"summary": "Writing the first event"}),
        )
        .unwrap();

    assert_eq!(event.id, "event-001");
    assert_eq!(event.track_id.as_deref(), Some("track-001"));
    assert_eq!(event.task_id.as_deref(), Some("task-001"));
    assert_eq!(event.kind, "progress");
    assert_eq!(event.payload["summary"], "Writing the first event");
}

// @minter:unit update-task-runtime-state
#[test]
fn update_task_runtime_state() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Review the execution plan",
            "queued",
            "planning",
        ))
        .unwrap();

    let update = emberflow::runtime::store::TaskStateUpdate {
        status: Some("need-input"),
        phase: Some("planning"),
        track_id: Some("track-001"),
        executor: Some("assistant"),
        agent_instance_id: None,
        execution: Some("interactive session"),
        intent_summary: Some("Review the plan before implementation"),
    };
    let updated = store.update_task_state("task-001", update).unwrap();

    assert_eq!(updated.status, "need-input");
    assert_eq!(updated.phase, "planning");
    assert_eq!(
        updated.intent_summary.as_deref(),
        Some("Review the plan before implementation")
    );
    assert_eq!(store.get_track("track-001").unwrap().status, "planning");
}

// @minter:unit delete-track-removes-associated-runtime-state
#[test]
fn delete_track_removes_associated_runtime_state() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Persist the first event",
            "running",
            "implementing",
        ))
        .unwrap();
    store
        .record_event(
            "event-001",
            Some("track-001"),
            Some("task-001"),
            "progress",
            serde_json::json!({"summary": "Writing the first event"}),
        )
        .unwrap();

    assert_eq!(store.list_tasks().unwrap().len(), 1);
    assert_eq!(store.list_events(None, None, None).unwrap().len(), 1);

    let deleted = store.delete_track("track-001").unwrap();
    assert_eq!(deleted.id, "track-001");

    assert!(store.get_track("track-001").is_err());
    assert!(store.get_task("task-001").is_err());
    assert!(store.get_event("event-001").is_err());
    assert!(store.list_tasks().unwrap().is_empty());
    assert!(store.list_events(None, None, None).unwrap().is_empty());
}

// @minter:unit store-multiple-projections-for-one-event
#[test]
fn store_multiple_projections_for_one_event() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Persist the first event",
            "running",
            "implementing",
        ))
        .unwrap();
    let event = store
        .record_event(
            "event-001",
            Some("track-001"),
            Some("task-001"),
            "progress",
            serde_json::json!({"summary": "Writing the first event"}),
        )
        .unwrap();

    store
        .record_projection(
            &event.id,
            "user",
            None,
            serde_json::json!({"format": "plain-text"}),
        )
        .unwrap();
    store
        .record_projection(
            &event.id,
            "runtime",
            Some(".emberflow/context/status.md"),
            serde_json::json!({"line": "phase: implementing | status: running | details: Writing the first event"}),
        )
        .unwrap();

    let projections = store.list_projections(Some(&event.id), None).unwrap();
    let kinds: std::collections::BTreeSet<_> = projections
        .into_iter()
        .map(|projection| projection.projection_kind)
        .collect();
    assert_eq!(
        kinds,
        ["runtime", "user"].into_iter().map(String::from).collect()
    );
}

// @minter:unit invalid-status-is-rejected
#[test]
fn invalid_status_is_rejected() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();

    let err = store
        .create_track("track-invalid", "Broken track", "paused")
        .unwrap_err();
    assert!(err.to_string().contains("unsupported"));
}

// @minter:unit invalid-task-status-is-rejected
#[test]
fn invalid_task_status_is_rejected() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Build EmberFlow V1", "planning")
        .unwrap();

    let err = store
        .create_task(task_input(
            "task-invalid",
            Some("track-001"),
            "Broken task",
            "planning",
            "planning",
        ))
        .unwrap_err();

    assert!(err.to_string().contains("unsupported"));
}

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

// @minter:unit canonical-track-persists-resume-metadata
#[test]
fn canonical_track_persists_resume_metadata() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();

    let track = store
        .upsert_track_metadata(canonical_track_metadata_input(
            "emberflow-runtime-split-20260402",
        ))
        .unwrap();

    assert_eq!(track.track_id, "emberflow-runtime-split-20260402");
    assert_eq!(track.track_type, "feature");
    assert_eq!(track.status, "in-progress");
    assert_eq!(
        track.description,
        "Build EmberFlow runtime with track projection engine"
    );
    assert_eq!(track.branch, "feature/runtime-v1");
    assert_eq!(
        track.spec_ref.as_deref(),
        Some("emberflow/specs/emberflow-v1.spec")
    );

    let reread = store
        .get_track_metadata("emberflow-runtime-split-20260402")
        .unwrap();
    assert_eq!(reread, track);
}

// @minter:integration canonical-track-imports-existing-track-directory
#[test]
fn canonical_track_imports_existing_track_directory() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    let track_id = "emberflow-runtime-split-20260402";

    let metadata = store
        .upsert_track_metadata(canonical_track_metadata_input(track_id))
        .unwrap();
    let brief = store
        .replace_track_brief(track_id, brief_sections())
        .unwrap();
    let plan = store
        .replace_track_plan(track_id, canonical_plan())
        .unwrap();

    let reread_metadata = store.get_track_metadata(track_id).unwrap();
    let reread_brief = store.get_track_brief(track_id).unwrap();
    let reread_plan = store.get_track_plan(track_id).unwrap();

    assert_eq!(metadata, reread_metadata);
    assert_eq!(brief, reread_brief);
    assert_eq!(plan, reread_plan);
    assert_eq!(reread_plan.phases.len(), 2);
    assert!(!reread_brief.sections.is_empty());
}

// @minter:unit canonical-track-stores-ordered-brief-sections
#[test]
fn canonical_track_stores_ordered_brief_sections() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .upsert_track_metadata(canonical_track_metadata_input(
            "emberflow-runtime-split-20260402",
        ))
        .unwrap();

    let sections = brief_sections();
    let brief = store
        .replace_track_brief(
            "emberflow-runtime-split-20260402",
            vec![
                sections[3].clone(),
                sections[0].clone(),
                sections[1].clone(),
            ],
        )
        .unwrap();

    let ordered_keys: Vec<_> = brief
        .sections
        .iter()
        .map(|section| section.section_key.as_str())
        .collect();
    assert_eq!(ordered_keys, vec!["objective", "context", "next_step"]);

    let reread = store
        .get_track_brief("emberflow-runtime-split-20260402")
        .unwrap();
    let reread_keys: Vec<_> = reread
        .sections
        .iter()
        .map(|section| section.section_key.as_str())
        .collect();
    assert_eq!(reread_keys, vec!["objective", "context", "next_step"]);
}

// @minter:unit canonical-track-stores-ordered-plan-structure
#[test]
fn canonical_track_stores_ordered_plan_structure() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .upsert_track_metadata(canonical_track_metadata_input(
            "emberflow-runtime-split-20260402",
        ))
        .unwrap();

    let plan = store
        .replace_track_plan("emberflow-runtime-split-20260402", canonical_plan())
        .unwrap();

    let phase_keys: Vec<_> = plan
        .phases
        .iter()
        .map(|phase| phase.phase_id.as_str())
        .collect();
    assert_eq!(phase_keys, vec!["phase-1", "phase-2"]);

    let phase_one_item_keys: Vec<_> = plan.phases[0]
        .items
        .iter()
        .map(|item| item.item_id.as_str())
        .collect();
    assert_eq!(
        phase_one_item_keys,
        vec!["phase-1/item-1", "phase-1/item-2"]
    );

    let reread = store
        .get_track_plan("emberflow-runtime-split-20260402")
        .unwrap();
    assert_eq!(reread.phases.len(), 2);
    assert_eq!(reread.phases[1].items[0].item_id, "phase-2/item-1");
}

// @minter:unit canonical-plan-items-remain-distinct-from-runtime-tasks
#[test]
fn canonical_plan_items_remain_distinct_from_runtime_tasks() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .upsert_track_metadata(canonical_track_metadata_input(
            "emberflow-runtime-split-20260402",
        ))
        .unwrap();
    store
        .replace_track_plan("emberflow-runtime-split-20260402", canonical_plan())
        .unwrap();

    let task = store
        .create_task_for_plan_item(
            task_input(
                "runtime-task-001",
                Some("emberflow-runtime-split-20260402"),
                "Implement the canonical metadata table",
                "running",
                "implementing",
            ),
            "phase-1/item-1",
        )
        .unwrap();

    assert_eq!(task.id, "runtime-task-001");
    assert_eq!(
        task.track_id.as_deref(),
        Some("emberflow-runtime-split-20260402")
    );
    assert_eq!(task.plan_item_id.as_deref(), Some("phase-1/item-1"));

    let reread_plan = store
        .get_track_plan("emberflow-runtime-split-20260402")
        .unwrap();
    assert_eq!(reread_plan.phases[0].items[0].item_id, "phase-1/item-1");

    let update = emberflow::runtime::store::TaskStateUpdate {
        status: Some("awaiting-review"),
        phase: Some("verifying"),
        track_id: Some("emberflow-runtime-split-20260402"),
        executor: None,
        agent_instance_id: None,
        execution: None,
        intent_summary: None,
    };
    let updated = store.update_task_state("runtime-task-001", update).unwrap();
    assert_eq!(updated.plan_item_id.as_deref(), Some("phase-1/item-1"));
}

// @minter:unit canonical-track-rejects-plan-item-without-stable-placement
#[test]
fn canonical_track_rejects_plan_item_without_stable_placement() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .upsert_track_metadata(canonical_track_metadata_input(
            "emberflow-runtime-split-20260402",
        ))
        .unwrap();

    let err = store
        .replace_track_plan(
            "emberflow-runtime-split-20260402",
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

    assert!(err.to_string().contains("item"));
    assert!(err.to_string().contains("stable"));
}

// @minter:unit claim-task-acquires-exclusive-lease
#[test]
fn claim_task_acquires_exclusive_lease() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    let lease = store.claim_task("task-001", "agent-a", Some(300)).unwrap();

    assert_eq!(lease.holder, "agent-a");
    assert!(!lease.acquired_at.is_empty());
    assert!(lease.expires_at.is_some());
}

// @minter:unit same-holder-reclaim-refreshes-lease
#[test]
fn same_holder_reclaim_refreshes_lease() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    store.claim_task("task-001", "agent-a", Some(60)).unwrap();
    let refreshed = store.claim_task("task-001", "agent-a", Some(600)).unwrap();

    assert_eq!(refreshed.holder, "agent-a");
    assert!(refreshed.expires_at.is_some());
}

// @minter:unit different-holder-claim-rejected
#[test]
fn different_holder_claim_rejected() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    store.claim_task("task-001", "agent-a", Some(300)).unwrap();
    let err = store
        .claim_task("task-001", "agent-b", Some(300))
        .unwrap_err();

    assert!(err.to_string().contains("already held"), "error was: {err}");
}

// @minter:unit release-clears-lease
#[test]
fn release_clears_lease() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    store.claim_task("task-001", "agent-a", Some(300)).unwrap();
    store.release_task("task-001", "agent-a").unwrap();
    let active = store.check_lease("task-001").unwrap();

    assert!(active.is_none());
}

// @minter:unit release-by-wrong-holder-fails
#[test]
fn release_by_wrong_holder_fails() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    store.claim_task("task-001", "agent-a", Some(300)).unwrap();
    let err = store.release_task("task-001", "agent-b").unwrap_err();

    assert!(
        err.to_string().contains("not the lease holder"),
        "error was: {err}"
    );
}

// @minter:unit expired-lease-auto-cleared-on-access
#[test]
fn expired_lease_auto_cleared_on_access() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task",
            "queued",
            "planning",
        ))
        .unwrap();

    // Insert an already-expired lease directly via store helper
    store
        .claim_task_with_expiry("task-001", "agent-a", "2020-01-01T00:00:00Z")
        .unwrap();

    let active = store.check_lease("task-001").unwrap();
    assert!(active.is_none(), "expired lease should be auto-cleared");
}

// @minter:unit expire-stale-leases-bulk-cleanup
#[test]
fn expire_stale_leases_bulk_cleanup() {
    let tmp = tempdir().unwrap();
    let store = RuntimeStore::new(tmp.path().join("emberflow.db")).unwrap();
    store
        .create_track("track-001", "Test track", "planning")
        .unwrap();
    store
        .create_task(task_input(
            "task-001",
            Some("track-001"),
            "Test task A",
            "queued",
            "planning",
        ))
        .unwrap();
    store
        .create_task(task_input(
            "task-002",
            Some("track-001"),
            "Test task B",
            "queued",
            "planning",
        ))
        .unwrap();

    store
        .claim_task_with_expiry("task-001", "agent-a", "2020-01-01T00:00:00Z")
        .unwrap();
    store
        .claim_task_with_expiry("task-002", "agent-b", "2020-01-01T00:00:00Z")
        .unwrap();

    let cleared = store.expire_stale_leases().unwrap();
    assert_eq!(cleared, 2);

    assert!(store.check_lease("task-001").unwrap().is_none());
    assert!(store.check_lease("task-002").unwrap().is_none());
}
