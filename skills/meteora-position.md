---
name: meteora-position
version: 1
custody: T0
summary: Read Meteora DLMM positions for a wallet, return per-position status with IL and range state.
---

# Meteora DLMM Position Reader

Read-only skill. The agent **does not** sign or build transactions here — that is
`meteora-claim` / `meteora-rebalance`. This skill returns a structured summary the
agent can paste into Telegram.

## When to use

Triggered by:

- The `daily-report` cron SOP
- The `range-monitor` cron SOP (out-of-range check)
- A user DM containing `report` or `#<position_id>`
- The other skills (`meteora-claim`, `meteora-rebalance`) calling this to fetch state

## What to fetch

Three sources, in this order. Cap output to **~200 tokens per position** to avoid
flooding the context window (this is a real cost on every call).

### 1. Wallet's positions (Solana RPC)

```
POST ${SOLANA_RPC_URL}
{ "jsonrpc":"2.0","id":1,"method":"getProgramAccounts",
  "params":[
    "${DLMM_PROGRAM}",
    { "encoding":"base64",
      "filters":[
        { "dataSize": <POSITION_ACCOUNT_SIZE> },
        { "memcmp": { "offset": <POSITION_OWNER_OFFSET>,
                      "bytes":"${WALLET_PUBKEY}" } }
      ]
    }
  ] }
```

`POSITION_OWNER_OFFSET` is 40 (8-byte discriminator + 32-byte lb_pair). If your
RPC rejects the query, drop the `dataSize` filter and rely on the owner
`memcmp` alone.

**Empty results are valid.** If `getProgramAccounts` returns no accounts (or DAS
returns no assets), the wallet simply has no DLMM positions — report
`No DLMM positions for this wallet` honestly. Do not treat an empty result as
an error, and never invent positions.

**If you can't decode the borsh locally**, use Helius DAS as a fallback:

```
POST https://mainnet.helius-rpc.com
{ "jsonrpc":"2.0","id":1,"method":"getAssetsByOwner",
  "params":{ "ownerAddress":"${WALLET_PUBKEY}",
             "page":1,"limit":100,
             "displayOptions":{ "showFungible":true } } }
```

then filter to assets whose `content.metadata.name` matches the DLMM position NFT
pattern (collection = DLMM Position).

### 2. Pool state (Meteora REST)

For each position's `pool_address`:

```
GET ${METEORA_API}/pair/${pool_address}
```

Returns: `bin_step`, `active_id`, `active_bin_price`, `token_x`, `token_y`,
24h volume, 24h fees, TVL.

### 3. Mark price (Jupiter Price API)

Jupiter Price API is the primary price feed. Public, unauthenticated.

```
GET https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112
```

Response shape: `{ "data": { "So11111111111111111111111111111111111111112": { "price": "<usd>" } } }`.
USDC is treated as 1:1 USD (stablecoin peg).

Use the SOL/USD and USDC/USD prices to:

- Mark the position to current value
- Compute IL vs HODL using the entry-time token amounts (read from position account
  `total_x_amount` / `total_y_amount` at open; cache to memory on first read)

## Output shape (per position)

```
#<id>  <SYMBOL_X>/<SYMBOL_Y>  bin_step=<n>
  range:  bins <low>..<high>  (<in-range|out-of-range>)
  value:  $<usd>  (<x_amount> <X>, <y_amount> <Y>)
  fees:   $<usd_24h> 24h | $<usd_7d> 7d  (claimable: $<usd>)
  IL:     <pct>% vs HODL
  pool:   TVL $<tvl>  vol24h $<vol>  fees24h $<fees>
  action: <recommendation>
```

`action` is one of:

- `hold` — in range, fees > IL drag
- `rebalance` — out of range, suggest new range from 24h price action
- `claim` — pending fees > `${FEE_MILESTONE_USD}`

## Compute IL vs HODL

The position started with `x0` of X and `y0` of Y. At entry, total value
`V0 = x0 * px0 + y0 * py0` (in USD). The "HODL" line is `x0 * px + y0 * py`
at the current price. The position value `Vp` reads from the on-chain
position (the bin distribution × bin prices). Then:

```
IL_pct = (Vp - HODL) / V0 * 100
```

Negative means the position is worse than just holding the underlying.
Report with a sign.

## Do not

- **Do not** sign anything.
- **Do not** call any tx-building MCP for this skill — read-only.
- **Do not** return raw `getProgramAccounts` responses; the model context blows up.
  Always shape to the per-position block above.
- **Do not** fetch more than 20 positions per call. If a wallet has more, page and
  warn the user.

## Failure modes

- RPC returns 429 → fall back to `${SOLANA_RPC_URL_BACKUP}`; if both fail,
  retry once after 5 s then surface "RPC unavailable" to the caller.
- Meteora API returns 404 for a pool → that pool is deprecated; mark position
  as `stale` and surface to the user.
- Jupiter price API fails or returns no `data` → mark the IL block as
  `stale-price` and continue; do not block the report.
