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

3. **Settlement cleanup** — For each pool slot with a pending marker
   (`pending_tx_<nonce>` with the recorded `previous_nonce_hash`), check:
   `dlmm_reader {"mode":"status","nonce_address":"<nonce>","previous_nonce_hash":"<recorded>"}`
   - `settled: true` → clear the pending marker — that pool slot is free for reuse
   - `settled: false` → keep the marker; do NOT propose a new tx on that slot
   Tools: dlmm_reader, memory_recall

   The pool size is `__config.nonce_addresses` (host-injected, see
   `DEPLOY.md` §7). SOPs and skills track one in-flight tx per slot, so
   parallel pending approvals can coexist.
