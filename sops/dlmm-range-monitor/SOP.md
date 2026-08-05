# DLMM Range Monitor

## Steps

1. **Check positions** — `read_skill meteora-position`, execute with:
   - `${SOLANA_RPC_URL}` = `<<RPC_URL>>`
   - `${SOLANA_RPC_URL_BACKUP}` = `<<RPC_URL_BACKUP>>`
   - `${DLMM_PROGRAM}` = `<<DLMM_PROGRAM>>`
   - `${WALLET_PUBKEY}` = `<<WALLET_PUBKEY>>`
   - ${FEE_MILESTONE_USD} = `<<FEE_MILESTONE_USD>>`
   - ${IL_ALERT_PCT} = `<<IL_ALERT_PCT>>`
   - Meteora: `http_request GET https://dlmm-api.meteora.ag/pair/<pool_address>`
   - Price: `http_request GET https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112`
   Empty = valid, output `all clear`, stop.
   Tools: read_skill, http_request

2. **Alert** — If any position is out-of-range, IL < -`<<IL_ALERT_PCT>>`%, or claimable ≥ `<<FEE_MILESTONE_USD>>`:
   `send_message_to_peer` → `telegram.<<CHANNEL_ALIAS>>` → `<<TARGET>>`
   One message per triggered condition. Format from meteora-report (⚠️ block).
   If nothing triggers → output `all clear`, send nothing.
   Tools: send_message_to_peer
