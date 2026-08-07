---
name: meteora-rebalance
version: 5
custody: T1
summary: Build atomic remove+add rebalance tx in-wasm (dlmm_builder), present the raw unsigned tx (base64) for the operator to sign in any wallet (multi-nonce pool)
---

# meteora-rebalance

Build the atomic rebalance (AdvanceNonce → removeLiquidity 100% →
addLiquidityByStrategy) with the `dlmm_builder` plugin in-wasm. Present the
**raw unsigned transaction (base64)** — the operator signs it in their own
wallet. Agent never signs, never submits.

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
dlmm_builder {"mode":"rebalance","position_id":"<id>","new_low":<lo>,"new_high":<hi>,"label":"Rebalance"}
```

The nonce address comes from `__config.nonce_addresses` (host-injected
pool, anti-spoof). The LLM may hint which pool slot to use via
`args.nonce_address` — the plugin validates the hint is inside the pool.
If the pool is empty, the plugin falls back to the latest blockhash
(demo path; expires ~90 s). Do NOT ask the operator for new nonces —
the operator pre-allocates the pool (`DEPLOY.md` §7).

The plugin validates mechanically (owner match, liquidity > 0, low < high,
range contains the live active bin, bin-array indexes inside the default
bitmap), derives re-deposit amounts from the position's shares × bin reserves
(fetched in-wasm), and returns `{"tx_base64":"...","instructions":3,"recent_blockhash":"<nonce hash>",...}`.

### 4. Reply with the unsigned tx

Reply with the raw `tx_base64` from the plugin output. The operator signs it
in their wallet. Don't include the `action_url`.

```
#<id> rebalance · bins <old_lo>..<old_hi> → <new_lo>..<new_hi>

Unsigned tx (base64) — sign in your wallet:
<tx_base64>
```

Show `tx_base64` in full (own message if Telegram truncates). `recent_blockhash`
is the durable-nonce hash when `__config.nonce_address` is set, so the tx
doesn't expire in 90s; if `instructions` is 2 (no nonce), tell the operator
it expires in ~90s. Echo only the plugin's fields.

### 5. Confirm settlement

Poll every 10s for ≤60s: `dlmm_reader {"mode":"status","nonce_address":"<the pool slot you used in step 3>","previous_nonce_hash":"<recent_blockhash from step 3>"}`.

`settled: true` → nonce advanced → tx landed:

```
✓ #<id> rebalanced · <new_lo>..<new_hi>
```

When `settled: true`, **free the pool slot** in agent memory so the same
nonce can be reused for a future tx.

## Rules

- Never sign. Never submit. Never broadcast.
- Never reuse the same pool slot while a tx is unsettled. Pick a different
  slot for the next concurrent tx.
- Range never > ±15% without `wide` override.
- Plugin error → reply verbatim, no retry.
