# EmberFlow Integration

EmberFlow is designed to integrate with agent systems through a small,
explicit runtime surface instead of through implicit file conventions.

Current state: EmberFlow is available as a library-style runtime surface with
canonical SQLite storage, optional projected filesystem views, and a local
stdio MCP transport. The public MCP contract is resource-oriented for reads
and tool-oriented for writes.

## Generic integration pattern

An agent system integrates with EmberFlow in five steps:

1. Resolve the project state root and open the shared `.emberflow/` store.
2. Call `initialize()` to discover supported capabilities, workspace
   bootstrap metadata, EmberFlow's canonical role, and the read resources
   available to the client.
3. Load context through the resource layer when the client needs a
   user-facing summary before acting:
   - `emberflow://workspace/overview`
   - `emberflow://tracks/{trackId}/record`
   - `emberflow://tracks/{trackId}/resume`
   - `emberflow://tracks/{trackId}/transparency`
   - `emberflow://tracks/{trackId}/context`
   - `emberflow://tracks/{trackId}/brief`
   - `emberflow://tracks/{trackId}/plan`
   - `emberflow://tracks/{trackId}/runtime`
   - `emberflow://tracks/{trackId}/events`
   - `emberflow://tasks/{taskId}/visibility`
   - `emberflow://tasks/{taskId}/events`
   - `emberflow://protocol/client-contract`
4. Use tools for durable mutations. There are no public read methods in the
   contract.
5. Treat projected files as views only; never treat them as the source of
   truth. EmberFlow's projection engine keeps those views aligned
   automatically after canonical mutations, and retries if the filesystem is
   temporarily unavailable.

That pattern works whether the consumer is a CLI agent, a workflow runner, or
a higher-level orchestration layer.

## Canonical vs projected usage

- In **canonical** mode, EmberFlow stores and reads the shared SQLite state
  only.
- In **projected** mode, EmberFlow also materializes human-readable filesystem
  views under `.emberflow/`; those views refresh automatically after canonical
  writes.

Both modes use the same canonical SQLite data.

## Read resources

The MCP surface is intentionally self-describing:

- `initialize()` explains EmberFlow's canonical role, marks projected files
  as derived-only, and advertises the read-resource layer
- `emberflow://workspace/overview` returns a high-level view of known tracks
  and their current visibility fields
- `emberflow://tracks/{trackId}/record` returns the canonical track record
- `emberflow://tracks/{trackId}/resume` returns one composed resume view with
  summary, plan, and runtime visibility for a single track
- `emberflow://tracks/{trackId}/transparency` returns one display-ready
  canonical state block for a single track
- `emberflow://tracks/{trackId}/context` returns the canonical track context
  composed from metadata, brief, and plan
- `emberflow://tracks/{trackId}/plan` returns the execution plan view
- `emberflow://tracks/{trackId}/brief` returns the durable summary / intent
  context
- `emberflow://tracks/{trackId}/runtime` returns the current runtime status
  view for a track
- `emberflow://tracks/{trackId}/events` returns the canonical event history for
  a track
- `emberflow://tasks/{taskId}/visibility` returns executor, execution, and
  intent summary for one task
- `emberflow://tasks/{taskId}/events` returns the canonical event history for
  a task
- `emberflow://protocol/client-contract` returns the short contract summary
  clients should follow

These reads let clients consume dynamic context directly from EmberFlow
instead of rebuilding it from multiple low-level calls or static knowledge
files.

## Mutation tools

Repo-first adapter helpers in this branch install EmberFlow on top of an
existing local workspace setup for development. The EmberFlow runtime itself
remains consumer-agnostic. Its public read surface is resources only.

Use the tool surface when you need:

- creating or updating tracks, tasks, events, brief sections, or plan phases
- runtime writes that must persist to the canonical store

## Downstream consumers

Any agent framework or orchestration layer can be a consumer. Each consumer
uses the same standard transport, resource layer, and visibility contract.

## Integration rules

- do not duplicate the state model in the consumer
- do not treat projections as authoritative
- do not bake a consumer-specific workflow into EmberFlow itself
- do not invent a second source of truth for task visibility metadata
- do not add public read methods to the MCP contract

## See Also

- [`README.md`](README.md)
- [`state-model.md`](state-model.md)
- [`projections.md`](projections.md)
- [`../mcp/README.md`](../mcp/README.md)
