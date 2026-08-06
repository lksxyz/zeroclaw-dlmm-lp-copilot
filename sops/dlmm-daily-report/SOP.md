# Daily DLMM Position Report

## Steps

1. **Discover + report positions** — `dlmm_reader` (in-wasm; RPC/owner come
   from the plugin config section, not SOP vars):
   - `{"mode":"positions"}` → ids
   - `{"mode":"report","position_ids":[...]}` → ranges, active bin, claimable
   Empty = valid. "No DLMM positions" is not an error — report the empty state.
   Tools: dlmm_reader

2. **Format** — `read_skill meteora-report`, render the step-1 output with the
   Telegram template. Report only what `dlmm_reader` returned — never invent
   claimable or range values.
   Tools: read_skill

3. **Send** — `send_message_to_peer` → `telegram.<<CHANNEL_ALIAS>>` → `<<TARGET>>`
   Tools: send_message_to_peer
