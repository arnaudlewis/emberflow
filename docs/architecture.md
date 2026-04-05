# EmberFlow Architecture

EmberFlow is a standalone open-source project. Its job is to own the
coordination and persistence layer that agent systems build on top of.

EmberFlow's boundary is intentionally narrow:

| Area | Responsibility | Current state |
|------|----------------|---------------|
| State model | Own canonical track state, task visibility fields, and optional projected filesystem views | Implemented in `state-model.md` |
| MCP surface | Expose runtime operations, self-description, resource reads, and write tools to clients | Library surface + local stdio transport present |
| Runtime | Validate, store, and project protocol events | Rust crate + SQLite store + projection engine |
| Schemas | Define the runtime data contract | SQLite schema implemented |
| Tests | Guard the EmberFlow behaviors | Present |

## Object Model

EmberFlow revolves around four runtime concepts:

- **Track** — durable work unit and resume context
- **Task** — execution unit attached to a track or running independently; may
  carry canonical visibility metadata such as `executor`, `execution`, and
  `intent_summary`
- **Event** — append-only runtime message
- **Projection** — derived view used by humans or other tools
- **Resource** — read-only MCP view over canonical state

The MCP read layer exposes these canonical concepts as read-only resources:

- `emberflow://workspace/overview` — workspace-wide cross-track visibility
- `emberflow://tracks/{trackId}/record` — canonical track record
- `emberflow://tracks/{trackId}/resume` — track summary, plan, and runtime
  visibility in one resume view
- `emberflow://tracks/{trackId}/context` — canonical track context composed
  from metadata, brief, and plan
- `emberflow://tracks/{trackId}/brief` — durable track summary / intent
  context
- `emberflow://tracks/{trackId}/plan` — canonical execution plan
- `emberflow://tracks/{trackId}/runtime` — current runtime state for one track
- `emberflow://tracks/{trackId}/events` — canonical event history for one
  track
- `emberflow://tasks/{taskId}/visibility` — executor, execution, and intent
  summary for one task
- `emberflow://tasks/{taskId}/events` — canonical event history for one task
- `emberflow://protocol/client-contract` — bootstrap and read-vs-write
  contract summary

The state model is split into canonical data and optional projections:

- canonical track metadata, brief, plan, tasks, and events live in SQLite
- task visibility metadata lives with the canonical task record and is
  projected from the same store
- projected filesystem views are derived from the canonical store; automatic
  filesystem refresh is handled by the projection engine in projected mode
- projected files may be helpful for readability, but they are never the
  source of truth
- read resources are derived from canonical state and are the only public
  read path
- tools are the public write path

These concepts are intentionally separate from agent roles and domain
knowledge. EmberFlow owns the runtime contract; other products own the
tooling and workflows that consume it.

## What EmberFlow Does Not Own

- agent role definitions
- domain knowledge
- workflow prompting
- adapter-specific installation logic
- client-specific display chrome for the visibility block
- public read methods

Those remain outside EmberFlow.

## See Also

- [`README.md`](README.md)
- [`state-model.md`](state-model.md)
- [`projections.md`](projections.md)
- [`integration.md`](integration.md)
