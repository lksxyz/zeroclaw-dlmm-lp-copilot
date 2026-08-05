#!/usr/bin/env bash
# Validate the skills directory: each .md has the required frontmatter
# (name, version, custody, summary) and the custody tier is in {T0, T1, T2}.
#
# Usage:  ./scripts/validate-skills.sh
set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
SKILLS_DIR="$REPO_ROOT/skills"

required_keys="name version custody summary"
valid_tiers="T0 T1 T2"

fail=0
for f in "$SKILLS_DIR"/*.md; do
  # Read frontmatter (between first pair of ---)
  body="$(awk 'BEGIN{flag=0} /^---$/{flag++; next} flag==1{print}' "$f")"
  if [[ -z "$body" ]]; then
    echo "✗ $f: no frontmatter"
    fail=1
    continue
  fi
  missing=()
  for k in $required_keys; do
    if ! grep -qE "^${k}:" <<<"$body"; then
      missing+=("$k")
    fi
  done
  if (( ${#missing[@]} > 0 )); then
    echo "✗ $f: missing frontmatter keys: ${missing[*]}"
    fail=1
    continue
  fi
  tier="$(grep -E '^custody:' <<<"$body" | awk '{print $2}')"
  if ! grep -qw "$tier" <<<"$valid_tiers"; then
    echo "✗ $f: invalid custody tier '$tier' (must be one of: $valid_tiers)"
    fail=1
    continue
  fi
  echo "✓ $f ($tier)"
done

if (( fail )); then
  echo
  echo "skill validation: FAILED"
  exit 1
fi
echo
echo "skill validation: OK"
