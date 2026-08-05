---
name: meteora-position
version: 2
custody: T0
summary: Fetch all DLMM positions for a wallet. Return per-position status with IL, fees, range state.
---

# meteora-position

Tool: `http_request`. Never `web_fetch`, `browser`, `web_search_tool`, `weather`.

## Trigger

- DM: `^(report|claim|rebalance)\b` (case-insensitive). Execute immediately, no questions.
- Cron: `dlmm-daily-report`, `dlmm-range-monitor`
- Called by: `meteora-claim`, `meteora-rebalance`

## Steps

### 1. Fetch positions (RPC)

```
http_request POST ${SOLANA_RPC_URL}
{"jsonrpc":"2.0","id":1,"method":"getProgramAccounts",
 "params":["${DLMM_PROGRAM}", {"encoding":"base64","filters":[
   {"dataSize":<POSITION_ACCOUNT_SIZE>},
   {"memcmp":{"offset":40,"bytes":"${WALLET_PUBKEY}"}}
 ]}]}
```

offset=40 = 8 (discriminator) + 32 (lb_pair). Drop `dataSize` if RPC rejects.

Empty = valid. Reply "No DLMM positions for this wallet." Never invent.

### 2. Pool state (Meteora)

Per position:

```
http_request GET ${METEORA_API}/pair/<pool_address>
```

→ `bin_step`, `active_id`, TVL, vol24h, fees24h.

### 3. SOL price (Jupiter)

```
http_request GET https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112
```

→ `data["So11111111111111111111111111111111111111112"].price`. USDC = 1.0.

## Output

Per position, ≤ 200 tokens:

```
#<id> <X>/<Y> bin_step=<n>
  range: bins <lo>..<hi> (in|out)
  value: $<usd> (<x_amt> X, <y_amt> Y)
  fees: $<24h> 24h | $<7d> 7d  claim: $<claimable>
  IL: <pct>% vs HODL
  action: hold|rebalance|claim|review
```

Action: out-of-range → `rebalance` · IL < `-${IL_ALERT_PCT}%` → `review` · claimable ≥ `${FEE_MILESTONE_USD}` → `claim` · else → `hold`

## IL

```
V0 = entry value   (memory.baseline_value_<id>, write on first read)
HODL = entry amounts × current price
Vp = on-chain amounts × current price
IL% = (Vp − HODL) / V0 × 100
```

Negative = worse than HODL. No baseline → write V0 now, show `--` for delta.

## Limits

- ≤ 20 positions. More → page + warn
- No raw RPC output — shape per block
- Read-only. Never sign, never build tx

## Failures

- RPC 429 → `${SOLANA_RPC_URL_BACKUP}`. Both fail → "RPC unavailable", stop
- Meteora 404 → mark `stale`, continue
- Jupiter no data → mark `stale-price`, continue
