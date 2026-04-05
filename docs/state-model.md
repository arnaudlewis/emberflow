# EmberFlow State Model

EmberFlow keeps durable runtime state in SQLite and treats filesystem views as
derived projections.

## Canonical and projected modes

EmberFlow supports two modes:

- **canonical** — the SQLite database is the source of truth; filesystem
  projections are not required
- **projected** — the same canonical SQLite state is still authoritative, but
  EmberFlow also materializes readable filesystem projections

The mode changes how EmberFlow surfaces state, not which data is canonical.
SQLite always wins.

## Project root and `.emberflow/`

EmberFlow stores project-level state under a single `.emberflow/` directory.
That directory contains the shared database and, in projected mode, the
filesystem views built from canonical state.

The resolved root is determined in this order:

1. If Git is available, EmberFlow uses the parent of
   `git rev-parse --path-format=absolute --git-common-dir`.
2. If Git is not available, EmberFlow falls back to `./`.

If Git is available, EmberFlow reads `emberflow.config.json` from the current
worktree/repo toplevel. If Git is not available, it looks in `./`.

If an `emberflow.config.json` file is present, it can override the mode and
optionally redirect where `.emberflow/` lives via a `root` value relative to
the config file's directory.

When no override is present, the resolved root is already the EmberFlow state
root.

The config file is optional. Git-backed projects can rely on the default
resolution, while non-Git directories can still opt in explicitly when they
need a different root.

### Canonical storage

The canonical database lives at:

```text
<resolved_root>/.emberflow/emberflow.db
```

### Projected files

In projected mode, EmberFlow materializes readable files beneath the same
project root, for example:

```text
<resolved_root>/.emberflow/tracks/
<resolved_root>/.emberflow/context/
```

These files are views of canonical SQLite state. They are useful for humans,
debugging, and compatibility, but they are not the source of truth.

Projection refresh behavior is documented separately in
[`projections.md`](projections.md). The current runtime treats SQLite as the
canonical record; in projected mode, EmberFlow refreshes filesystem views
automatically from that canonical state.

### Task visibility metadata

Tasks may carry lightweight visibility fields alongside their runtime state:

- `executor` — who is handling the task, for example a named subagent or the
  generic `assistant` label
- `execution` — the execution context or workflow label, for example
  `interactive session`, `direct delegation`, or `feature`
- `intent_summary` — a short human-readable summary of what the task is meant
  to accomplish

These fields live with the canonical task record in SQLite. They help clients
render a clear, user-facing view of ongoing work without forcing every caller
to invent the same labels independently.

## Why this shape exists

- one canonical store per project, not per workspace
- no manual sync between multiple worktrees
- a clean separation between durable state and readable projections
- a simple default that works with Git repos and still degrades gracefully

## See Also

- [`architecture.md`](architecture.md)
- [`projections.md`](projections.md)
- [`integration.md`](integration.md)
