spec emberflow-projection-engine v0.4.0
title "EmberFlow projection engine"

description
  Defines how EmberFlow derives human-readable and runtime projections from
  canonical protocol events without turning projections into the canonical
  record.

motivation
  EmberFlow must project protocol events to the user and to Conductor-compatible
  runtime status under `.emberflow/context/` while keeping the canonical event
  stream in the runtime store. In projected mode, EmberFlow must also derive
  human-readable filesystem views under `.emberflow/tracks/` without allowing
  those views to become the source of truth.

behavior progress-projects-to-plain-text-chat [happy_path]
  "When a progress event is projected to the user, EmberFlow renders plain discussion text instead of a shell command"

  given
    A progress event has been recorded
    @event = Event {{ id: "event-001", kind: "progress" }}

  when project-user-view
    eventId = @event.id

  then returns userProjection
    assert eventId == "event-001"
    assert kind == "progress"
    assert format == "plain-text"
    assert summary is_present

behavior progress-projects-to-runtime-status-line [happy_path]
  "When a progress event is projected to EmberFlow runtime state, EmberFlow renders the canonical single-line status record under `.emberflow/context/`"

  given
    A progress event has been recorded
    @event = Event {{ id: "event-001", kind: "progress" }}

  when project-runtime-view
    eventId = @event.id

  then returns runtimeProjection
    assert eventId == "event-001"
    assert targetPath == ".emberflow/context/status.md"
    assert lineFormat == "phase: ... | status: ... | details: ..."

behavior progress-runtime-inherits-canonical-task-state [edge_case]
  "When a progress event omits status and phase overrides, EmberFlow projects runtime state from the canonical task instead of inventing contradictory execution state"

  given
    A canonical task already exists in plan review state
    @task = Task {{ id: "task-plan-review", status: "need-input", phase: "planning" }}
    @event = Event {{ id: "event-001", kind: "progress", taskId: @task.id }}

  when project-runtime-view
    eventId = @event.id

  then returns runtimeProjection
    assert eventId == "event-001"
    assert status == "need-input"
    assert phase == "planning"
    assert lineFormat == "phase: ... | status: ... | details: ..."

behavior progress-runtime-respects-explicit-overrides [happy_path]
  "When a progress event includes explicit status or phase overrides, EmberFlow projects those explicit runtime values instead of the canonical task defaults"

  given
    A canonical task already exists in plan review state
    @task = Task {{ id: "task-plan-review", status: "need-input", phase: "planning" }}
    @event = Event {{ id: "event-002", kind: "progress", taskId: @task.id }}

  when project-runtime-view
    eventId = @event.id
    status = "running"
    phase = "implementing"

  then returns runtimeProjection
    assert eventId == "event-002"
    assert status == "running"
    assert phase == "implementing"

behavior transient-progress-does-not-mutate-track [edge_case]
  "When a transient progress event is projected, EmberFlow leaves durable track state unchanged"

  given
    A track is already in durable execution
    @track = Track {{ id: "track-001", status: "in-progress" }}
    @event = Event {{ id: "event-001", kind: "progress" }}

  when project-track-view
    trackId = @track.id
    eventId = @event.id

  then returns trackProjection
    assert trackId == "track-001"
    assert eventId == "event-001"
    assert durableChange == "none"

  then side_effect
    assert The track remains in-progress because progress is runtime chatter, not durable progress

behavior blocker-projects-durable-track-state [error_case]
  "When a blocker event prevents the workflow from continuing, EmberFlow projects the durable track state to blocked"

  given
    A track is actively running
    @track = Track {{ id: "track-001", status: "in-progress" }}
    @event = Event {{ id: "event-002", kind: "blocker" }}

  when project-track-view
    trackId = @track.id
    eventId = @event.id

  then returns trackProjection
    assert trackId == "track-001"
    assert status == "blocked"
    assert summary is_present

behavior handoff-projects-track-to-review [happy_path]
  "When a handoff event marks work ready for validation, EmberFlow projects the durable track state to review"

  given
    A track has reached a reviewable handoff point
    @track = Track {{ id: "track-001", status: "in-progress" }}
    @event = Event {{ id: "event-003", kind: "handoff" }}

  when project-track-view
    trackId = @track.id
    eventId = @event.id

  then returns trackProjection
    assert trackId == "track-001"
    assert status == "review"
    assert summary is_present

behavior projected-mode-materializes-track-filesystem-view [happy_path]
  "When EmberFlow runs in projected mode, it materializes readable filesystem views for runtime status and durable track context under `.emberflow/`"

  given
    EmberFlow has been configured in projected mode
    @track = Track {{ id: "track-001" }}

  when project-filesystem-view
    mode = "projected"
    trackId = @track.id

  then returns projectionTargets
    assert mode == "projected"
    assert runtimeStatusPath == ".emberflow/context/status.md"
    assert trackIndexPath == ".emberflow/tracks/tracks.md"
    assert trackDirectoryPath == ".emberflow/tracks/track-001/"
    assert metadataPath == ".emberflow/tracks/track-001/metadata.json"
    assert briefPath == ".emberflow/tracks/track-001/brief.md"
    assert planPath == ".emberflow/tracks/track-001/plan.md"
    assert summaryPath == ".emberflow/tracks/track-001/index.md"

behavior projected-filesystem-views-are-rendered-from-canonical-sqlite-state [happy_path]
  "When EmberFlow rewrites projected files, it renders their contents from canonical SQLite state instead of from stale files or in-memory cache"

  given
    EmberFlow has canonical track metadata, brief, plan, and runtime state in projected mode
    @track = Track {{ id: "track-001" }}

  when project-filesystem-view
    mode = "projected"
    trackId = @track.id

  then returns renderedFilesystemView
    assert runtimeStatusPath == ".emberflow/context/status.md"
    assert trackIndexPath == ".emberflow/tracks/tracks.md"
    assert metadataPath == ".emberflow/tracks/track-001/metadata.json"
    assert briefPath == ".emberflow/tracks/track-001/brief.md"
    assert planPath == ".emberflow/tracks/track-001/plan.md"
    assert summaryPath == ".emberflow/tracks/track-001/index.md"
    assert contents are_present

behavior canonical-write-attempts-immediate-projection-refresh [happy_path]
  "When canonical track state changes in projected mode, EmberFlow records the affected filesystem targets and immediately attempts to refresh them without rebuilding unrelated projections"

  given
    EmberFlow is storing track context canonically in projected mode
    @track = Track {{ id: "track-001" }}

  when store-canonical-track-context
    mode = "projected"
    trackId = @track.id

  then returns refreshAttempt
    assert mode == "projected"
    assert trackId == "track-001"
    assert refresh == "attempted-immediately"
    assert projectedFiles is_present

behavior projected-filesystem-refresh-is-atomic [happy_path]
  "When EmberFlow rewrites projected files, it replaces each target atomically so no partial projection file is exposed"

  given
    EmberFlow is refreshing projected filesystem views
    @track = Track {{ id: "track-001" }}

  when refresh-projected-filesystem-view
    mode = "projected"
    trackId = @track.id

  then returns atomicRefresh
    assert mode == "projected"
    assert trackId == "track-001"
    assert atomicWrite == "temp-file-then-rename"
    assert partialArtifacts == "none"

behavior successful-projection-refresh-clears-dirty-targets [happy_path]
  "When a projected filesystem refresh succeeds, EmberFlow clears the dirty targets that were written successfully"

  given
    EmberFlow has pending dirty projection targets in projected mode
    @track = Track {{ id: "track-001" }}

  when refresh-projected-filesystem-view
    mode = "projected"
    trackId = @track.id
    filesystem = "writable"

  then returns projectionSuccess
    assert mode == "projected"
    assert trackId == "track-001"
    assert dirtyTargetsCleared is_present
    assert dirtyTargetsRemaining == "none"

behavior projection-failure-preserves-canonical-write [error_case]
  "When EmberFlow cannot rewrite projected files after a canonical write, it keeps the canonical state committed and retains the affected projection targets for retry"

  given
    EmberFlow has accepted a canonical track update in projected mode
    @track = Track {{ id: "track-001" }}

  when refresh-projected-filesystem-view
    mode = "projected"
    trackId = @track.id
    filesystem = "unavailable"

  then returns projectionFailure
    assert mode == "projected"
    assert trackId == "track-001"
    assert canonicalWrite == "committed"
    assert failureReason contains "filesystem"
    assert dirtyTargets is_present

behavior dirty-projections-retry-on-later-access [edge_case]
  "When EmberFlow starts, reads, or writes while dirty projection targets remain, it retries those filesystem refreshes automatically without requiring manual materialization"

  given
    EmberFlow has pending dirty projection targets from an earlier filesystem failure
    @track = Track {{ id: "track-001" }}

  when reconcile-dirty-projections
    trigger = "startup-or-read-or-write"
    trackId = @track.id

  then returns reconciliation
    assert trackId == "track-001"
    assert trigger == "startup-or-read-or-write"
    assert reconciliation == "automatic"
    assert manualCommand == "not-required"

behavior canonical-mode-does-not-materialize-projected-files [happy_path]
  "When EmberFlow runs in canonical mode, it keeps SQLite authoritative and does not create projected `.emberflow/context/` or `.emberflow/tracks/` files"

  given
    EmberFlow has been configured in canonical mode
    @track = Track {{ id: "track-001" }}

  when store-canonical-track-context
    mode = "canonical"
    trackId = @track.id

  then returns canonicalWrite
    assert mode == "canonical"
    assert trackId == "track-001"
    assert projectedFilesCreated == "none"
    assert filesystemWrites == "none"

depends on emberflow-project-layout >= 0.1.0
