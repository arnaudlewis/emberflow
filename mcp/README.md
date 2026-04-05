# EmberFlow MCP

This directory documents the EmberFlow MCP transport boundary.

EmberFlow exposes a library-style MCP-ready surface in `src/`, and this
directory documents the local stdio transport that presents that surface to
consumers while keeping canonical state in the shared `.emberflow/` project
root.

Transport responsibilities:

- standard MCP `initialize` handshake for capability discovery
- standard MCP `notifications/initialized` lifecycle notification
- self-description of EmberFlow's canonical role and recommended client usage
- read-resource discovery for workspace, track, task, and protocol context
- protocol validation
- canonical state reads via resources and writes via tools
- read-only resources for workspace overview, track record, resume, context,
  brief, plan, runtime, events, task visibility, task events, and the client
  contract
- runtime responses that surface projected visibility state derived from the
  canonical store
- local-process execution only; no remote daemon or auth layer

The initialize handshake advertises the MCP protocol version, server info,
resource/tool capabilities, the durable track bootstrap contract, and the
workspace database metadata required to find the shared project state. It also
describes EmberFlow as the canonical tracked/runtime layer, marks projected
files as derived-only, and advertises the read-resource layer available from
the transport. Consumers can use that response to locate the shared project
root, load the right resource URIs, and decide whether they need a resource
read or a tool call. There are no public read methods in the transport
contract.

The transport should not make projections canonical. It exposes the canonical
store first and treats projections as derived views; the projection engine
docs describe the refresh behavior in more detail.

## Read resources

These read-only URIs compose canonical EmberFlow state into client-friendly
views:

| Resource URI | Purpose |
|--------------|---------|
| `emberflow://workspace/overview` | Workspace-wide tracks, status, and next steps |
| `emberflow://tracks/{trackId}/record` | Canonical track record |
| `emberflow://tracks/{trackId}/resume` | Composed resume view for one track |
| `emberflow://tracks/{trackId}/transparency` | Display-ready canonical transparency block for one track |
| `emberflow://tracks/{trackId}/context` | Canonical track context composed from track metadata, brief, and plan |
| `emberflow://tracks/{trackId}/brief` | Durable track summary / intent context |
| `emberflow://tracks/{trackId}/plan` | Execution plan phases, tasks, and progress |
| `emberflow://tracks/{trackId}/runtime` | Current runtime state for one track |
| `emberflow://tracks/{trackId}/events` | Canonical event history for one track |
| `emberflow://tasks/{taskId}/visibility` | Executor, execution, and intent summary for one task |
| `emberflow://tasks/{taskId}/events` | Canonical event history for one task |
| `emberflow://protocol/client-contract` | Bootstrap, read-vs-write, and transparency contract summary |

## Read vs tool split

- use read resources when you need context, resume state, or a workspace
  overview
- use the standard MCP resource methods (`resources/list`,
  `resources/templates/list`, `resources/read`) when you need generic client
  compatibility; EmberFlow also keeps its legacy aliases (`list_resources`,
  `read_resource`) for older callers
- use the standard MCP tool methods (`tools/list`, `tools/call`) for canonical
  mutations when working with a generic MCP host; EmberFlow exposes only the
  canonical EmberFlow-prefixed tool names
- use the EmberFlow-prefixed tool names (`emberflow-track-create`,
  `emberflow-track-metadata-upsert`, `emberflow-track-brief-replace`,
  `emberflow-track-plan-replace`, `emberflow-task-create`,
  `emberflow-event-record`) when invoking tools through a generic MCP host
- use tools when you need durable mutations
- there are no public read methods in the contract
- keep projected files derived; never treat them as the authoritative source
  of truth

## See Also

- [`../docs/state-model.md`](../docs/state-model.md)
- [`../docs/projections.md`](../docs/projections.md)
- [`../docs/integration.md`](../docs/integration.md)
