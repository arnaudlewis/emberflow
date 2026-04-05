spec emberflow-mcp-stdio-transport v1.0.0
title "EmberFlow MCP stdio transport"

description
  Defines the standalone local stdio transport that exposes EmberFlow's
  resources-only read surface and write-tool runtime surface as a consumable
  MCP server process for Runestone and any other external MCP client. The
  transport supports the standard MCP initialize, resource discovery/read, and
  tool lifecycle methods over Content-Length framed JSON-RPC stdio so generic
  clients can complete the initialize handshake, discover and read the same
  canonical views, and invoke the same stable write surface.

motivation
  EmberFlow should be consumable as a local MCP server without embedding the
  Rust library directly into each client. The transport must stay thin: expose
  the canonical EmberFlow write tools, stable read-only resource discovery,
  and the clean reads-via-resources contract without reintroducing public read
  methods or client-specific behavior.

behavior stdio-transport-starts-as-local-process [happy_path]
  "When a client launches the EmberFlow MCP transport, EmberFlow starts as a local stdio process instead of a remote service"

  given
    A client is starting EmberFlow MCP for a local project

  when start-stdio-server

  then returns transportReady
    assert mode == "stdio"
    assert hosting == "local-process"
    assert auth == "none"
    assert endpoint == "stdin-stdout"

behavior stdio-transport-initializes-over-stdio [happy_path]
  "When a client initializes EmberFlow through the stdio transport, EmberFlow returns only the standard MCP initialization envelope over the session"

  given
    A client has launched the local EmberFlow stdio process

  when initialize
    client = "runestone"

  then returns initializationResponse
    assert protocolVersion == "2024-11-05"
    assert serverInfo.name == "emberflow"
    assert capabilities.resources is_present
    assert capabilities.tools is_present

behavior stdio-transport-omits-resource-catalog-from-initialize [happy_path]
  "When a client initializes EmberFlow through stdio, EmberFlow does not embed a resource catalog in the initialize payload"

  given
    A client has launched the local EmberFlow stdio process

  when initialize
    client = "external-client"

  then returns initializationResponse
    assert resources is_absent
    assert resourceViews is_absent

behavior stdio-transport-uses-content-length-framed-json-rpc [happy_path]
  "When a client speaks standard MCP over stdio, EmberFlow exchanges Content-Length framed JSON-RPC messages so a host can complete the initialize handshake"

  given
    A client has launched the local EmberFlow stdio process

  when initialize
    client = "external-client"

  then returns initializationResponse
    assert protocolVersion == "2024-11-05"
    assert serverInfo.name == "emberflow"

behavior stdio-transport-mirrors-client-framing [happy_path]
  "When a client sends JSON lines (newline-delimited) requests instead of Content-Length framed requests, EmberFlow responds in JSON lines so the client receives valid JSON rather than Content-Length headers it cannot parse"

  given
    A client that uses JSON lines framing (no Content-Length header) has launched the local EmberFlow stdio process

  when initialize
    client = "json-lines-client"
    framing = "json-lines"

  then returns initializationResponse
    assert responseFraming == "json-lines"
    assert protocolVersion == "2024-11-05"
    assert serverInfo.name == "emberflow"

behavior stdio-transport-advertises-stable-tool-surface [happy_path]
  "When a client initializes the EmberFlow stdio transport, EmberFlow advertises the stable EmberFlow-prefixed write-tool surface through standard MCP tool capabilities"

  given
    A client has launched the local EmberFlow stdio process

  when initialize
    client = "external-client"

  then returns initializationResponse
    assert capabilities.tools is_present

behavior stdio-transport-omits-emberflow-bootstrap-extension-fields-from-initialize [happy_path]
  "When a client initializes EmberFlow through stdio, EmberFlow keeps its EmberFlow-specific bootstrap extensions out of the initialize payload"

  given
    A client has launched the local EmberFlow stdio process

  when initialize
    client = "external-client"

  then returns initializationResponse
    assert emberflowCapabilities is_absent
    assert knowledgeViews is_absent
    assert projectedFiles is_absent
    assert sourceOfTruth is_absent

behavior stdio-transport-omits-runtime-bootstrap-fields-from-initialize [happy_path]
  "When a client initializes EmberFlow through stdio, EmberFlow keeps runtime bootstrap visibility fields out of the initialize payload"

  given
    A client has launched the local EmberFlow stdio process

  when initialize
    client = "external-client"

  then returns initializationResponse
    assert systemRole is_absent
    assert trackBootstrap is_absent
    assert workspaceDb is_absent

behavior stdio-transport-resolves-workspace-root-from-current-directory [happy_path]
  "When a client launches the EmberFlow stdio transport without explicit path overrides, EmberFlow resolves the workspace root using the current project layout rules"

  given
    A local project uses the standard EmberFlow layout

  when start-stdio-server
    cwd = "/workspace/repo"

  then returns transportReady
    assert workspaceRoot == "/workspace/repo"
    assert workspaceDb.defaultPath contains ".emberflow/emberflow.db"

behavior stdio-transport-honors-explicit-workspace-root [happy_path]
  "When a client provides an explicit workspace root for the stdio transport, EmberFlow uses that workspace root for state resolution"

  given
    A client knows the intended EmberFlow workspace root

  when start-stdio-server
    workspaceRoot = "/workspace/repo"

  then returns transportReady
    assert workspaceRoot == "/workspace/repo"
    assert workspaceDb.stateRoot == "/workspace/repo/.emberflow"

behavior stdio-transport-honors-explicit-state-path-compatibility [happy_path]
  "When a client provides an explicit canonical state path for compatibility, the EmberFlow stdio transport opens that canonical state path"

  given
    A client is using the explicit canonical state path compatibility mode

  when start-stdio-server
    statePath = "/workspace/repo/.emberflow/emberflow.db"

  then returns transportReady
    assert workspaceDb.defaultPath == "/workspace/repo/.emberflow/emberflow.db"

behavior stdio-transport-dispatches-canonical-track-reads [happy_path]
  "When a client reads canonical track context through the stdio transport, EmberFlow returns the same context through the public resource layer"

  given
    A track already has canonical metadata, brief sections, and plan phases
    @track = Track {{ id: "track-001" }}

  when read-resource
    uri = "emberflow://tracks/track-001/context"

  then returns resource
    assert uri == "emberflow://tracks/track-001/context"
    assert trackId == "track-001"
    assert metadata is_present
    assert brief is_present
    assert plan is_present

behavior stdio-transport-lists-readable-resource-catalog [happy_path]
  "When a client asks EmberFlow over stdio for its resource catalog, EmberFlow returns concrete resource objects with uri fields for the stable read-only views"

  given
    A client has launched the local EmberFlow stdio process

  when list-resources

  then returns resourceCatalog
    assert resources is_present
    assert resources contains {"uri": "emberflow://workspace/overview"}
    assert resources contains {"uri": "emberflow://tracks/{trackId}/transparency"}
    assert resources contains {"uri": "emberflow://protocol/client-contract"}

behavior stdio-transport-lists-resource-templates-via-standard-mcp-method [happy_path]
  "When a client asks EmberFlow over stdio for resource templates through the standard MCP discovery method, EmberFlow returns stable resource templates with uriTemplate fields only"

  given
    A client has launched the local EmberFlow stdio process

  when standard-resource-template-discovery

  then returns resourceCatalog
    assert resourceTemplates is_present
    assert resourceTemplates contains {"uriTemplate": "emberflow://workspace/overview"}
    assert resourceTemplates contains {"uriTemplate": "emberflow://tracks/{trackId}/transparency"}
    assert resourceTemplates contains {"uriTemplate": "emberflow://protocol/client-contract"}

behavior stdio-transport-accepts-standard-initialized-notification [happy_path]
  "When a client sends the standard MCP initialized notification after a successful initialize handshake, EmberFlow accepts it without emitting a protocol error response"

  given
    A client has completed a successful standard MCP initialize handshake with EmberFlow

  when initialized-notification

  then returns noProtocolError
    assert response == "none"

behavior stdio-transport-reads-resource-views [happy_path]
  "When a client reads one of EmberFlow's advertised resources over stdio, EmberFlow returns the corresponding high-level read-only view"

  given
    The workspace already has canonical state for the requested view

  when read-resource
    uri = "emberflow://workspace/overview"

  then returns resource
    assert uri == "emberflow://workspace/overview"
    assert tracks is_present

behavior stdio-transport-reads-resource-via-standard-mcp-method [happy_path]
  "When a client reads an EmberFlow resource through the standard MCP resource read method, EmberFlow returns the same canonical content it exposes through the legacy alias"

  given
    The workspace already has canonical state for the requested view

  when standard-resource-read
    uri = "emberflow://workspace/overview"

  then returns resource
    assert uri == "emberflow://workspace/overview"
    assert tracks is_present

behavior stdio-transport-lists-tools-via-standard-mcp-method [happy_path]
  "When a client asks EmberFlow over stdio for the standard MCP tool catalog, EmberFlow returns the stable EmberFlow-prefixed write-tool surface with schemas"

  given
    A client has launched the local EmberFlow stdio process

  when standard-tool-discovery

  then returns toolCatalog
    assert tools == ["emberflow-track-create", "emberflow-track-metadata-upsert", "emberflow-track-brief-replace", "emberflow-track-plan-replace", "emberflow-track-archive", "emberflow-track-delete", "emberflow-task-create", "emberflow-event-record"]
    assert inputSchemas is_present

behavior stdio-transport-calls-tools-via-standard-mcp-method [happy_path]
  "When a client invokes an EmberFlow mutation through the standard MCP tool call method, EmberFlow applies the same canonical mutation as the legacy alias"

  given
    A client has a supported canonical write operation to invoke

  when standard-tool-call
    name = "emberflow-track-create"

  then returns toolResult
    assert isError == false
    assert content is_present

behavior stdio-transport-deletes-track-state-through-prefixed-tools [happy_path]
  "When a client deletes a track through stdio, EmberFlow removes the track and the related task visibility state so orphaned runtime data is not left behind"

  given
    A client has launched the local EmberFlow stdio process
    A track already has related runtime task state

  when standard-tool-call
    name = "emberflow-track-delete"

  then returns toolResult
    assert isError == false

  then side_effect
    assert The deleted track no longer serves task visibility resources through stdio

behavior stdio-transport-rejects-unprefixed-tool-names [error_case]
  "When a client calls an EmberFlow write tool by an unprefixed legacy name, EmberFlow rejects the request so only the canonical EmberFlow-prefixed names remain supported"

  given
    A client has a supported canonical write operation to invoke

  when standard-tool-call
    name = "create_track"

  then returns protocolError
    assert code == "method_not_found"
    assert message contains "unknown method"

behavior stdio-transport-returns-workspace-overview-view [happy_path]
  "When a client reads the workspace overview resource through stdio, EmberFlow returns the high-level dynamic workspace view"

  given
    The workspace already has at least one canonical track with runtime visibility

  when read-resource
    uri = "emberflow://workspace/overview"

  then returns workspaceOverview
    assert source == "emberflow-canonical-state"
    assert tracks is_present

behavior stdio-transport-reads-workspace-overview-with-legacy-null-metadata [edge_case]
  "When a client reads the workspace overview resource for a track whose metadata columns contain legacy NULL values, EmberFlow still returns a valid overview without crashing"

  given
    The workspace has a track with NULL metadata columns from a legacy schema

  when read-resource
    uri = "emberflow://workspace/overview"

  then returns workspaceOverview
    assert source == "emberflow-canonical-state"
    assert tracks is_present

behavior stdio-transport-returns-track-resume-view [happy_path]
  "When a client reads the track resume resource through stdio, EmberFlow returns the composed resume view for that track"

  given
    A track already has summary, plan, and runtime visibility
    @track = Track {{ id: "track-001" }}

  when read-resource
    uri = "emberflow://tracks/track-001/resume"

  then returns trackResume
    assert source == "emberflow-canonical-state"
    assert trackId == "track-001"
    assert summarySections is_present
    assert plan is_present

behavior stdio-transport-dispatches-canonical-runtime-writes [happy_path]
  "When a client submits a supported runtime mutation through the stdio transport, EmberFlow persists that mutation in the canonical store and keeps follow-up reads on resources"

  given
    A client has a supported canonical runtime event to record

  when record-event
    eventId = "event-001"
    kind = "progress"

  then returns eventRecord
    assert id == "event-001"
    assert kind == "progress"
    assert payload is_present

behavior stdio-transport-rejects-unknown-method [error_case]
  "When a client sends an unknown MCP method through the EmberFlow stdio transport, EmberFlow returns a structured protocol error"

  given
    A client sends a method outside the EmberFlow MCP contract

  when call-unknown-method
    method = "delete-track"

  then returns protocolError
    assert code == "method_not_found"
    assert message contains "unknown"

behavior stdio-transport-rejects-invalid-parameters [error_case]
  "When a client sends invalid parameters through the EmberFlow stdio transport, EmberFlow returns a structured validation error"

  given
    A client calls a supported EmberFlow MCP method with invalid parameters

  when record-event
    eventId = ""
    kind = "retry"

  then returns validationError
    assert field is_present
    assert reason is_present

behavior stdio-transport-reports-missing-project-state [error_case]
  "When a client launches the EmberFlow stdio transport for a project whose EmberFlow state cannot be resolved, EmberFlow reports the failure explicitly"

  given
    The local project state cannot be resolved for EmberFlow

  when start-stdio-server
    cwd = "/workspace/broken-repo"

  then returns startupError
    assert source == "workspace-resolution"
    assert message contains "EmberFlow"

behavior stdio-transport-keeps-protocol-output-separate-from-diagnostics [edge_case]
  "When the EmberFlow stdio transport emits diagnostics, it keeps MCP protocol output separate from human-readable diagnostics"

  given
    The local EmberFlow stdio transport emits a runtime diagnostic while serving requests

  when emit-diagnostic
    level = "warn"

  then returns diagnosticOutput
    assert channel == "stderr"

  then side_effect
    assert MCP protocol responses remain machine-readable on stdout

depends on emberflow-mcp-surface >= 1.0.0
depends on emberflow-project-layout >= 1.0.0
