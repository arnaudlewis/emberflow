# EmberFlow Claude transparency contract

<!-- EMBERFLOW CONTRACT START -->
EmberFlow transparency contract for Claude.

When EmberFlow is active:
- Resolve EmberFlow and call `initialize` before tracked work.
- Before planned work, resolve or create the durable EmberFlow track that will carry the work.
- Display the current tracked state as coming from EmberFlow.
- If a field is unavailable, render `unavailable from EmberFlow`.
- After each tracked mutation, reload `emberflow://tracks/{trackId}/transparency` from canonical state before displaying the updated transparency block.
- before the handoff proceeds, display the current EmberFlow state again.
- Treat `.emberflow/` filesystem projections as derived views only; canonical state stays in EmberFlow.
- Semantic tracked-work transitions come from the root/orchestrator layer, not individual worker agents.
- Worker agents do not own canonical EmberFlow track, plan, or task writes.
- Hooks may enforce observable invariants, but hooks alone are not the source of semantic tracked state.

EmberFlow display shape (all fields — always displayed after every tracked mutation):
- Source: EmberFlow canonical state
- Track: <track-id>
- Track status: <track-status>
- Task status: <task-status-or-unavailable>
- Phase: <phase-or-unavailable>
- Next: <next-step-or-unavailable>
- Task: <task-id-or-unavailable>
- Executor: <executor-or-unavailable>
- Execution: <execution-or-unavailable>
- Intent: <intent-summary-or-unavailable>
- Updated: <updated-at-or-unavailable>
<!-- EMBERFLOW CONTRACT END -->
