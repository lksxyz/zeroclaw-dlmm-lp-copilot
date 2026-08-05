# Daily DLMM Position Report

## Steps

1. **Fetch positions** — Read the `meteora-position` skill with `read_skill`, then follow its instructions exactly to fetch every DLMM position of wallet `<<WALLET_PUBKEY>>`. Substitute these literal values for the skill's `${...}` placeholders:
   - `${SOLANA_RPC_URL}` = `<<RPC_URL>>`
   - `${SOLANA_RPC_URL_BACKUP}` = `<<RPC_URL_BACKUP>>` (leave empty if none)
   - `${DLMM_PROGRAM}` = `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSK9q8Mfev5Rq` (mainnet)
   - `${METEORA_API}` = `https://dlmm-api.meteora.ag`
   - Price feed: `GET https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112` for SOL/USD; USDC = 1:1 USD
   - `${WALLET_PUBKEY}` = `<<WALLET_PUBKEY>>`
   - `${FEE_MILESTONE_USD}` = `<<FEE_MILESTONE_USD>>`
   An empty account list means the wallet has no DLMM positions — that is a valid outcome. Report `No DLMM positions for this wallet` honestly; it is not an error and you must not invent positions.
   Return the complete per-position block summary (range, value, fees, IL, action) as your step output.
   - tools: read_skill, http_request

2. **Format report** — Read the `meteora-report` skill with `read_skill` and follow it exactly to render the positions from step 1 into the final Telegram daily report (value, fees 24h, range status, IL vs HODL, action per position). Your step output is the finished report text.
   - tools: read_skill

3. **Send report** — Deliver the report with `send_message_to_peer`:
   - channel: `telegram.<<CHANNEL_ALIAS>>`
   - target: `<<TARGET>>`
   - message: the report text from step 2
   - tools: send_message_to_peer
