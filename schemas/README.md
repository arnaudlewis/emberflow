# EmberFlow Schemas

This directory holds the structural contracts that EmberFlow runtime code
consumes.

For the first wave, the canonical SQLite schema lives in
`runtime/sqlite/schema.sql`.

The library surface also exposes deterministic initialization metadata for
clients that need to bootstrap a fresh workspace before mutating runtime
state. That metadata is derived from the runtime layer rather than stored as a
separate schema artifact.

Later this folder can host generated JSON Schema, Rust types, or other shared
contract artifacts derived from the runtime schema.
