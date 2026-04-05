[![Latest Release](https://img.shields.io/github/v/release/arnaudlewis/emberflow?label=version&color=blue)](https://github.com/arnaudlewis/emberflow/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/arnaudlewis/emberflow/total?color=green)](https://github.com/arnaudlewis/emberflow/releases)
[![Homebrew](https://img.shields.io/badge/homebrew-arnaudlewis%2Ftap%2Femberflow-orange)](https://github.com/arnaudlewis/homebrew-tap)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

# emberflow

The local-first runtime for agentic workflow tracking.

EmberFlow gives AI agents a single source of truth for work in progress. When agents work on multi-step tasks, they need somewhere to record what phase they're in, who's handling what, and what happened — across tool calls, across conversations, and across restarts. EmberFlow is that place: a local SQLite store exposed via MCP, so work persists, resumes, and stays observable without any cloud infrastructure.

## Install

```bash
brew install arnaudlewis/tap/emberflow
```

Installs both `emberflow` (CLI) and `emberflow-mcp` (MCP server).

<details>
<summary>Manual download</summary>

Download the archive for your platform from the [latest release](https://github.com/arnaudlewis/emberflow/releases/latest), extract it, and place `emberflow` and `emberflow-mcp` on your `PATH`. SHA-256 checksums are in `SHA256SUMS.txt`.

</details>

<details>
<summary>Build from source</summary>

```bash
cargo install --path .
```

</details>

## Get Started

Connect EmberFlow to your agent client, then let the agent drive:

```bash
# Claude Code
emberflow install claude --scope user

# Codex
emberflow install codex --scope user
```

For a project-scoped install shared across team members via the repository:

```bash
emberflow install claude --scope project
emberflow install codex --scope project
```

The install commands are additive and idempotent — they merge into existing client configuration without replacing it.

Once connected, your agent can create a track, record progress, and pick up where it left off:

```
# In a Claude Code or Codex session, the agent will:
# 1. Call initialize() to discover EmberFlow's capabilities and workspace state
# 2. Create a track for the current work unit
# 3. Record events as work progresses
# 4. Resume from canonical state in any future session
```

Follow the [integration guide](docs/integration.md) for the full walkthrough.

## Core Concepts

| Concept | What it is |
|---------|-----------|
| **Track** | A durable work unit — the resume context for a feature, task, or investigation |
| **Task** | An execution unit attached to a track; carries who is handling it and what it's doing |
| **Event** | An append-only message on a track or task — the canonical history |
| **Projection** | A derived filesystem view of canonical state; useful for humans and debugging, never authoritative |
| **Resource** | A read-only MCP view over canonical state — the only public read surface |

State lives in SQLite. Everything else is derived from it.

## MCP Surface

### Read resources

Load context through these URIs before acting. There are no public read methods in the MCP contract — resources are the only read path.

| Resource URI | Purpose |
|--------------|---------|
| `emberflow://workspace/overview` | Workspace-wide view of active tracks, status, and next steps |
| `emberflow://tracks/{trackId}/record` | Canonical track record |
| `emberflow://tracks/{trackId}/resume` | Composed resume view: summary, plan, and runtime visibility |
| `emberflow://tracks/{trackId}/transparency` | Display-ready canonical state block for one track |
| `emberflow://tracks/{trackId}/context` | Track context composed from metadata, brief, and plan |
| `emberflow://tracks/{trackId}/brief` | Durable track summary and intent context |
| `emberflow://tracks/{trackId}/plan` | Execution plan phases, tasks, and progress |
| `emberflow://tracks/{trackId}/runtime` | Current runtime state for one track |
| `emberflow://tracks/{trackId}/events` | Canonical event history for one track |
| `emberflow://tasks/{taskId}/visibility` | Executor, execution, and intent summary for one task |
| `emberflow://tasks/{taskId}/events` | Canonical event history for one task |
| `emberflow://protocol/client-contract` | Client bootstrap and read-vs-write contract summary |

`initialize()` self-describes EmberFlow's canonical role, recommends the client sequence, and advertises the resource layer.

### Write tools

Use tools for all durable mutations:

| Tool | Purpose |
|------|---------|
| `emberflow-track-create` | Create a new track |
| `emberflow-track-metadata-upsert` | Update track metadata |
| `emberflow-track-brief-replace` | Replace the track brief |
| `emberflow-track-plan-replace` | Replace the execution plan |
| `emberflow-task-create` | Create a task on a track |
| `emberflow-task-claim` | Claim a task for execution |
| `emberflow-task-release` | Release a claimed task |
| `emberflow-event-record` | Append an event to a track or task |
| `emberflow-track-archive` | Archive a completed track |
| `emberflow-track-delete` | Delete a track |

## Architecture

EmberFlow owns the runtime and control-plane layer:

- canonical SQLite state under `.emberflow/` at the project root
- optional projected filesystem views kept aligned with canonical state
- a local stdio MCP transport for reads (resources) and writes (tools)
- adapter helpers for Claude and Codex that configure clients from the installed package

**What EmberFlow does not own:** agent role definitions, domain knowledge, workflow prompting, or client-specific display logic. Those stay outside EmberFlow.

## Documentation

- [`docs/README.md`](docs/README.md) — documentation home and reading order
- [`docs/state-model.md`](docs/state-model.md) — where state lives and how canonical vs projected modes differ
- [`docs/projections.md`](docs/projections.md) — filesystem projection lifecycle
- [`docs/architecture.md`](docs/architecture.md) — product boundary and object model
- [`docs/integration.md`](docs/integration.md) — how agent systems integrate with EmberFlow
- [`mcp/README.md`](mcp/README.md) — local stdio MCP surface

## License

MIT. See [LICENSE](LICENSE).
