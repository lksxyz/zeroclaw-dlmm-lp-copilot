---
name: meteora-position
version: 3
custody: T0
summary: Fetch DLMM positions and compute impermanent loss vs HODL
---

# meteora-position

Tools: use `http_request` for all external calls. Avoid `web_fetch`, `web_search_tool`, `browser` — results are unreliable.

## Trigger

DM `^(report|claim|rebalance)\b` or cron `dlmm-daily-report`/`dlmm-range-monitor`. Execute now. No questions.

## Steps

### 1. Get SOL price

```
http_request GET https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112
```

Parse: `data["So11111111111111111111111111111111111111112"].price`. USDC = 1.0.

### 2. Get positions from RPC

```
http_request POST ${SOLANA_RPC_URL}
{"jsonrpc":"2.0","id":1,"method":"getProgramAccounts",
 "params":["${DLMM_PROGRAM}",{"encoding":"base64","filters":[
   {"dataSize":<POSITION_ACCOUNT_SIZE>},
   {"memcmp":{"offset":40,"bytes":"${WALLET_PUBKEY}"}}
 ]}]}
```

Offset 40 = 8 discriminator + 32 lb_pair. Drop dataSize if RPC rejects.

Empty result = "No DLMM positions for this wallet." Stop here.

### 3. Get pool state per position

For each position found, extract `lb_pair` (first 32 bytes after discriminator, base58 encode it):

```
http_request GET https://dlmm-api.meteora.ag/pair/<pool_address>
```

Returns: `bin_step`, `active_id`, TVL, vol24h, fees24h, `token_x.symbol`, `token_y.symbol`.

## Output per position

```
#<id> <X>/<Y> bin_step=<n>
  range: bins <lo>..<hi> (in|out)
  value: $<usd> (<x_amt> X, <y_amt> Y)
  fees: $<24h> 24h | $<7d> 7d  claim: $<claimable>
  IL: <pct>% vs HODL
  action: hold|rebalance|claim|review
```

Action: out-of-range → `rebalance` · IL < `-${IL_ALERT_PCT}%` → `review` · claimable ≥ `${FEE_MILESTONE_USD}` → `claim` · else → `hold`.

## IL formula

```
V0 = memory.baseline_value_<id> (write on first read, show -- if missing)
HODL = entry_x * sol_price + entry_y * 1.0
Vp = total_x_amount/1e9 * sol_price + total_y_amount/1e6 * 1.0
IL% = (Vp - HODL) / V0 * 100
```

## Limits

≤20 positions. No raw RPC output. Read-only (no sign, no tx build).

## Failures

- RPC 429 → retry with `${SOLANA_RPC_URL_BACKUP}`. Both fail → "RPC unavailable", stop
- https://dlmm-api.meteora.ag 404 → mark pool stale, continue
- https://api.jup.ag no data → mark price stale, continue
