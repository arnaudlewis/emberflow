#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT_DIR/.." && pwd)"

assert_exists() {
  local path="$1"
  [ -e "$path" ] || { echo "FAIL: missing $path" >&2; exit 1; }
}

assert_contains() {
  local needle="$1"
  local path="$2"
  grep -qF "$needle" "$path" || { echo "FAIL: expected '$needle' in $path" >&2; exit 1; }
}

assert_equals() {
  local expected="$1"
  local actual="$2"
  [ "$expected" = "$actual" ] || {
    echo "FAIL: expected '$expected', got '$actual'" >&2
    exit 1
  }
}

count_occurrences() {
  local needle="$1"
  local path="$2"
  grep -cF "$needle" "$path" || true
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

FAKE_HOME="$TMP/home"
FAKE_PROJECT="$TMP/project"
FAKE_BIN="$TMP/bin"
LOG="$TMP/claude.log"
mkdir -p "$FAKE_HOME" "$FAKE_PROJECT" "$FAKE_BIN"

cat > "$FAKE_BIN/claude" <<'EOF'
#!/usr/bin/env bash
echo "$@" >> "${FAKE_CLAUDE_LOG:?}"
exit 0
EOF
chmod +x "$FAKE_BIN/claude"

export HOME="$FAKE_HOME"
export PATH="$FAKE_BIN:$PATH"
export FAKE_CLAUDE_LOG="$LOG"
export CLAUDE_BIN="$FAKE_BIN/claude"

mkdir -p "$HOME/.claude"
cat > "$HOME/.claude/settings.json" <<'EOF'
{
  "permissions": {
    "allow": ["Read"],
    "additionalDirectories": ["~/custom"]
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "/tmp/custom-hook.sh"
          }
        ]
      }
    ]
  }
}
EOF

bash "$ROOT_DIR/adapters/claude/install.sh" --scope user >/dev/null
bash "$ROOT_DIR/adapters/claude/install.sh" --scope user >/dev/null

assert_exists "$HOME/.claude/bin/emberflow-mcp"
assert_exists "$HOME/.claude/hooks/emberflow-bash-guard.sh"
assert_exists "$HOME/.claude/hooks/emberflow-write-guard.sh"
assert_contains "\"mcp__emberflow__*\"" "$HOME/.claude/settings.json"
assert_contains "/tmp/custom-hook.sh" "$HOME/.claude/settings.json"
assert_contains "emberflow-bash-guard.sh" "$HOME/.claude/settings.json"
assert_contains "emberflow-write-guard.sh" "$HOME/.claude/settings.json"
[ "$(count_occurrences "emberflow-bash-guard.sh" "$HOME/.claude/settings.json")" -eq 1 ] || {
  echo "FAIL: expected Claude bash guard hook to be deduplicated" >&2
  exit 1
}
assert_contains "mcp add --scope user emberflow -- $HOME/.claude/bin/emberflow-mcp" "$LOG"

mkdir -p "$FAKE_PROJECT/.claude"
bash "$ROOT_DIR/adapters/claude/install.sh" --scope project --project-root "$FAKE_PROJECT" >/dev/null
assert_exists "$FAKE_PROJECT/.claude/settings.json"
assert_exists "$FAKE_PROJECT/.claude/bin/emberflow-mcp"

mkdir -p "$HOME/.codex"
cat > "$HOME/.codex/config.toml" <<'EOF'
model = "gpt-5.4"
approval_policy = "untrusted"
sandbox_mode = "read-only"

[mcp_servers.custom]
command = "/tmp/custom"
args = []
EOF

bash "$ROOT_DIR/adapters/codex/install.sh" --scope user >/dev/null
bash "$ROOT_DIR/adapters/codex/install.sh" --scope user >/dev/null

assert_exists "$HOME/.codex/bin/emberflow-mcp"
assert_contains 'approval_policy = "untrusted"' "$HOME/.codex/config.toml"
assert_contains 'sandbox_mode = "read-only"' "$HOME/.codex/config.toml"
assert_contains '[mcp_servers.custom]' "$HOME/.codex/config.toml"
assert_contains '[mcp_servers.emberflow]' "$HOME/.codex/config.toml"
[ "$(count_occurrences "[mcp_servers.emberflow]" "$HOME/.codex/config.toml")" -eq 1 ] || {
  echo "FAIL: expected Codex emberflow MCP section to be deduplicated" >&2
  exit 1
}

mkdir -p "$FAKE_PROJECT/.codex"
cat > "$FAKE_PROJECT/.codex/config.toml" <<'EOF'
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "workspace-write"

[mcp_servers.custom]
command = "/tmp/custom"
args = []
EOF

bash "$ROOT_DIR/adapters/codex/install.sh" --scope project --project-root "$FAKE_PROJECT" >/dev/null
assert_exists "$FAKE_PROJECT/.codex/bin/emberflow-mcp"
assert_contains 'approval_policy = "never"' "$FAKE_PROJECT/.codex/config.toml"
assert_contains 'sandbox_mode = "workspace-write"' "$FAKE_PROJECT/.codex/config.toml"
assert_contains '[mcp_servers.custom]' "$FAKE_PROJECT/.codex/config.toml"
assert_contains '[mcp_servers.emberflow]' "$FAKE_PROJECT/.codex/config.toml"

codex_wrapper_resolves_repo_root_from_cwd_before_env_and_falls_back() {
  local home_dir="$TMP/home-wrapper"
  local cwd_repo="$TMP/cwd-repo"
  local env_repo="$TMP/env-repo"
  local wrapper

  mkdir -p "$home_dir" "$cwd_repo" "$env_repo"

  mkdir -p "$cwd_repo/emberflow/target/debug" "$cwd_repo/nested"
  cat > "$cwd_repo/emberflow/Cargo.toml" <<'EOF'
[package]
name = "emberflow"
version = "0.0.0"
EOF
  cat > "$cwd_repo/emberflow/target/debug/emberflow-mcp" <<'EOF'
#!/usr/bin/env bash
echo cwd-repo
EOF
  chmod +x "$cwd_repo/emberflow/target/debug/emberflow-mcp"

  mkdir -p "$env_repo/emberflow/target/debug"
  cat > "$env_repo/emberflow/Cargo.toml" <<'EOF'
[package]
name = "emberflow"
version = "0.0.0"
EOF
  cat > "$env_repo/emberflow/target/debug/emberflow-mcp" <<'EOF'
#!/usr/bin/env bash
echo env-repo
EOF
  chmod +x "$env_repo/emberflow/target/debug/emberflow-mcp"

  PATH="$FAKE_BIN:$PATH" HOME="$home_dir" bash "$ROOT_DIR/adapters/codex/install.sh" --scope user >/dev/null
  wrapper="$home_dir/.codex/bin/emberflow-mcp"

  assert_contains "$REPO_ROOT" "$wrapper"
  assert_contains 'EMBERFLOW_REPO_ROOT' "$wrapper"
  assert_contains 'pwd -P' "$wrapper"

  local cwd_output
  cwd_output="$(cd "$cwd_repo/nested" && EMBERFLOW_REPO_ROOT="$env_repo" "$wrapper")"
  assert_equals "cwd-repo" "$cwd_output"

  local env_output
  env_output="$(cd "$TMP" && EMBERFLOW_REPO_ROOT="$env_repo" "$wrapper")"
  assert_equals "env-repo" "$env_output"
}

codex_wrapper_resolves_repo_root_from_cwd_before_env_and_falls_back

echo "emberflow adapter checks passed"
