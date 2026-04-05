spec emberflow-mcp-surface v1.0.0
title "EmberFlow MCP surface"

description
  Defines the public EmberFlow MCP contract for initializing the runtime,
  advertising a resources-only read surface, persisting canonical runtime and
  track mutations through write tools, and serving self-describing dynamic
  knowledge resources to clients.

motivation
  Runestone and other clients need one clean EmberFlow MCP contract: read
  canonical state through stable resources, write durable state through tools,
  discover the full high-level resource catalog at bootstrap, and avoid any
  public low-level read methods that would split the client contract.

behavior mcp-initializes-with-capability-discovery [happy_path]
  "When a client initializes EmberFlow MCP, EmberFlow reports the supported EmberFlow-prefixed write tools as public runtime capabilities"

  given
    A client is connecting to EmberFlow for runtime coordination

  when initialize
    client = "runestone"

  then returns initializationResponse
    assert capabilities == ["emberflow-track-create", "emberflow-track-metadata-upsert", "emberflow-track-brief-replace", "emberflow-track-plan-replace", "emberflow-track-archive", "emberflow-track-delete", "emberflow-task-create", "emberflow-event-record", "emberflow-task-claim", "emberflow-task-release"]

behavior mcp-initializes-with-self-description [happy_path]
  "When a client initializes EmberFlow MCP, EmberFlow describes its canonical role and the dynamic knowledge views it serves"

  given
    A client is connecting to EmberFlow for runtime coordination

  when initialize
    client = "runestone"

  then returns initializationResponse
    assert systemRole == "canonical tracked runtime and visibility layer"
    assert sourceOfTruth == "emberflow-canonical-state"
    assert projectedFiles == "derived-only"
    assert preferredClientSequence == ["initialize", "list_resources", "read_resource", "mutate_via_emberflow_mcp"]
    assert knowledgeViews == ["workspace-overview", "track-record", "track-resume", "track-transparency", "track-context", "track-brief", "track-plan", "track-runtime", "track-events", "task-visibility", "task-events", "client-contract"]

behavior mcp-initializes-with-resource-discovery [happy_path]
  "When a client initializes EmberFlow MCP, EmberFlow advertises the stable read-only resources it serves for dynamic knowledge"

  given
    A client is connecting to EmberFlow for runtime coordination

  when initialize
    client = "runestone"

  then returns initializationResponse
    assert resources == ["emberflow://workspace/overview", "emberflow://tracks/{trackId}/record", "emberflow://tracks/{trackId}/resume", "emberflow://tracks/{trackId}/transparency", "emberflow://tracks/{trackId}/context", "emberflow://tracks/{trackId}/brief", "emberflow://tracks/{trackId}/plan", "emberflow://tracks/{trackId}/runtime", "emberflow://tracks/{trackId}/events", "emberflow://tasks/{taskId}/visibility", "emberflow://tasks/{taskId}/events", "emberflow://protocol/client-contract"]

behavior mcp-initializes-with-track-bootstrap [happy_path]
  "When a client initializes EmberFlow MCP, EmberFlow reports the durable track bootstrap expectations needed to resume work"

  given
    A client is connecting to EmberFlow for runtime coordination

  when initialize
    client = "runestone"

  then returns initializationResponse
    assert trackBootstrap.briefArtifact == "brief.md"
    assert trackBootstrap.requiredSections == ["objective", "context", "decisions", "non_goals", "current_state", "workspace_branch_pr_context", "next_step"]

  then side_effect
    assert Durable track bootstrap expectations are surfaced before any runtime mutation happens

behavior mcp-initializes-with-workspace-db-metadata [happy_path]
  "When a client initializes EmberFlow MCP, EmberFlow reports deterministic workspace DB metadata without mutating state"

  given
    A client is connecting to EmberFlow for runtime coordination

  when initialize
    client = "runestone"

  then returns initializationResponse
    assert workspaceDb.defaultPath contains ".emberflow/emberflow.db"
    assert workspaceDb.stateRoot contains ".emberflow"
    assert workspaceDb.mode == "sqlite"
    assert workspaceDb.initialization == "ready"

behavior mcp-initializes-with-canonical-mode-by-default [happy_path]
  "When a client initializes EmberFlow MCP without a projection override, EmberFlow reports canonical mode as the default project-state mode"

  given
    A client is connecting to EmberFlow for runtime coordination

  when initialize
    client = "runestone"

  then returns initializationResponse
    assert workspaceDb.projectionMode == "canonical"

behavior mcp-initializes-with-configured-projection-mode [happy_path]
  "When a project config requests projected mode, EmberFlow reports that projected mode while keeping SQLite canonical"

  given
    A project config has opted into projected mode

  when initialize
    client = "runestone"

  then returns initializationResponse
    assert workspaceDb.projectionMode == "projected"
    assert workspaceDb.stateRoot contains ".emberflow"

behavior mcp-records-canonical-event [happy_path]
  "When a client submits a supported protocol event through the MCP surface, EmberFlow records it in the canonical runtime store"

  given
    A caller has canonical event data to persist

  when record-event
    eventId = "event-001"
    kind = "progress"

  then returns eventRecord
    assert id == "event-001"
    assert kind == "progress"
    assert payload is_present

behavior mcp-creates-task-with-canonical-visibility [happy_path]
  "When a client creates a task through EmberFlow MCP with canonical visibility fields, EmberFlow returns executor, execution, and intent_summary with the task record"

  given
    A caller is creating runtime work through EmberFlow MCP
    @track = Track {{ id: "track-001" }}

  when create-task
    taskId = "task-visibility"
    trackId = @track.id
    title = "Investigate the transport contract"
    status = "running"
    phase = "planning"
    executor = "assistant"
    execution = "interactive session"
    intent_summary = "Investigate the transport contract end to end"

  then returns taskRecord
    assert id == "task-visibility"
    assert trackId == "track-001"
    assert executor == "assistant"
    assert execution == "interactive session"
    assert intent_summary == "Investigate the transport contract end to end"

behavior mcp-creates-task-with-generic-executor-fallback [edge_case]
  "When a client omits executor while creating a task through EmberFlow MCP, EmberFlow falls back to the generic executor label 'assistant'"

  given
    A caller is creating runtime work through EmberFlow MCP

  when create-task
    taskId = "task-standalone"
    title = "Inspect recent events"
    status = "running"
    phase = "verifying"
    execution = "interactive session"
    intent_summary = "Inspect recent events for the user"

  then returns taskRecord
    assert id == "task-standalone"
    assert executor == "assistant"
    assert execution == "interactive session"
    assert intent_summary == "Inspect recent events for the user"

behavior mcp-reads-track-record-resource [happy_path]
  "When a client reads a track record resource, EmberFlow returns the durable canonical track state for that work unit"

  given
    A track already exists in the runtime store
    @track = Track {{ id: "track-001", status: "in-progress" }}

  when read-resource
    uri = "emberflow://tracks/track-001/record"

  then returns trackRecord
    assert uri == "emberflow://tracks/track-001/record"
    assert id == "track-001"
    assert status == "in-progress"
    assert updatedAt is_present

behavior mcp-reads-track-events-resource [happy_path]
  "When a client reads a track events resource, EmberFlow returns recent canonical events for that track"

  given
    A track already has recorded event history
    @track = Track {{ id: "track-001" }}

  when read-resource
    uri = "emberflow://tracks/track-001/events"

  then returns eventFeed
    assert uri == "emberflow://tracks/track-001/events"
    assert trackId == "track-001"
    assert items is_present

behavior mcp-reads-empty-track-events-resource [edge_case]
  "When a client reads a track events resource for a track with no canonical events yet, EmberFlow returns an empty feed instead of failing"

  given
    A track exists but has no recorded event history yet
    @track = Track {{ id: "track-empty" }}

  when read-resource
    uri = "emberflow://tracks/track-empty/events"

  then returns eventFeed
    assert uri == "emberflow://tracks/track-empty/events"
    assert trackId == "track-empty"
    assert items == "empty"

behavior mcp-rejects-unsupported-event-kind [error_case]
  "When a client submits an event kind outside the protocol contract, EmberFlow rejects the request"

  given
    A caller submits an unsupported runtime message

  when record-event
    eventId = "event-invalid"
    kind = "retry"

  then returns validationError
    assert field == "kind"
    assert reason contains "unsupported"

behavior mcp-reads-track-context-resource [happy_path]
  "When a client reads a track context resource, EmberFlow returns the canonical metadata, brief, and plan together from the runtime store"

  given
    A track already has canonical metadata, brief sections, and plan phases stored
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when read-resource
    uri = "emberflow://tracks/emberflow-runtime-split-20260402/context"

  then returns trackContext
    assert uri == "emberflow://tracks/emberflow-runtime-split-20260402/context"
    assert trackId == "emberflow-runtime-split-20260402"
    assert metadata is_present
    assert brief is_present
    assert plan is_present

behavior mcp-lists-readable-resource-catalog [happy_path]
  "When a client asks EmberFlow for its resource catalog, EmberFlow returns the stable read-only resource URIs for dynamic knowledge"

  given
    A client is connected to EmberFlow MCP

  when list-resources

  then returns resourceCatalog
    assert items == ["emberflow://workspace/overview", "emberflow://tracks/{trackId}/record", "emberflow://tracks/{trackId}/resume", "emberflow://tracks/{trackId}/transparency", "emberflow://tracks/{trackId}/context", "emberflow://tracks/{trackId}/brief", "emberflow://tracks/{trackId}/plan", "emberflow://tracks/{trackId}/runtime", "emberflow://tracks/{trackId}/events", "emberflow://tasks/{taskId}/visibility", "emberflow://tasks/{taskId}/events", "emberflow://protocol/client-contract"]

behavior mcp-reads-workspace-overview-resource [happy_path]
  "When a client reads the workspace overview resource, EmberFlow returns the canonical workspace summary without requiring manual view reconstruction"

  given
    The workspace already has at least one track with canonical visibility state

  when read-resource
    uri = "emberflow://workspace/overview"

  then returns resource
    assert uri == "emberflow://workspace/overview"
    assert tracks is_present

behavior mcp-reads-track-resume-resource [happy_path]
  "When a client reads a track resume resource, EmberFlow returns the composed resume view for that track"

  given
    A track already has canonical metadata, brief sections, plan phases, and runtime visibility
    @track = Track {{ id: "track-001", status: "in-progress" }}

  when read-resource
    uri = "emberflow://tracks/track-001/resume"

  then returns resource
    assert uri == "emberflow://tracks/track-001/resume"
    assert trackId == "track-001"
    assert summarySections is_present

behavior mcp-reads-track-runtime-resource [happy_path]
  "When a client reads a track runtime resource, EmberFlow returns the canonical runtime visibility view for that track"

  given
    A current runtime task already exists for one track
    @track = Track {{ id: "track-001" }}

  when read-resource
    uri = "emberflow://tracks/track-001/runtime"

  then returns resource
    assert uri == "emberflow://tracks/track-001/runtime"
    assert trackId == "track-001"
    assert statusLine is_present

behavior mcp-reads-track-transparency-resource [happy_path]
  "When a client reads a track transparency resource, EmberFlow returns the display-ready canonical state for that track"

  given
    A current runtime task already exists for one track
    @track = Track {{ id: "track-001" }}

  when read-resource
    uri = "emberflow://tracks/track-001/transparency"

  then returns resource
    assert uri == "emberflow://tracks/track-001/transparency"
    assert trackId == "track-001"
    assert trackStatus is_present
    assert taskStatus is_present
    assert phase is_present
    assert next is_present

behavior mcp-reads-task-visibility-resource [happy_path]
  "When a client reads a task visibility resource, EmberFlow returns the canonical visibility fields for that task"

  given
    A current runtime task already exists for one track
    @task = Task {{ id: "task-visibility", executor: "assistant", execution: "interactive session", intent_summary: "Investigate the transport contract end to end" }}

  when read-resource
    uri = "emberflow://tasks/task-visibility/visibility"

  then returns resource
    assert uri == "emberflow://tasks/task-visibility/visibility"
    assert taskId == "task-visibility"
    assert executor == "assistant"
    assert execution == "interactive session"
    assert intent_summary == "Investigate the transport contract end to end"

behavior mcp-reads-client-contract-resource [happy_path]
  "When a client reads the client contract resource, EmberFlow returns the canonical reading and mutation guidance for consumers"

  given
    A client is connected to EmberFlow MCP

  when read-resource
    uri = "emberflow://protocol/client-contract"

  then returns resource
    assert uri == "emberflow://protocol/client-contract"
    assert sourceOfTruth == "emberflow-canonical-state"
    assert projectedFiles == "derived-only"
    assert preferredClientSequence is_present

behavior mcp-reads-empty-track-context-resource [edge_case]
  "When a client reads a track context resource after only metadata has been stored, EmberFlow returns the stored metadata with empty brief and plan collections"

  given
    A track has canonical metadata but no brief sections or plan phases yet
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when read-resource
    uri = "emberflow://tracks/emberflow-runtime-split-20260402/context"

  then returns trackContext
    assert uri == "emberflow://tracks/emberflow-runtime-split-20260402/context"
    assert trackId == "emberflow-runtime-split-20260402"
    assert metadata is_present
    assert brief.sections == "empty"
    assert plan.phases == "empty"

behavior mcp-reads-track-brief-resource [happy_path]
  "When a client reads a track brief resource, EmberFlow returns the ordered stored brief sections"

  given
    A track already has canonical brief sections stored
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when read-resource
    uri = "emberflow://tracks/emberflow-runtime-split-20260402/brief"

  then returns trackBrief
    assert uri == "emberflow://tracks/emberflow-runtime-split-20260402/brief"
    assert trackId == "emberflow-runtime-split-20260402"
    assert sections is_present

behavior mcp-reads-empty-track-brief-resource [edge_case]
  "When a client reads a track brief resource before any sections are stored, EmberFlow returns an empty brief"

  given
    A track has canonical metadata but no brief sections yet
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when read-resource
    uri = "emberflow://tracks/emberflow-runtime-split-20260402/brief"

  then returns trackBrief
    assert uri == "emberflow://tracks/emberflow-runtime-split-20260402/brief"
    assert trackId == "emberflow-runtime-split-20260402"
    assert sections == "empty"

behavior mcp-reads-track-plan-resource [happy_path]
  "When a client reads a track plan resource, EmberFlow returns the ordered stored phases and items"

  given
    A track already has canonical plan phases stored
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when read-resource
    uri = "emberflow://tracks/emberflow-runtime-split-20260402/plan"

  then returns trackPlan
    assert uri == "emberflow://tracks/emberflow-runtime-split-20260402/plan"
    assert trackId == "emberflow-runtime-split-20260402"
    assert phases is_present
    assert items is_present

behavior mcp-reads-empty-track-plan-resource [edge_case]
  "When a client reads a track plan resource before any phases are stored, EmberFlow returns an empty plan"

  given
    A track has canonical metadata but no plan phases yet
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when read-resource
    uri = "emberflow://tracks/emberflow-runtime-split-20260402/plan"

  then returns trackPlan
    assert uri == "emberflow://tracks/emberflow-runtime-split-20260402/plan"
    assert trackId == "emberflow-runtime-split-20260402"
    assert phases == "empty"

behavior mcp-upserts-canonical-track-metadata [happy_path]
  "When a client upserts canonical track metadata, EmberFlow persists it in the runtime store"

  given
    A caller has canonical metadata for a track
    @track = Track {{ id: "emberflow-runtime-split-20260402", status: "in-progress" }}

  when upsert-track-metadata
    trackId = @track.id

  then returns trackMetadata
    assert trackId == "emberflow-runtime-split-20260402"
    assert status == "in-progress"
    assert branch is_present

behavior mcp-updates-canonical-track-metadata [happy_path]
  "When a client upserts canonical track metadata for an existing track, EmberFlow updates the stored canonical metadata"

  given
    A track already has canonical metadata stored
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when upsert-track-metadata
    trackId = @track.id

  then returns trackMetadata
    assert trackId == "emberflow-runtime-split-20260402"
    assert description contains "updated"
    assert branch == "arnaudlewis/mcp-track-surface-v2"

behavior mcp-rejects-invalid-track-brief-section [error_case]
  "When a client replaces canonical track brief sections with an invalid section key, EmberFlow rejects the request"

  given
    A track already has canonical metadata stored
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when replace-track-brief
    trackId = @track.id

  then returns validationError
    assert field == "section_key"
    assert reason contains "present"

behavior mcp-rejects-invalid-track-plan-item-placement [error_case]
  "When a client replaces canonical track plan phases with an item missing stable placement, EmberFlow rejects the request"

  given
    A track already has canonical metadata stored
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when replace-track-plan
    trackId = @track.id

  then returns validationError
    assert field == "item"
    assert reason contains "stable"

behavior mcp-manual-archive-requires-terminal-track-status [error_case]
  "When a client manually archives an active track, EmberFlow rejects the request until the track has reached review or done"

  given
    A track is still active and not ready for manual archival
    @track = Track {{ id: "track-001", status: "in-progress" }}

  when archive-track
    trackId = @track.id

  then returns validationError
    assert field == "status"
    assert reason contains "review"
    assert reason contains "done"

behavior mcp-replaces-canonical-track-brief [happy_path]
  "When a client replaces canonical track brief sections, EmberFlow stores the new ordered brief"

  given
    A track already exists in the runtime store
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when replace-track-brief
    trackId = @track.id

  then returns trackBrief
    assert trackId == "emberflow-runtime-split-20260402"
    assert sections is_present

behavior mcp-replaces-canonical-track-plan [happy_path]
  "When a client replaces canonical track plan phases, EmberFlow stores the new ordered plan"

  given
    A track already exists in the runtime store
    @track = Track {{ id: "emberflow-runtime-split-20260402" }}

  when replace-track-plan
    trackId = @track.id

  then returns trackPlan
    assert trackId == "emberflow-runtime-split-20260402"
    assert phases is_present
    assert items is_present

depends on emberflow-project-layout >= 0.1.0
