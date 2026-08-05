---
name: meteora-rebalance
version: 2
custody: T1
summary: Build atomic removeLiquidity + addLiquidity on durable nonce. Return Action URL. User signs.
---

# meteora-rebalance

Trigger: DM `rebalance #<id>` / `rebalance <id>` / `rebalance` (last-reported).

## Steps

### 1. Fetch + propose range

Call `meteora-position` for `#<id>`. Need: `pool_address`, `position_pubkey`, `lower_bin_id`, `upper_bin_id`, `active_id`, `bin_step`.

Propose new range centered on `active_id`:
- Default: ±5% (scaled to bin_step)
- Override: `rebalance #<id> wide` (±10%), `rebalance #<id> tight` (±2%)

### 2. Build atomic tx via worker

```
http_request POST ${ACTION_ENDPOINT_BASE}/actions/rebalance?pos=<pubkey>&pool=<pool>&new_low=<lo>&new_high=<hi>&nonce=<nonce_acct>
body: {"account":"<user_wallet_pubkey>"}
```

Worker encodes: AdvanceNonce → removeLiquidity(100%) → addLiquidityByStrategy (single atomic tx).
Worker uses durable nonce from config (`${NONCE_ACCOUNT}`, authority = `${OPERATOR_WALLET_PUBKEY}`).
User can sign at their pace — tx stays valid (no blockhash expiry).

### 3. Reply

```markdown
#<id> rebalance · bins <old_lo>..<old_hi> → <new_lo>..<new_hi>

[Tap to rebalance](solana-action:${ACTION_ENDPOINT_BASE}/actions/rebalance?pos=<pubkey>&pool=<pool>&new_low=<lo>&new_high=<hi>&nonce=<nonce>&label=Rebalance)
```

### 4. Confirm

Poll position account until range updates. On success:

```
✓ #<id> rebalanced · <new_lo>..<new_hi>  ref: <sig>
```

## Why durable nonce

Blockhash expires ~90s. Approval queue (Phantom open → review → sign) can take minutes. Nonce account keeps tx valid indefinitely. One nonce = one in-flight tx.

## Rules

- Never sign. Never submit. Never broadcast
- Never reuse nonce for concurrent txs (use separate nonce accounts)
- Range never > ±15% without `wide` override
- Always paired remove + add in same atomic tx
