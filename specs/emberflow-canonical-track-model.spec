spec emberflow-canonical-track-model v0.2.0
title "EmberFlow canonical track model"

description
  Defines the canonical durable track model for metadata, brief sections, and
  plan structure stored in EmberFlow SQLite.

motivation
  EmberFlow must become the single source of truth for track resume context and
  planning before filesystem projections can stop being authoritative.

behavior canonical-track-resolves-project-state-root [happy_path]
  "When a caller resolves EmberFlow project state, the canonical model stores metadata under `.emberflow/` at the resolved project root"

  given
    A caller is opening EmberFlow in a project workspace

  when resolve-project-state-root
    root = "default"

  then returns projectState
    assert dbPath contains ".emberflow/emberflow.db"
    assert tracksPath contains ".emberflow/tracks"
    assert contextPath contains ".emberflow/context"

behavior canonical-track-allows-root-override [happy_path]
  "When a project config provides a root override, EmberFlow resolves canonical state under that override instead of the default base directory"

  given
    A project config exists with a root override

  when resolve-project-state-root
    root = "../shared-state"

  then returns projectState
    assert dbPath contains ".emberflow/emberflow.db"
    assert tracksPath contains ".emberflow/tracks"
    assert contextPath contains ".emberflow/context"

behavior canonical-track-persists-resume-metadata [happy_path]
  "When a caller stores durable track identity in EmberFlow, the canonical model persists the metadata needed to resume and route the track"

  given
    A caller is opening one durable track in the canonical model

  when upsert-track-metadata
    trackId = "emberflow-runtime-split-20260402"
    type = "feature"
    status = "in-progress"
    description = "Build EmberFlow runtime and integrate Blacksmith monorepo split"
    branch = "arnaudlewis/emberflow-v1-impl-v1"
    specRef = "emberflow/specs/emberflow-v1.spec"

  then returns trackMetadata
    assert trackId == "emberflow-runtime-split-20260402"
    assert type == "feature"
    assert status == "in-progress"
    assert description is_present
    assert branch == "arnaudlewis/emberflow-v1-impl-v1"
    assert specRef == "emberflow/specs/emberflow-v1.spec"

behavior canonical-track-stores-ordered-brief-sections [happy_path]
  "When a caller records durable resume context for one track, EmberFlow stores ordered brief sections canonically instead of relying on markdown files"

  given
    A canonical track already exists
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when replace-track-brief
    trackId = @track.id
    sections = "objective,context,decisions,non_goals,current_state,workspace_branch_pr_context,next_step"

  then returns trackBrief
    assert trackId == "emberflow-runtime-split-20260402"
    assert sections is_present
    assert objective is_present
    assert context is_present
    assert nextStep is_present

  then side_effect
    assert Brief section order remains stable when the canonical brief is read back

behavior canonical-track-stores-ordered-plan-structure [happy_path]
  "When a caller records one durable implementation plan, EmberFlow stores plan phases and plan items canonically for that track"

  given
    A canonical track already exists
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when replace-track-plan
    trackId = @track.id
    phases = "phase-1,phase-2,phase-3"

  then returns trackPlan
    assert trackId == "emberflow-runtime-split-20260402"
    assert phases is_present
    assert items is_present

  then side_effect
    assert Phase order and item order remain stable when the canonical plan is read back

behavior canonical-plan-items-remain-distinct-from-runtime-tasks [edge_case]
  "When runtime execution is linked to a planned unit of work, EmberFlow keeps the durable plan item and the runtime task as distinct records"

  given
    A canonical track plan already exists
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}
    @planItem = PlanItem {{ id: "phase-2/task-1" }}

  when create-task
    trackId = @track.id
    taskId = "runtime-task-001"
    planItemId = @planItem.id

  then returns taskRecord
    assert trackId == "emberflow-runtime-split-20260402"
    assert id == "runtime-task-001"
    assert planItemId == "phase-2/task-1"

  then side_effect
    assert Updating runtime task status does not rewrite the durable plan item itself

behavior canonical-track-imports-existing-track-directory [happy_path]
  "When a caller imports an existing filesystem track, EmberFlow captures the metadata, brief, and plan into one canonical track model"

  given
    A filesystem track already exists

  when import-track-directory
    path = ".emberflow/tracks/emberflow-runtime-split-20260402"

  then returns canonicalTrack
    assert trackId == "emberflow-runtime-split-20260402"
    assert metadata is_present
    assert brief is_present
    assert plan is_present

behavior canonical-track-rejects-plan-item-without-stable-placement [error_case]
  "When a caller stores a plan item without a stable phase or ordering position, EmberFlow rejects the write instead of accepting ambiguous plan structure"

  given
    A caller submits an incomplete canonical plan item

  when replace-track-plan
    trackId = "emberflow-runtime-split-20260402"
    phases = "phase-1"
    item = "task-without-position"

  then returns validationError
    assert field contains "item"
    assert reason contains "stable"

depends on emberflow-runtime-store >= 0.1.0
depends on emberflow-project-layout >= 0.1.0
