# EmberFlow Codex adapter

This adapter installs EmberFlow into Codex CLI as a local stdio MCP server.

What it does:

- installs an `emberflow-mcp` wrapper under the selected Codex scope
- installs Codex root instructions that state the EmberFlow client contract
- merges an `emberflow` MCP server entry into `config.toml`
- leaves existing Codex settings untouched

Supported scopes:

- `user`
- `project`

Current guardrails:

- EmberFlow MCP server configured through `config.toml`
- developer instructions reinforce the canonical EmberFlow state contract
- existing `approval_policy` and `sandbox_mode` settings are preserved

Compared with Claude, Codex guardrails are currently lighter in this adapter
and rely on the client contract plus the user's existing Codex policy
settings rather than on tool hooks.

The installed root instructions tell Codex to:

- initialize EmberFlow before tracked work
- treat EmberFlow as the canonical source of tracked state
- display the transparency block with canonical track/task status fields
- reload canonical state after durable mutations
- never guess state when EmberFlow is unavailable
