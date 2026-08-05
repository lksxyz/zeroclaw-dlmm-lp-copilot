---
name: meteora-rebalance
version: 3
custody: T1
summary: Build atomic remove+add rebalance tx on durable nonce, return Action URL
---

# meteora-rebalance

Build atomic removeLiquidity + addLiquidity on durable nonce. Return Action URL. User signs.

Tools: use `http_request`, `send_message_to_peer`, `read_skill`, `memory_recall`. Avoid `web_fetch`, `web_search_tool`, `browser` — stick to `http_request` for network calls.

## Trigger

DM `rebalance #<id>` / `rebalance <id>` / `rebalance` (last-reported). Execute now.

## Steps

### 1. Read position

`read_skill meteora-position`, fetch `#<id>`. Get: `pool_address`, `position_pubkey`, `lower_bin_id`, `upper_bin_id`, `active_id`, `bin_step`.

### 2. Propose new range

Center on `active_id`. Scale by bin_step:
- Default ±5% → `range_bins = 0.05 / (bin_step / 10000)`
- `rebalance #<id> wide` → ±10%
- `rebalance #<id> tight` → ±2%

### 3. Build atomic tx

```
http_request POST ${ACTION_ENDPOINT_BASE}/actions/rebalance?pos=<pubkey>&pool=<pool>&new_low=<lo>&new_high=<hi>&nonce=<nonce_acct>
{"account":"<user_wallet>"}
```

Worker encodes: AdvanceNonce → removeLiquidity(100%) → addLiquidityByStrategy. Durable nonce (`${NONCE_ACCOUNT}`, authority `${OPERATOR_WALLET_PUBKEY}`). Tx stays valid indefinitely.

### 4. Reply

```
#<id> rebalance · bins <old_lo>..<old_hi> → <new_lo>..<new_hi>

[Tap to rebalance](solana-action:${ACTION_ENDPOINT_BASE}/actions/rebalance?pos=<pubkey>&pool=<pool>&new_low=<lo>&new_high=<hi>&nonce=<nonce>&label=Rebalance)
```

### 5. Confirm

Poll position account until range updates. On success:

```
✓ #<id> rebalanced · <new_lo>..<new_hi>  ref: <sig>
```

## Rules

- Never sign. Never submit. Never broadcast.
- Never reuse nonce for concurrent txs.
- Range never > ±15% without `wide` override.
- Always remove + add in same tx.
