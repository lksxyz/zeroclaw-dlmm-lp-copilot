# On-demand DLMM Position Report

## Steps

1. **Fetch positions** — Read the `meteora-position` skill with `read_skill`, then follow its instructions exactly to fetch every DLMM position of wallet `<<WALLET_PUBKEY>>`. Substitute these literal values for the skill's `${...}` placeholders:
   - `${SOLANA_RPC_URL}` = `<<RPC_URL>>`
   - `${SOLANA_RPC_URL_BACKUP}` = `<<RPC_URL_BACKUP>>` (leave empty if none)
   - `${DLMM_PROGRAM}` = `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSK9q8Mfev5Rq` (mainnet)
   - `${METEORA_API}` = `https://dlmm-api.meteora.ag`
   - Price feed: `GET https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112` for SOL/USD; USDC = 1:1 USD
   - `${WALLET_PUBKEY}` = `<<WALLET_PUBKEY>>`
   - `${FEE_MILESTONE_USD}` = `<<FEE_MILESTONE_USD>>`

   If the DM included a position id (`report #529630`), filter step 1's
   output to only that id before passing to step 2. If the id is unknown,
   reply with `unknown position #<id>` and stop.
   - tools: read_skill, http_request

2. **Format report** — Read the `meteora-report` skill with `read_skill` and follow it exactly to render the positions from step 1 into the Telegram report (the same format as the daily cron).
   - tools: read_skill

3. **Send report** — Deliver the report with `send_message_to_peer`:
   - channel: `telegram.<<CHANNEL_ALIAS>>`
   - target: `<<TARGET>>`
   - message: the report text from step 2
   - tools: send_message_to_peer

## Critical

This SOP runs only via `read_skill` and `http_request`. Do NOT improvise with
`memory_recall`, `content_search`, `glob_search`, or any local file search —
positions are on-chain, not in the workspace. If step 1 returns `No DLMM
positions for this wallet`, that is the truth: complete step 2 with the
empty-state format and send it. Do not retry, do not invent positions.