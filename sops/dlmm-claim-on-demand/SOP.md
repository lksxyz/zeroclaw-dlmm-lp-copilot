# On-demand Fee Claim

## Steps

1. **Parse id** — Extract the position id from the trigger message (`claim #529630` → `529630`). If absent, reply `usage: claim #<position_id>` and stop.

2. **Fetch position state** — Read the `meteora-position` skill with `read_skill` and follow its instructions exactly to fetch the named position's current state (fees, owner, bin range). Substitute these literal values for the skill's `${...}` placeholders:
   - `${SOLANA_RPC_URL}` = `<<RPC_URL>>`
   - `${DLMM_PROGRAM}` = `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSK9q8Mfev5Rq` (mainnet)
   - `${WALLET_PUBKEY}` = `<<WALLET_PUBKEY>>`
   - `${METEORA_API}` = `https://dlmm-api.meteora.ag`
   - tools: read_skill, http_request

3. **Build claim Action URL** — Read the `meteora-claim` skill with `read_skill` and follow its instructions exactly to construct the unsigned claim transaction and return the Solana Action URL.
   - tools: read_skill

4. **Send to operator** — DM the operator the claim preview:
   - channel: `telegram.<<CHANNEL_ALIAS>>`
   - target: `<<TARGET>>`
   - message: the prepared text from step 3 (includes the Action URL)
   - tools: send_message_to_peer

## Critical

The agent does NOT sign anything. The Action URL is the signing surface.
Do NOT improvise with `memory_recall`, `content_search`, or local file
searches for position data — all reads go through `http_request` against
`<<RPC_URL>>`.