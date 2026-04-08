spec emberflow-v1 v0.4.0
title "EmberFlow V1 runtime"

description
  Defines the first executable EmberFlow capability: a canonical runtime for
  protocol-backed work state, portable task visibility, shared project-root
  state resolution, derived projections under `.emberflow/`, and a minimal MCP
  surface.

motivation
  EmberFlow V1 should establish a clean runtime boundary below Runestone before
  any toolkit integration or workflow migration happens, including a generic
  visibility layer for who executes work, how it is being executed, and what
  will be done.

behavior emberflow-v1-persists-canonical-track-context [happy_path]
  "When a client needs durable track resume context in EmberFlow V1, the runtime stores the track metadata, brief, and plan canonically instead of relying on filesystem artifacts"

  given
    A client is preparing one durable track in EmberFlow
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when store-canonical-track-context
    trackId = @track.id

  then returns trackContext
    assert trackId == "emberflow-runtime-split-20260402"
    assert metadata is_present
    assert brief is_present
    assert plan is_present

  then side_effect
    assert Durable track context remains readable without treating `.emberflow/tracks/` as canonical

behavior emberflow-v1-resolves-shared-project-db-path [happy_path]
  "When EmberFlow V1 starts inside a git-backed workspace, it resolves the canonical database path from the shared project root rather than the current worktree root"

  given
    A client is opening EmberFlow inside a git-backed project workspace

  when initialize-workspace-state
    workspaceKind = "git"

  then returns workspaceState
    assert defaultPath contains ".emberflow/emberflow.db"
    assert stateRoot contains ".emberflow"
    assert projectionMode == "canonical"

behavior emberflow-v1-falls-back-to-local-root-without-git [edge_case]
  "When EmberFlow V1 starts without git metadata and no root override is configured, it falls back to the local project directory for `.emberflow/` resolution"

  given
    A client is opening EmberFlow outside a git-backed project

  when initialize-workspace-state
    workspaceKind = "non-git"

  then returns workspaceState
    assert defaultPath contains ".emberflow/emberflow.db"
    assert stateRoot contains ".emberflow"
    assert projectionMode == "canonical"

behavior emberflow-v1-persists-canonical-runtime-state [happy_path]
  "When a client records protocol-backed work in EmberFlow V1, the canonical state is persisted in the runtime store"

  given
    A client is using EmberFlow as the runtime authority for work state
    @track = Track {{ id: "track-001" }}
    @task = Task {{ id: "task-001" }}

  when record-runtime-state
    trackId = @track.id
    taskId = @task.id
    eventKind = "progress"

  then returns runtimeState
    assert trackId == "track-001"
    assert taskId == "task-001"
    assert eventKind == "progress"
    assert store == "canonical"

behavior emberflow-v1-surfaces-canonical-task-visibility [happy_path]
  "When a client asks EmberFlow V1 for current runtime visibility, the runtime returns canonical executor, execution, and intent_summary for the active task"

  given
    A client is using EmberFlow V1 as the canonical visibility and state layer
    @track = Track {{ id: "track-001" }}
    @task = Task {{ id: "task-visibility", executor: "assistant", execution: "direct delegation", intent_summary: "Implement canonical task visibility fields" }}

  when query-runtime-visibility
    trackId = @track.id

  then returns visibilityState
    assert trackId == "track-001"
    assert source == "canonical"
    assert executor == "assistant"
    assert execution == "direct delegation"
    assert intent_summary contains "canonical task visibility"

behavior emberflow-v1-treats-context-status-as-projection [edge_case]
  "When EmberFlow V1 emits projected runtime status, the status file remains a projection of canonical events rather than the source of truth"

  given
    A progress event has been recorded in EmberFlow
    @track = Track {{ id: "track-001" }}

  when project-runtime-status
    trackId = @track.id

  then returns runtimeProjection
    assert trackId == "track-001"
    assert targetPath == ".emberflow/context/status.md"
    assert source == "canonical-event-store"

  then side_effect
    assert Runtime status remains reconstructible from canonical events even if the projection file is rewritten

behavior emberflow-v1-exposes-queryable-runtime-tools [happy_path]
  "When a client needs to write or read runtime state, EmberFlow V1 exposes that capability through a minimal MCP surface"

  given
    A client needs to coordinate work through EmberFlow

  when use-runtime-tools
    toolset = "minimal-mcp"

  then returns capability
    assert toolset == "minimal-mcp"
    assert writesCanonicalEvents is_present
    assert readsTrackState is_present
    assert readsRuntimeStatus is_present

behavior emberflow-v1-supports-cross-runtime-review-visibility [happy_path]
  "When multiple runtimes record review tasks on the same track, EmberFlow V1 surfaces canonical visibility for each executor so the orchestrator can aggregate cross-runtime review findings"

  given
    A track has review tasks from two different runtimes
    @track = Track {{ id: "track-cross-review" }}
    @claudeTask = Task {{ id: "task-claude-review", trackId: @track.id, status: "done", phase: "reviewing", executor: "claude" }}
    @codexTask = Task {{ id: "task-codex-review", trackId: @track.id, status: "done", phase: "reviewing", executor: "codex" }}

  when query-runtime-visibility
    trackId = @track.id

  then returns visibilityState
    assert trackId == "track-cross-review"
    assert source == "canonical"
    assert tasks contains executor "claude"
    assert tasks contains executor "codex"

behavior emberflow-v1-rejects-runtime-writes-outside-the-protocol [error_case]
  "When a client attempts to write runtime state outside the shared protocol contract, EmberFlow V1 rejects the write instead of accepting ungoverned state"

  given
    A client submits a runtime message outside the supported protocol

  when record-runtime-state
    eventKind = "retry"

  then returns validationError
    assert field == "eventKind"
    assert reason contains "unsupported"

depends on emberflow-runtime-store >= 0.2.0
depends on emberflow-projection-engine >= 0.1.0
depends on emberflow-mcp-surface >= 0.5.0
depends on emberflow-canonical-track-model >= 0.1.0
depends on emberflow-project-layout >= 0.1.0
