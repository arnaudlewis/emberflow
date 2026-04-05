#!/usr/bin/env bash
set -euo pipefail

payload="$(cat)"

PYTHONPAYLOAD="$payload" python3 - <<'PY'
import json
import os
import sys

raw = os.environ.get("PYTHONPAYLOAD", "")
try:
    payload = json.loads(raw)
except Exception:
    raise SystemExit(0)

tool_input = payload.get("tool_input", {})
path_keys = (
    "file_path",
    "path",
    "target_file",
    "target_path",
    "old_file_path",
    "new_file_path",
)

paths = []
for key in path_keys:
    value = tool_input.get(key)
    if isinstance(value, str):
        paths.append(value)
    elif isinstance(value, list):
        for item in value:
            if isinstance(item, str):
                paths.append(item)

for path in paths:
    normalized = path.replace("\\", "/")
    if normalized.startswith(".emberflow/") or "/.emberflow/" in normalized:
        print(
            "BLOCKED: .emberflow projections are derived views; mutate EmberFlow through MCP instead",
            file=sys.stderr,
        )
        raise SystemExit(2)

raise SystemExit(0)
PY
