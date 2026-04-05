#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(git -C "$ROOT_DIR" rev-parse --show-toplevel 2>/dev/null || echo "$ROOT_DIR")"

assert_exists() {
  local path="$1"
  if [ ! -e "$path" ]; then
    echo "FAIL: expected path to exist: $path" >&2
    exit 1
  fi
}

assert_dir() {
  local path="$1"
  assert_exists "$path"
  if [ ! -d "$path" ]; then
    echo "FAIL: expected directory: $path" >&2
    exit 1
  fi
}

assert_file() {
  local path="$1"
  assert_exists "$path"
  if [ ! -f "$path" ]; then
    echo "FAIL: expected file: $path" >&2
    exit 1
  fi
}

assert_contains() {
  local needle="$1"
  local path="$2"
  if ! grep -qF "$needle" "$path"; then
    echo "FAIL: expected '$needle' in $path" >&2
    exit 1
  fi
}

assert_dir "$ROOT_DIR/mcp"
assert_dir "$ROOT_DIR/runtime"
assert_dir "$ROOT_DIR/runtime/sqlite"
assert_dir "$ROOT_DIR/schemas"
assert_dir "$ROOT_DIR/specs"
assert_dir "$ROOT_DIR/src"
assert_dir "$ROOT_DIR/adapters"
assert_dir "$ROOT_DIR/adapters/claude"
assert_dir "$ROOT_DIR/adapters/codex"
assert_dir "$ROOT_DIR/adapters/claude/hooks"

assert_file "$ROOT_DIR/.gitignore"
assert_file "$ROOT_DIR/Cargo.toml"
assert_file "$ROOT_DIR/tests/ci.sh"
assert_file "$ROOT_DIR/tests/adapters.sh"
assert_file "$ROOT_DIR/mcp/README.md"
assert_file "$ROOT_DIR/runtime/README.md"
assert_file "$ROOT_DIR/runtime/sqlite/schema.sql"
assert_file "$ROOT_DIR/schemas/README.md"
assert_file "$ROOT_DIR/minter.config.json"
assert_file "$ROOT_DIR/minter.lock"
assert_file "$ROOT_DIR/specs/emberflow-v1.spec"
assert_file "$ROOT_DIR/specs/emberflow-runtime-store.spec"
assert_file "$ROOT_DIR/specs/emberflow-projection-engine.spec"
assert_file "$ROOT_DIR/specs/emberflow-canonical-track-model.spec"
assert_file "$ROOT_DIR/specs/emberflow-mcp-surface.spec"
assert_file "$ROOT_DIR/specs/emberflow-client-contract.spec"
assert_file "$ROOT_DIR/specs/emberflow-project-layout.spec"
assert_file "$ROOT_DIR/tests/client_contract.rs"
assert_file "$ROOT_DIR/tests/codex_adapter.rs"
assert_file "$ROOT_DIR/tests/projection_dirty_targets.rs"
assert_file "$ROOT_DIR/adapters/claude/install.sh"
assert_file "$ROOT_DIR/adapters/claude/README.md"
assert_file "$ROOT_DIR/adapters/claude/hooks/emberflow-bash-guard.sh"
assert_file "$ROOT_DIR/adapters/claude/hooks/emberflow-write-guard.sh"
assert_file "$ROOT_DIR/adapters/codex/install.sh"
assert_file "$ROOT_DIR/adapters/codex/README.md"
assert_file "$REPO_ROOT/.github/workflows/emberflow.yml"
assert_contains "bash tests/ci.sh" "$REPO_ROOT/.github/workflows/emberflow.yml"
assert_contains ".minter/" "$ROOT_DIR/.gitignore"
assert_contains ".emberflow/" "$REPO_ROOT/.gitignore"

echo "emberflow layout checks passed"
