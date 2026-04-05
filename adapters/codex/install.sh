#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

SCOPE="user"
PROJECT_ROOT=""

usage() {
  cat <<'EOF'
Usage: ./emberflow/adapters/codex/install.sh [--scope user|project] [--project-root PATH]

Installs the EmberFlow Codex adapter by:
- creating an EmberFlow MCP stdio wrapper
- merging an emberflow MCP server section into Codex config
- installing Codex root instructions while preserving existing Codex settings

Examples:
  ./emberflow/adapters/codex/install.sh
  ./emberflow/adapters/codex/install.sh --scope project --project-root /path/to/repo
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
    TARGET_ROOT="${HOME}/.codex"
    ;;
  project)
    if [ -z "$PROJECT_ROOT" ]; then
      PROJECT_ROOT="$(pwd)"
    fi
    TARGET_ROOT="${PROJECT_ROOT}/.codex"
    ;;
  *)
    echo "Invalid scope: $SCOPE" >&2
    exit 1
    ;;
esac

BIN_DIR="${TARGET_ROOT}/bin"
CONFIG_FILE="${TARGET_ROOT}/config.toml"
WRAPPER_PATH="${BIN_DIR}/emberflow-mcp"
ROOT_INSTRUCTIONS_FILE="${TARGET_ROOT}/root.instructions.md"

mkdir -p "$BIN_DIR"
cp "$SCRIPT_DIR/root.instructions.md" "$ROOT_INSTRUCTIONS_FILE"

cat > "$WRAPPER_PATH" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

INSTALL_TIME_REPO_ROOT="__EMBERFLOW_INSTALL_TIME_REPO_ROOT__"

resolve_repo_root() {
  local search_dir
  search_dir="$(pwd -P)"

  while [ "$search_dir" != "/" ]; do
    if [ -f "$search_dir/emberflow/Cargo.toml" ]; then
      printf '%s\n' "$search_dir"
      return 0
    fi

    search_dir="$(dirname "$search_dir")"
  done

  if [ -n "${EMBERFLOW_REPO_ROOT:-}" ] && [ -f "${EMBERFLOW_REPO_ROOT}/emberflow/Cargo.toml" ]; then
    printf '%s\n' "${EMBERFLOW_REPO_ROOT}"
    return 0
  fi

  if [ -f "${INSTALL_TIME_REPO_ROOT}/emberflow/Cargo.toml" ]; then
    printf '%s\n' "${INSTALL_TIME_REPO_ROOT}"
    return 0
  fi

  echo "error: could not resolve EmberFlow repo root from \$PWD, EMBERFLOW_REPO_ROOT, or the install-time fallback" >&2
  exit 1
}

REPO_ROOT="$(resolve_repo_root)"
MANIFEST="$REPO_ROOT/emberflow/Cargo.toml"
BINARY="$REPO_ROOT/emberflow/target/debug/emberflow-mcp"

if [ -x "$BINARY" ]; then
  exec "$BINARY" "$@"
fi

exec cargo run --quiet --manifest-path "$MANIFEST" --bin emberflow-mcp -- "$@"
EOF

python3 - "$WRAPPER_PATH" "$REPO_ROOT" <<'PY'
import sys
from pathlib import Path

wrapper_path = Path(sys.argv[1])
install_time_repo_root = sys.argv[2]

text = wrapper_path.read_text()
text = text.replace(
    "__EMBERFLOW_INSTALL_TIME_REPO_ROOT__",
    install_time_repo_root,
)
wrapper_path.write_text(text)
PY

chmod +x "$WRAPPER_PATH"

python3 - "$CONFIG_FILE" "$WRAPPER_PATH" "$ROOT_INSTRUCTIONS_FILE" <<'PY'
import re
import sys
from pathlib import Path

config_path = Path(sys.argv[1])
wrapper = sys.argv[2]
instructions_path = Path(sys.argv[3])

text = config_path.read_text() if config_path.exists() else ""

instructions = instructions_path.read_text().rstrip("\n")

instructions_block = f'developer_instructions = """\n{instructions}\n"""\n'
developer_re = re.compile(r'(?ms)^developer_instructions\s*=\s*""".*?"""\s*(?=^\[|\Z)')
if developer_re.search(text):
    text = developer_re.sub(instructions_block, text, count=1)
else:
    text = text.rstrip()
    if text:
        text += "\n"
    text += "\n" + instructions_block

section_re = re.compile(r'(?ms)^\[mcp_servers\.emberflow\]\n.*?(?=^\[|\Z)')
section = f'\n[mcp_servers.emberflow]\ncommand = "{wrapper}"\nargs = []\n'

if section_re.search(text):
    text = section_re.sub(section, text)
else:
    text = text.rstrip() + "\n" + section

config_path.write_text(text.lstrip("\n"))
PY

echo "Installed EmberFlow Codex adapter in $TARGET_ROOT"
