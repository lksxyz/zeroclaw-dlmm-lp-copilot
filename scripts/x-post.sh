#!/usr/bin/env bash
# Helper to format a BUILD_LOG.md entry for X.
# Usage: ./scripts/x-post.sh "<entry text>"
# Output: prints the post; copies to clipboard if xclip / wl-copy / pbcopy is
# available. Best-effort — printing is the source of truth.

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <entry text>" >&2
  exit 1
fi

post="$1"

# Hard cap (X limit is 280 chars). Trim with ellipsis if longer.
if (( ${#post} > 280 )); then
  post="${post:0:277}…"
fi

echo "$post"

# Best-effort clipboard copy.
if command -v wl-copy >/dev/null 2>&1; then
  printf '%s' "$post" | wl-copy && echo "→ copied (wl-copy)" >&2
elif command -v xclip >/dev/null 2>&1; then
  printf '%s' "$post" | xclip -selection clipboard && echo "→ copied (xclip)" >&2
elif command -v pbcopy >/dev/null 2>&1; then
  printf '%s' "$post" | pbcopy && echo "→ copied (pbcopy)" >&2
fi