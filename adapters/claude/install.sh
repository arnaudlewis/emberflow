#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

SCOPE="user"
PROJECT_ROOT=""
CLAUDE_BIN="${CLAUDE_BIN:-$(command -v claude 2>/dev/null || true)}"

usage() {
  cat <<'EOF'
Usage: ./emberflow/adapters/claude/install.sh [--scope user|project] [--project-root PATH]

Installs the EmberFlow Claude adapter by:
- creating an EmberFlow MCP stdio wrapper
- merging EmberFlow guard hooks into Claude settings
- registering the emberflow MCP server if the Claude CLI is available

Examples:
  ./emberflow/adapters/claude/install.sh
  ./emberflow/adapters/claude/install.sh --scope project --project-root /path/to/repo
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --scope)
      SCOPE="${2:-}"
      shift 2
      ;;
    --project-root)
      PROJECT_ROOT="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

case "$SCOPE" in
  user)
    TARGET_ROOT="${HOME}/.claude"
    ;;
  project)
    if [ -z "$PROJECT_ROOT" ]; then
      PROJECT_ROOT="$(pwd)"
    fi
    TARGET_ROOT="${PROJECT_ROOT}/.claude"
    ;;
  *)
    echo "Invalid scope: $SCOPE" >&2
    exit 1
    ;;
esac

HOOKS_DIR="${TARGET_ROOT}/hooks"
BIN_DIR="${TARGET_ROOT}/bin"
SETTINGS_FILE="${TARGET_ROOT}/settings.json"
WRAPPER_PATH="${BIN_DIR}/emberflow-mcp"
CLAUDE_TEMPLATE="${SCRIPT_DIR}/CLAUDE.md"

mkdir -p "$HOOKS_DIR" "$BIN_DIR"

cat > "$WRAPPER_PATH" <<EOF
#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${REPO_ROOT}"
MANIFEST="\$REPO_ROOT/emberflow/Cargo.toml"
BINARY="\$REPO_ROOT/emberflow/target/debug/emberflow-mcp"

if [ -x "\$BINARY" ]; then
  exec "\$BINARY" "\$@"
fi

exec cargo run --quiet --manifest-path "\$MANIFEST" --bin emberflow-mcp -- "\$@"
EOF
chmod +x "$WRAPPER_PATH"

for hook in emberflow-bash-guard.sh emberflow-write-guard.sh; do
  cp "$SCRIPT_DIR/hooks/$hook" "$HOOKS_DIR/$hook"
  chmod +x "$HOOKS_DIR/$hook"
done

python3 - "$SETTINGS_FILE" "$HOOKS_DIR" "$REPO_ROOT" "$TARGET_ROOT/CLAUDE.md" "$CLAUDE_TEMPLATE" <<'PY'
import json
import os
import sys
from pathlib import Path

settings_path = Path(sys.argv[1])
hooks_dir = Path(sys.argv[2])
repo_root = sys.argv[3]
claude_path = Path(sys.argv[4])
template_path = Path(sys.argv[5])

if settings_path.exists():
  data = json.loads(settings_path.read_text())
else:
  data = {}

permissions = data.setdefault("permissions", {})
allow = permissions.setdefault("allow", [])
for value in ["Bash", "Read", "Edit", "Write", "mcp__emberflow__*"]:
    if value not in allow:
        allow.append(value)

additional_dirs = permissions.setdefault("additionalDirectories", [])
for value in [repo_root]:
    if value not in additional_dirs:
        additional_dirs.append(value)

hooks = data.setdefault("hooks", {})
pre = hooks.setdefault("PreToolUse", [])

required = [
    ("Bash", str(hooks_dir / "emberflow-bash-guard.sh")),
    ("Edit", str(hooks_dir / "emberflow-write-guard.sh")),
    ("Write", str(hooks_dir / "emberflow-write-guard.sh")),
]

for matcher, command in required:
    exists = False
    for entry in pre:
        if entry.get("matcher") != matcher:
            continue
        for hook in entry.get("hooks", []):
            if hook.get("type") == "command" and hook.get("command") == command:
                exists = True
                break
        if exists:
            break
    if exists:
        continue
    pre.append(
        {
            "matcher": matcher,
            "hooks": [
                {
                    "type": "command",
                    "command": command,
                }
            ],
        }
    )

settings_path.write_text(json.dumps(data, indent=2) + "\n")

contract_template = template_path.read_text().strip("\n") + "\n"
existing_claude = claude_path.read_text() if claude_path.exists() else ""
start_marker = "<!-- EMBERFLOW CONTRACT START -->"
end_marker = "<!-- EMBERFLOW CONTRACT END -->"

if start_marker in existing_claude and end_marker in existing_claude:
    start = existing_claude.index(start_marker)
    end = existing_claude.index(end_marker)
    before = existing_claude[:start].rstrip()
    after = existing_claude[end + len(end_marker):].lstrip("\n")
    merged = before
    if before:
        merged += "\n\n"
    merged += contract_template
    if after:
        merged += after if after.startswith("\n") else "\n" + after
elif existing_claude.strip():
    merged = existing_claude.rstrip("\n") + "\n\n" + contract_template
else:
    merged = contract_template

claude_path.parent.mkdir(parents=True, exist_ok=True)
claude_path.write_text(merged)
PY

if [ -n "$CLAUDE_BIN" ]; then
  "$CLAUDE_BIN" mcp remove emberflow -s "$SCOPE" >/dev/null 2>&1 || true
  "$CLAUDE_BIN" mcp add --scope "$SCOPE" emberflow -- "$WRAPPER_PATH"
else
  echo "warning: claude CLI not found; skipping 'claude mcp add'. Wrapper installed at $WRAPPER_PATH" >&2
fi

echo "Installed EmberFlow Claude adapter in $TARGET_ROOT"
