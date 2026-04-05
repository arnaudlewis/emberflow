# EmberFlow Projections

EmberFlow can optionally materialize filesystem projections from canonical
SQLite state. Projections are readable views, not the source of truth.

This page describes the projection engine behavior that is implemented in the
current runtime.

## Implemented runtime behavior

- canonical runtime state lives in SQLite
- projected mode records derived projection targets alongside canonical state
- projected files refresh automatically after canonical writes
- dirty projection targets persist if the filesystem is temporarily
  unavailable
- EmberFlow retries pending projection refreshes on the next mutation, read,
  or startup
- consumers can read durable track context and runtime status from the runtime
  surface without treating projected files as authoritative
- `.emberflow/context/` and `.emberflow/tracks/` are the filesystem view
  surfaces projected mode maintains for readability and compatibility

Deleting projected files does not delete canonical truth. EmberFlow can rebuild
the views deterministically from SQLite on the next access or mutation.

## What gets projected

Projected mode keeps the shared project state visible under `.emberflow/`.
The initial projection surface includes:

- `.emberflow/context/status.md`
- `.emberflow/tracks/tracks.md`
- `.emberflow/tracks/<track-id>/metadata.json`
- `.emberflow/tracks/<track-id>/brief.md`
- `.emberflow/tracks/<track-id>/plan.md`
- `.emberflow/tracks/<track-id>/index.md`

Those files exist to make the canonical state easier to inspect, reuse, or
bridge into older workflows. They never replace SQLite as the source of truth.

The projection engine keeps the views up to date continuously without a manual
materialization command. If the filesystem is briefly unavailable, the views
may lag behind the database for a short time, but the canonical record does not
change.

## Consistency rules

- canonical SQLite always wins
- projections are derived
- manual edits to projected files are not authoritative
- consumers should rely on EmberFlow reads and writes when freshness matters
- if projected files diverge temporarily, the next EmberFlow access refreshes
  the dirty targets

## Why this exists

- humans can inspect the state in files when needed
- existing workflows can keep a readable on-disk view
- the project retains one canonical store while still supporting filesystem
  ergonomics

## See Also

- [`state-model.md`](state-model.md)
- [`architecture.md`](architecture.md)
- [`integration.md`](integration.md)
