# DLMM Range Monitor

## Steps

1. **Check positions** — Read the `meteora-position` skill with `read_skill`, then follow its instructions exactly to fetch every DLMM position of wallet `<<WALLET_PUBKEY>>`. Substitute these literal values for the skill's `${...}` placeholders:
   - `${SOLANA_RPC_URL}` = `<<RPC_URL>>`
   - `${SOLANA_RPC_URL_BACKUP}` = `<<RPC_URL_BACKUP>>` (leave empty if none)
   - `${DLMM_PROGRAM}` = `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSK9q8Mfev5Rq` (mainnet)
   - `${METEORA_API}` = `https://dlmm-api.meteora.ag`
   - `${SWITCHBOARD_CROSSBAR}` = `https://crossbar.switchboard-oracle.xyz` (feeds `sol_usd`, `usdc_usd`)
   - `${WALLET_PUBKEY}` = `<<WALLET_PUBKEY>>`
   - `${FEE_MILESTONE_USD}` = `<<FEE_MILESTONE_USD>>`
   Return the per-position blocks (each with its `status`, `il_pct`, `claimable_usd`) as your step output.

2. **Alert if needed** — Evaluate step 1's output against the thresholds in the `meteora-position` skill: a position is out-of-range (`status = out-of-range`), IL worse than `<<IL_ALERT_PCT>>`%, or claimable fees ≥ `<<FEE_MILESTONE_USD>>` USD. If **any** position triggers a condition, send ONE Telegram message per triggered condition with `send_message_to_peer` (channel `telegram.<<CHANNEL_ALIAS>>`, target `<<TARGET>>`) using the urgent alert format from the skill (out-of-range / IL / fees). If nothing triggers, do not send anything — complete the run with output `all clear`.
   - tools: send_message_to_peer
