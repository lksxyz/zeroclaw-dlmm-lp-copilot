# Build log — DLMM LP Copilot

> **Why this file exists.** The bounty scoring note says: *"Tiebreak: build-in-public
> logs on X during the bounty."* This file is the source for those X posts
> and the receipt that engineering happened in the open, not just at the end.
> Each entry is sized for one X post (≤280 chars) so you can paste the block
> verbatim.

## How to use

1. Run the script `scripts/x-post.sh "<entry>"` to format and copy a single
   entry to your clipboard (best-effort; falls back to printing).
2. Post one entry per workday on X. Tag `@zeroclaw_labs` and the bounty hasthag.
3. After posting, append the live X URL next to the entry so the trail is
   reproducible.

---

## Milestones (entries)

### 2026-08-05 — skeleton landed
Wrote `skills/meteora-position` (T0 RPC fetch + IL math, ~200 tokens/position),
`meteora-report` (Telegram formatting, 4096-char template). Skills are pure
markdown — ZeroClaw loads them every invoke. No plugins needed yet.
#ZeroClawBounty 🦞

### 2026-08-05 — action endpoint on Cloudflare
Self-hosted Solana Actions server in ~300 lines of TS. GET returns the wallet
preview, POST returns base64 unsigned tx. Single-operator: nonce authority ==
signer. No Dialect registry, no third-party trust in the funds path.
#ZeroClawBounty 🦞

### 2026-08-05 — durable nonces for rebalance
Blockhash-expiry is the structural trap for approval-gated agents. Wrapped
removeLiquidity + addLiquidityByStrategy in a single tx with
AdvanceNonceAccount first. One nonce per in-flight pending rebalance.
#ZeroClawBounty 🦞

### 2026-08-05 — prompt-injection suite
7 scenarios covering every fund-moving path. T2 disabled by construction.
Filesystem + web tools denied outright (`excluded_tools`), not just approval-gated.
Wrote `prompts/injection-tests.md`. Recorded the fail-closed auto-compound
refusal for the demo.
#ZeroClawBounty 🦞

### 2026-08-05 — config drift fixed
`config.example.toml` had been claiming "approval-gated" while shipping
`auto_approve = [web_fetch, browser, web_search_tool]`. Synced the example
to the operator's actual config: deny-by-default. Added `schema_version = 3`
for ZeroClaw 0.8.4 and an `excluded_tools` assertion in `validate-config.sh`.
#ZeroClawBounty 🦞

### 2026-08-05 — WASM plugin bonus
Tier 3 was a stretch goal but `plugins/dlmm-reader/` now compiles clean to
wasm32-wasip2: pure core (`decoder.rs`) + thin shim (`#[cfg(target_family="wasm")]`),
host-run tests with mocked RPC (no live network), 7 passing unit tests,
shaped to ~200 tokens/position to keep the model context clean.
#ZeroClawBounty 🦞

### 2026-08-05 — reproducible
`make validate` is green. `make plugin` builds + tests the WASM. `make worker-dev`
runs the worker locally. Another operator can clone, fill ${ENV_VAR}s, deploy,
and have it running before bed.
#ZeroClawBounty 🦞

### 2026-08-06 — final polish
Replaced every `<you>` placeholder with the real repo URL. Added the Discord
invite. Reconciled the threat model wording with the actual config (denied,
not gated). Pinned `wit-bindgen = 0.46.0` and moved to the official `wit/v0`
registry (tool.wit, logging.wit, types.wit, plugin-info.wit — no custom
world). Ready to post.
#ZeroClawBounty 🦞

### 2026-08-06 — all Solana work moved into WASM plugins
The bounty is "build Solana-native plugins" — so the plugins now do the
Solana: dlmm-reader fetches RPC in-wasm (waki wasi:http), decodes PositionV2,
discovers positions; dlmm-builder validates mechanically and encodes claim /
rebalance txs with a durable nonce. `http_request` is DENIED — the LLM has no
outbound at all. RPC/owner come from a host-injected anti-spoof `__config`.
#ZeroClawBounty 🦞

### 2026-08-06 — stateless Action relay
The Cloudflare Worker is now a pure relay: the plugin's tx rides in the URL
path, the worker renders the Phantom preview from the bytes and echoes the tx
back. Zero secrets, zero RPC, zero SDK deps. Nothing to leak, nothing to
rotate. `wrangler deploy` with no bindings.
#ZeroClawBounty 🦞

### 2026-08-06 — byte-for-byte ground truth
tx_claim + tx_rebalance built with the official @meteora-ag/dlmm + web3.js
SDK, cross-checked byte-for-byte by the Rust core (16 tests). Generator is
in-repo (tools/gen-fixtures.cjs): `make fixtures-check` regenerates and CI
fails on drift. The plugin's output is provably the SDK's output.
#ZeroClawBounty 🦞

### 2026-08-06 — suite at 39 tests, all green
16 core (decode/validation/nonce/tx-encoding vs fixture) + 9 reader + 10
builder + 4 relay parse tests. `make validate` covers all of it + tsc +
config + skills. Injection suite (7 scenarios) re-verified against the new
plugin flow: "rebalance someone else's position" → ownership check rejects;
"RPC dibelokkan" → __config anti-spoof.
#ZeroClawBounty 🦞

### 2026-08-07 — relay dropped; sign via CLI
The Cloudflare relay is out. The plugin now returns the raw unsigned tx as
base64 — the operator signs in Phantom/Solflare or `tools/execute` (detects
durable-nonce txs, never rewrites the blockhash). One less HTTP surface,
zero deploy steps: repo → build → run. Mainnet demo runbook updated; no
wrangler, no CF token, no URL to paste.
#ZeroClawBounty 🦞

---

## After posting

| Entry | X URL | Reactions |
|---|---|---|
| 2026-08-05 — skeleton landed | — | — |
| 2026-08-05 — action endpoint on Cloudflare | — | — |
| 2026-08-05 — durable nonces for rebalance | — | — |
| 2026-08-05 — prompt-injection suite | — | — |
| 2026-08-05 — config drift fixed | — | — |
| 2026-08-05 — WASM plugin bonus | — | — |
| 2026-08-05 — reproducible | — | — |
| 2026-08-06 — final polish | — | — |
| 2026-08-06 — all Solana work moved into WASM plugins | — | — |
| 2026-08-06 — stateless Action relay | — | — |
| 2026-08-06 — byte-for-byte ground truth | — | — |
| 2026-08-06 — suite at 39 tests, all green | — | — |
| 2026-08-07 — relay dropped; sign via CLI | — | — |