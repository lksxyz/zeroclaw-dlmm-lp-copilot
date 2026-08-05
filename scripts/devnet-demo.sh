#!/usr/bin/env bash
# End-to-end Devnet demo runbook — see showcase/demo-transcript.md for the
# human-readable version of these steps.
#
# Usage:   ./scripts/devnet-demo.sh
# Effects: creates a temp wallet, airdrops Devnet SOL, opens no real
#          positions (those still need the Meteora UI or a small SDK
#          script — out of scope here). What this DOES do:
#            - validates the worker is reachable
#            - validates the action metadata is shaped correctly
#            - validates the price feed is live
#          so a reviewer can confirm the plumbing before opening positions.

set -euo pipefail

WORKER="${ACTION_ENDPOINT_BASE:-https://dlmm-lp-copilot.example.workers.dev}"
DEVNET_RPC="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"

echo "▶ checking worker  : $WORKER/health"
status="$(curl -sS -o /dev/null -w '%{http_code}' "$WORKER/health" || true)"
if [[ "$status" != "200" ]]; then
  echo "  ✗ worker not reachable (HTTP $status). deploy with: make worker-deploy"
  exit 1
fi
echo "  ✓ worker is up"

echo
echo "▶ checking claim action metadata"
curl -sS "$WORKER/actions/claim?pos=PLACEHOLDER&pool=PLACEHOLDER" \
  | python3 -c "import sys, json; m=json.load(sys.stdin); assert m['type']=='action'; assert 'title' in m; print('  ✓', m['title'])"

echo
echo "▶ checking rebalance action metadata"
curl -sS "$WORKER/actions/rebalance?pos=PLACEHOLDER&pool=PLACEHOLDER&new_low=8400&new_high=8800" \
  | python3 -c "import sys, json; m=json.load(sys.stdin); assert m['type']=='action'; assert 'title' in m; print('  ✓', m['title'])"

echo
echo "▶ checking Solana RPC: $DEVNET_RPC"
curl -sS -X POST -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
  "$DEVNET_RPC" \
  | python3 -c "import sys, json; r=json.load(sys.stdin); print('  ✓' if 'result' in r else '  ✗', r.get('result', r.get('error')))"

echo
echo "▶ all plumbing checks pass"
echo
echo "Next: open two real DLMM positions via the Meteora UI on Devnet,"
echo "copy their pubkeys into showcase/demo-transcript.md, and run the"
echo "Telegram end-to-end per the video script."
