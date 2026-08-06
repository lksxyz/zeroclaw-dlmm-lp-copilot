---
name: meteora-rebalance
version: 4
custody: T1
summary: Build atomic remove+add rebalance tx in-wasm (dlmm_builder), return action URL
---

# meteora-rebalance

Build the atomic rebalance (AdvanceNonce → removeLiquidity 100% →
addLiquidityByStrategy) with the `dlmm_builder` plugin in-wasm. Return the
`solana-action:` URL. User signs.

Tools: `dlmm_builder`, `dlmm_reader`, `send_message_to_peer`, `read_skill`,
`memory_recall`. `http_request` is DENIED.

## Trigger

DM `rebalance #<id>` / `rebalance <id>` / `rebalance` (last-reported). Execute now.

## Steps

### 1. Read the position

`dlmm_reader {"mode":"report","position_ids":["<id>"]}` — get `range`,
`active_bin_id`, `owner_match` (must be true). Also fetch `bin_step` for the
range scaling below.

### 2. Propose new range

Center on `active_id`. Scale by bin_step:
- Default ±5% → `range_bins = 0.05 / (bin_step / 10000)`
- `rebalance #<id> wide` → ±10%
- `rebalance #<id> tight` → ±2%

`new_low = active_id - range_bins`, `new_high = active_id + range_bins`.

### 3. Build atomic tx

```
dlmm_builder {"mode":"rebalance","position_id":"<id>","nonce_address":"${NONCE_ACCOUNT}","new_low":<lo>,"new_high":<hi>,"label":"Rebalance"}
```

Plugin validates mechanically (owner match, liquidity > 0, low < high, range
contains the live active bin, bin-array indexes inside the default bitmap),
derives re-deposit amounts from the position's shares × bin reserves (fetched
in-wasm), and returns `{"action_url":"solana-action:...","instructions":3,...}`.

### 4. Reply

```
#<id> rebalance · bins <old_lo>..<old_hi> → <new_lo>..<new_hi>

Tap to sign: <action_url>
```

### 5. Confirm settlement

Poll every 10s for ≤60s: `dlmm_reader {"mode":"status","nonce_address":"${NONCE_ACCOUNT}","previous_nonce_hash":"<recent_blockhash from step 3>"}`.

`settled: true` → nonce advanced → tx landed:

```
✓ #<id> rebalanced · <new_lo>..<new_hi>
```

## Rules

- Never sign. Never submit. Never broadcast.
- Never reuse the nonce while a tx is unsettled.
- Range never > ±15% without `wide` override.
- Plugin error → reply verbatim, no retry.
