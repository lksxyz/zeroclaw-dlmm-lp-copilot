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
    "providers.models.openrouter",
    "risk_profiles",
    "channels.telegram",
    "agents",
    "skills.meteora",
    "sops.triggers",
    "memory",
]

# ZeroClaw 0.8.4 requires schema_version. Refuse older shapes so an operator
# copying an old gist fails fast instead of at daemon startup.
if "schema_version" not in cfg:
    print("✗ schema_version missing (required by ZeroClaw 0.8.4)")
    sys.exit(1)
if cfg["schema_version"] < 3:
    print(f"✗ schema_version={cfg['schema_version']} is below the 0.8.4 minimum (3)")
    sys.exit(1)
print(f"✓ schema_version={cfg['schema_version']}")

# The `dlmm` risk profile must use excluded_tools (deny-by-default), not just
# auto_approve — the bounty scoring penalises "approval-gated" alone because
# Telegram approvals are missable.
risk = cfg.get("risk_profiles", {}).get("dlmm", {})
if not risk.get("excluded_tools"):
    print("✗ risk_profiles.dlmm has no excluded_tools — deny-by-default is required")
    sys.exit(1)
forbidden_required = ["content_search", "file_read", "web_fetch"]
missing = [t for t in forbidden_required if t not in risk["excluded_tools"]]
if missing:
    print(f"✗ risk_profiles.dlmm.excluded_tools missing: {', '.join(missing)}")
    sys.exit(1)
print(f"✓ risk_profiles.dlmm denies {len(risk['excluded_tools'])} tools")

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
