# EmberFlow Codex Root Instructions

These instructions apply to the root Codex session when EmberFlow is installed.
Treat them as stricter than local repo guidance when they overlap.

## Core rule

Before any tracked work, initialize EmberFlow against the active workspace.
Before any planned work, resolve or create the durable EmberFlow track that will
carry the work.

## EmberFlow client contract

- Resolve the workspace root explicitly.
- If the workspace root cannot be resolved, fail explicitly and do not continue
  without EmberFlow. no silent fallback.
- Treat EmberFlow canonical state as the source of truth.
- Treat `.emberflow/` files and projected files as derived views only.
- Use EmberFlow MCP for durable mutations on tracked state.
- Load the minimal context before continuing:
  - track
  - track transparency
  - next
  - runtime context
- Planned work does not proceed until a durable track has been resolved.
- The root/orchestrator layer declares semantic tracked-work transitions such as
  plan approval, artifact approval, phase changes, and completion.
- Worker agents do not own canonical EmberFlow track, plan, or task writes.
- Hooks may enforce observable invariants, but hooks alone are not the source of
  semantic tracked state.

## Transparency block

When tracked work is active, display a clear EmberFlow block after every tracked
mutation (track create/update, task create/update, event record, brief/plan
replace, metadata upsert, archive, delete) that names:

- `Source: EmberFlow`
- `Track: <track-id>`
- `Track status: <track-status>`
- `Task status: <task-status>`
- `Phase: <phase>`
- `Next: <next-step>`
- `Task: <task-id>`
- `Executor: <executor>`
- `Execution: <execution>`
- `Intent: <intent-summary>`
- `Updated: <updated-at>`

If EmberFlow omits a field, display `unavailable from EmberFlow` instead of
inventing a value.

## Failure behavior

- Never present guessed canonical state. Use `Source: unavailable` when EmberFlow
  cannot supply a truthful source label.
- If EmberFlow is unavailable, report that explicitly.
- After a durable mutation, reload `emberflow://tracks/{trackId}/transparency`
  from canonical EmberFlow state before displaying the updated transparency
  block.
- Before a handoff, display the current EmberFlow state.

## Uniform contract

This contract applies to every client equally, including Claude, Codex, and any
other orchestration layer. There is no privileged client path and every client follows the same contract.
