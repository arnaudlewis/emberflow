#!/usr/bin/env bash
set -euo pipefail

payload="$(cat)"

PYTHONPAYLOAD="$payload" python3 - <<'PY'
import json
import os
import re
import sys

raw = os.environ.get("PYTHONPAYLOAD", "")
try:
    payload = json.loads(raw)
except Exception:
    raise SystemExit(0)

command = payload.get("tool_input", {}).get("command")
if not isinstance(command, str) or not command.strip():
    raise SystemExit(0)

if re.search(r"\brm\b(?:\s+-[^\s]+)*\s+(\./)?\.emberflow(?:\s|$)", command):
    print("BLOCKED: remove EmberFlow state through EmberFlow, not rm -rf .emberflow", file=sys.stderr)
    raise SystemExit(2)

if re.search(r"\bsqlite3\b.*\.emberflow/emberflow\.db", command):
    print("BLOCKED: direct sqlite access bypasses EmberFlow canonical rules", file=sys.stderr)
    raise SystemExit(2)

if ".emberflow/" in command and re.search(r"(?:^|[\s;|&])(?:>|>>|tee\b|cp\b|mv\b|sed\s+-i\b|perl\s+-0pi\b)", command):
    print("BLOCKED: direct shell writes into .emberflow projections are not allowed", file=sys.stderr)
    raise SystemExit(2)

raise SystemExit(0)
PY
