# EmberFlow Distribution Strategy

This document captures the intended packaging and installation strategy for
EmberFlow. It is a planning note, not an implementation contract.

## Goals

EmberFlow should be easy to install, upgrade, and remove without depending on a
checked-out repository.

The distribution path should:

- keep EmberFlow local-first
- make the canonical runtime easy to install on a clean machine
- let Claude and Codex install EmberFlow as standard clients
- keep the MCP runtime separate from the client-facing install flow
- preserve additive, idempotent installation behavior

EmberFlow is not intended to become a hosted service in V1. The canonical
runtime state is local and anchored in the project root, so a remote daemon or
auth layer would add complexity without solving the core problem.

## Recommended package shape

The target product shape is two binaries:

- `emberflow` — the user-facing CLI for install, uninstall, diagnostics, and
  future operational commands
- `emberflow-mcp` — the local stdio MCP transport used by clients

Keeping these separate makes the installation and client wiring clearer:

- `emberflow` owns distribution and adapter setup
- `emberflow-mcp` owns the runtime protocol boundary

This separation also keeps future client integrations from depending on a
repository checkout or `cargo run` fallback.

## Primary distribution channel: Homebrew

Homebrew is the intended primary channel for end users.

The target user flow should look like:

```bash
brew install arnaudlewis/tap/emberflow
emberflow install claude --scope user
emberflow install codex --scope user
```

For teams or shared repos:

```bash
emberflow install claude --scope project
emberflow install codex --scope project
```

Homebrew is the preferred channel because it gives EmberFlow:

- a familiar install/update path on macOS
- a stable global binary location
- easy uninstallation
- versioned releases that can be pinned or upgraded

The public install command is:

```bash
brew install arnaudlewis/tap/emberflow
```

## Secondary channels

Secondary channels are acceptable for development and fallback distribution:

- GitHub Releases with prebuilt artifacts
- `cargo install` for contributors and Rust-native users
- repository checkout + local adapters for active development only

These channels are useful, but they should not be the primary user story.

## Adapter installation target

Once EmberFlow is installed, the adapter commands should configure clients from
the installed package, not from a repo checkout.

Target experience:

- `emberflow install claude --scope user|project`
- `emberflow install codex --scope user|project`

The adapter install flow should remain additive and idempotent:

- merge into existing Claude/Codex config instead of replacing it
- preserve existing hooks, permissions, and instructions
- add EmberFlow-specific guardrails and MCP wiring
- keep `.emberflow/` projections derived, not authoritative

### Claude

Claude should receive:

- an EmberFlow MCP stdio registration
- additive `settings.json` changes
- additive `CLAUDE.md` contract text
- EmberFlow guard hooks for direct `.emberflow/` writes and destructive shell
  access

### Codex

Codex should receive:

- an EmberFlow MCP stdio registration
- additive `config.toml` changes limited to the EmberFlow MCP server entry
- additive root instructions that explain the client contract
- no global `approval_policy` or `sandbox_mode` overrides

## Current development mode vs target distribution

### Current development mode

Today, the adapters are still repository-first:

- they live under `emberflow/adapters/*`
- they can fall back to `cargo run`
- they are good for development, CI, and iteration

That is a useful dev mode, but it is not the final product distribution path.

### Target product mode

The target product mode is:

- install EmberFlow once
- run the CLI from the installed binary
- configure Claude/Codex through the CLI
- keep all client setup additive and local

In other words: the repo is for development; the installed package is for use.

## Current release pipeline

EmberFlow's release workflow now mirrors the minter-style dry-run / execute
pattern:

- `workflow_dispatch` defaults to a dry run
- `execute=true` bumps `emberflow/Cargo.toml`, renders changelog
  preview/body, creates the `emberflow-vX.Y.Z` tag, builds release archives,
  publishes the GitHub Release, and updates the Homebrew tap
- dry runs still validate CI plus build/package output, but stop before
  tagging, publishing, or tap updates
- release archives are produced for macOS arm/x86 and Linux arm/x86 targets
- every release publishes checksums alongside the archives
- archives always include `README.md`; `LICENSE` is included when present and
  skipped without failing if the snapshot does not contain one
- the tap update installs both `emberflow` and `emberflow-mcp`

The release job is intentionally separate from the development CI so clean
machine install tests can stay focused on the packaged experience.

## What the future distribution PR must do

The eventual distribution PR should:

- define the Homebrew formula or tap workflow
- ensure the installed CLI can locate its adapter assets
- remove the need for repository-relative wrappers in the end-user path
- keep the adapters and MCP transport working from a clean install
- preserve the local-only V1 model
- add release and update instructions that match the real packaging flow

## Release workflow shape

The release workflow should match the Minter quality bar:

- `workflow_dispatch` with `execute=false` by default for a real dry run mode
- a prepare phase that previews the next semantic version, changelog, and
  `emberflow-vX.Y.Z` tag from conventional commits
- an execute path that bumps `emberflow/Cargo.toml`, tags, builds, publishes,
  and updates the Homebrew tap
- multi-target archives for macOS and Linux
- checksum generation and GitHub Release publication
- automated Homebrew formula updates for the EmberFlow tap

Dry runs should validate the release gate and produce a human-readable preview
without creating tags, releases, or tap commits.

## Local development and testing strategy

Before the distribution layer is finalized:

- keep using the repo checkout for active development
- validate adapter install flows with temporary `HOME` / project-root fixtures
- keep `cargo test`, adapter shell tests, and `minter ci` as the core gating
  checks
- once the Homebrew path exists, add a clean-machine install test path to prove
  the packaged experience

The important rule is that the development workflow should mirror the product
workflow as closely as possible, even before the package is released.

## See Also

- [`README.md`](README.md)
- [`docs/integration.md`](docs/integration.md)
- [`docs/projections.md`](docs/projections.md)
- [`docs/state-model.md`](docs/state-model.md)
