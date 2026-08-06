---
name: meteora-position
version: 4
custody: T0
summary: Discover DLMM positions and build per-position summaries via dlmm_reader
---

# meteora-position

Tools: `dlmm_reader` only. `http_request` is DENIED (excluded_tools) — all RPC
traffic runs in the plugin under a host-injected `__config`. Avoid
`web_fetch`, `web_search_tool`, `browser`.

## Trigger

DM `^(report|claim|rebalance)\b` or cron `dlmm-daily-report`/`dlmm-range-monitor`. Execute now. No questions.

## Steps

### 1. Discover positions

```
dlmm_reader {"mode":"positions"}
```

Returns `{"positions":[{"id","lb_pair","range":[lo,hi]}]}` — the plugin fetches
getProgramAccounts (owner memcmp) in-wasm. Empty list → "No DLMM positions for
this wallet." Stop here.

### 2. Full reports

```
dlmm_reader {"mode":"report","position_ids":["<id1>","<id2>",...]}
```

Per position: `range`, `active_bin_id`, `in_range`, `owner_match`,
`claimable.{x,y,usd_approx}`, `liquidity_shares`. Prices (Jupiter) and mint
decimals are fetched in-wasm — no host-fed USD.

`owner_match: false` → flag for review, never act.

## Output per position

```
#<id> bin <lo>..<hi> · <in|out> · active=<n>
  claimable: <x> X + <y> Y ≈ $<usd>
  owner: <match|MISMATCH> · shares: <liquidity_shares>
  action: hold|rebalance|claim|review
```

Action: out-of-range → `rebalance` · owner mismatch → `review` (never act) · claimable ≥ `${FEE_MILESTONE_USD}` → `claim` · else → `hold`.

The plugin reports only what it decoded on-chain: ranges, claimable fees
(raw + human + USD), owner match, liquidity shares. It does NOT fabricate
position value or IL — those need bin arrays and are out of report scope.

## Limits

≤20 positions. No raw RPC output. Read-only (no sign, no tx build).

## Failures

- Plugin error → reply the error verbatim, stop
- Report missing a position → mark stale, continue
