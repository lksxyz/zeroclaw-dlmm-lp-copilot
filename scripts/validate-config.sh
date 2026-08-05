#!/usr/bin/env bash
# Validate the ZeroClaw config: TOML parse + required sections present.
#
# Usage:  ./scripts/validate-config.sh
# Exits non-zero on any failure.
set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG="$REPO_ROOT/config.example.toml"

if [[ ! -f "$CONFIG" ]]; then
  echo "✗ config.example.toml missing at $CONFIG" >&2
  exit 1
fi

# 1. TOML parse via python (ubiquitous)
python3 - "$CONFIG" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    cfg = tomllib.load(f)

required_sections = [
    "providers.models.anthropic",
    "risk_profiles",
    "channels.telegram",
    "agents",
    "skills.meteora",
    "sops.triggers",
    "memory",
]

def has(d, dotted):
    cur = d
    for part in dotted.split("."):
        if not isinstance(cur, dict) or part not in cur:
            return False
        cur = cur[part]
    return True

missing = [s for s in required_sections if not has(cfg, s)]
if missing:
    print(f"✗ missing sections: {', '.join(missing)}")
    sys.exit(1)
else:
    print(f"✓ all {len(required_sections)} required sections present")
PY

# 2. No real secrets in the example
if grep -qE 'api_key = "[A-Za-z0-9]{20,}"' "$CONFIG"; then
  echo "✗ config.example.toml contains what looks like an inline api_key" >&2
  exit 1
fi
if grep -qE 'bot_token = "[0-9]{8,}:[A-Za-z0-9_-]{20,}"' "$CONFIG"; then
  echo "✗ config.example.toml contains what looks like a real bot_token" >&2
  exit 1
fi
echo "✓ no inline secrets in config.example.toml"

# 3. All SOP TOMLs parse
for f in "$REPO_ROOT"/sops/dlmm-*/SOP.toml; do
  python3 -c "import sys, tomllib; tomllib.load(open(sys.argv[1], 'rb'))" "$f" \
    || { echo "✗ $f does not parse as TOML" >&2; exit 1; }
  echo "✓ $f"
done

echo
echo "config validation: OK"
