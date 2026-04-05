# EmberFlow Claude adapter

This adapter installs EmberFlow into Claude Code as a local stdio MCP server.

What it does:

- installs an `emberflow-mcp` wrapper under the selected Claude scope
- merges an EmberFlow contract block into `CLAUDE.md` additively
- merges EmberFlow guard hooks into `settings.json`
- registers the `emberflow` MCP server with `claude mcp add`

Supported scopes:

- `user`
- `project`

Guardrails:

- keeps existing Claude instructions intact while appending the EmberFlow contract block
- blocks direct shell deletion of `.emberflow/`
- blocks direct shell writes into `.emberflow/`
- blocks direct edits/writes into `.emberflow/`

The adapter treats EmberFlow as the canonical source of tracked state and the
filesystem projections as derived views only.

The installed transparency contract now expects a richer canonical display so
clients can see the current track/task state after each update.
