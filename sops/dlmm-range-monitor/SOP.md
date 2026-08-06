# DLMM Range Monitor

## Steps

1. **Discover + report positions** — `dlmm_reader` (in-wasm; RPC/owner come
   from the plugin config section, not SOP vars):
   - `{"mode":"positions"}` → ids
   - `{"mode":"report","position_ids":[...]}` → ranges, active bin, claimable
   Empty = valid, output `all clear`, stop.
   Tools: dlmm_reader

2. **Alert** — If any position is out-of-range or claimable ≥
   `<<FEE_MILESTONE_USD>>`:
   `send_message_to_peer` → `telegram.<<CHANNEL_ALIAS>>` → `<<TARGET>>`
   One message per triggered condition. Format from meteora-report (⚠️ block).
   If nothing triggers → output `all clear`, send nothing.
   Tools: send_message_to_peer

3. **Settlement cleanup** — If memory has a pending marker
   (`pending_tx_<nonce>` with the recorded `previous_nonce_hash`), check:
   `dlmm_reader {"mode":"status","nonce_address":"<<NONCE_ACCOUNT>>","previous_nonce_hash":"<recorded>"}`
   - `settled: true` → clear the pending marker — a new proposal is safe
   - `settled: false` → keep the marker; do NOT propose a new tx on that nonce
   Tools: dlmm_reader, memory_recall
