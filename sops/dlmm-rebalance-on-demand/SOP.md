# On-demand Rebalance

## Steps

1. **Parse id** — Extract the position id from the trigger message (`rebalance #529630` → `529630`). If absent, reply `usage: rebalance #<position_id>` and stop.

2. **Fetch position + suggest range** — Read the `meteora-position` skill with `read_skill` and follow its instructions exactly to fetch the named position's current state plus 24h price action (so the SOP can suggest a new bin range).
   - `${SOLANA_RPC_URL}` = `<<RPC_URL>>`
   - `${DLMM_PROGRAM}` = `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSK9q8Mfev5Rq` (mainnet)
   - `${METEORA_API}` = `https://dlmm-api.meteora.ag`
   - Price feed: `GET https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112`
   - `${WALLET_PUBKEY}` = `<<WALLET_PUBKEY>>`
   - tools: read_skill, http_request

3. **Build rebalance Action URL** — Read the `meteora-rebalance` skill with `read_skill` and follow it exactly to build the unsigned removeLiquidity + addLiquidity pair (with durable nonce) and return the Solana Action URL.
   - tools: read_skill

4. **Send to operator** — DM the operator the rebalance preview:
   - channel: `telegram.<<CHANNEL_ALIAS>>`
   - target: `<<TARGET>>`
   - message: the prepared text from step 3 (includes the suggested range and the Action URL)
   - tools: send_message_to_peer

## Critical

The agent does NOT sign anything. The Action URL is the signing surface;
the user's wallet holds the keys. Durable nonces (`<<NONCE_ACCOUNT>>`)
let the prepared transaction outlive the ~90 s blockhash window while
the operator is reviewing. Do NOT improvise with `memory_recall`,
`content_search`, or local file searches — all reads go through
`http_request` against `<<RPC_URL>>`.