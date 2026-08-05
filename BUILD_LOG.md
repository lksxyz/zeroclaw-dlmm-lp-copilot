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
not gated). Pinned `wit-bindgen = 0.46.0` and added the assumptions note to
`wit/world.wit`. Ready to post.
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