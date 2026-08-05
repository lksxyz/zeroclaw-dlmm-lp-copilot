# Daily DLMM Position Report

## Steps

1. **Fetch positions** — `read_skill meteora-position`, execute with:
   - `${SOLANA_RPC_URL}` = `<<RPC_URL>>`
   - `${SOLANA_RPC_URL_BACKUP}` = `<<RPC_URL_BACKUP>>`
   - `${DLMM_PROGRAM}` = `<<DLMM_PROGRAM>>`
   - `${METEORA_API}` = `<<METEORA_API>>`
   - `${WALLET_PUBKEY}` = `<<WALLET_PUBKEY>>`
   - ${FEE_MILESTONE_USD} = `<<FEE_MILESTONE_USD>>`
   - Price: `http_request GET https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112`
   Empty = valid. "No DLMM positions" is not an error.
   Tools: read_skill, http_request

2. **Format** — `read_skill meteora-report`, render positions from step 1.
   Tools: read_skill

3. **Send** — `send_message_to_peer` → `telegram.<<CHANNEL_ALIAS>>` → `<<TARGET>>`
   Tools: send_message_to_peer
