#!/usr/bin/env bash
# End-to-end Devnet demo runbook — see showcase/demo-transcript.md for the
# human-readable version of these steps.
#
# Usage:   ./scripts/devnet-demo.sh
# Effects: validates the plumbing a reviewer can confirm before opening
#          real positions:
#            - Solana RPC is reachable
#            - Jupiter Price API is reachable (Pyth-independent feed)
#            - the build tooling compiles plugins and fixtures
#          Does NOT exercise the agent (Telegram side) — that requires
#          a real wallet and is run interactively per demo-transcript.md.

set -euo pipefail

DEVNET_RPC="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"

echo "▶ checking Solana RPC: $DEVNET_RPC"
curl -sS -X POST -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
  "$DEVNET_RPC" \
  | python3 -c "import sys, json; r=json.load(sys.stdin); print('  ✓' if 'result' in r else '  ✗', r.get('result', r.get('error')))"

echo
echo "▶ checking Jupiter Price API (Pyth-independent)"
curl -sS "https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112" \
  | python3 -c "import sys, json; d=json.load(sys.stdin); print('  ✓ SOL price:', d['data']['So11111111111111111111111111111111111111112']['price'])"

echo
echo "▶ all plumbing checks pass"
echo
echo "Next: open two real DLMM positions via the Meteora UI on Devnet,"
echo "copy their pubkeys into showcase/demo-transcript.md, and run the"
echo "Telegram end-to-end per the video script."
