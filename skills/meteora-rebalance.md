---
name: meteora-rebalance
version: 1
custody: T1
summary: Build an unsigned removeLiquidity + addLiquidity pair on a durable nonce, return a Solana Action URL.
---

# Rebalance a DLMM Position (T1 — Build Unsigned + Durable Nonce)

The user said `rebalance #<id>` (or `rebalance` with the last-mentioned
position). The agent proposes a new bin range, builds an **atomic** transaction
(removeLiquidity from old bins + addLiquidity into new bins), and wraps it
in a Solana Action URL.

The transaction uses a **durable nonce** so that the user can take as long
as they want to approve in their wallet. A normal recent blockhash expires
in ~60-90 seconds, and the approval queue (signing in Phantom) routinely
outlives that.

## When to use

- User DM: `rebalance #4822`
- After `meteora-report` suggests `rebalance`

## Steps

### 1. Fetch state and propose a new range

Call `meteora-position` for `#<id>`. You need:

- `pool_address`
- `position_account_pubkey`
- `operator_wallet_pubkey`
- `current_lower_bin_id`, `current_upper_bin_id`
- `current_active_id` (the active bin)
- 24h price range (Jupiter price history if available, else the active bin
  range from the pool) — for the proposal

**Propose a new range**:

- Center the new range on the current `active_id`
- Default width: ±5% around `active_id_price`, scaled to `bin_step`
  (e.g. bin_step=10 → ~110 bins for a ±5% range on a stable pair)
- Override with `rebalance #<id> wide` (±10%) or `rebalance #<id> tight` (±2%)

### 2. Build the atomic transaction

Three instructions, in this order:

1. `AdvanceNonceAccount` — bumps the stored nonce so the tx is valid for signing
2. `removeLiquidity` — pulls 100% from the old position (closes the bin distribution)
3. `addLiquidityByStrategy` — re-deposits into the new bin range

The nonce account must be funded (~0.0015 SOL rent) and the operator wallet
must be its `authority`. The agent does **not** sign anything; the user
signs in Phantom. The worker does the encoding.

### 3. Hit the action endpoint

```
GET  ${ACTION_ENDPOINT_BASE}/actions/rebalance
     ?pos=<position_pubkey>
     &pool=<pool_address>
     &new_low=<new_lower_bin_id>
     &new_high=<new_upper_bin_id>
     &nonce=<nonce_account_pubkey>

POST ${ACTION_ENDPOINT_BASE}/actions/rebalance
     body: { "account": "<user_wallet_pubkey>" }
```

`GET` returns the action metadata — title is "Rebalance #<id>", description
shows old range, new range, expected value impact. `POST` returns the
base64-encoded transaction (already wraps the nonce advance + remove + add).

### 4. Return a Solana Action URL

```
solana-action:${ACTION_ENDPOINT_BASE}/actions/rebalance?pos=...&pool=...&new_low=...&new_high=...&nonce=...&label=Rebalance
```

### 5. Reply

```
#<id> rebalance prepared
  old range: bins <old_low>..<old_high>  (active <old_active>)
  new range: bins <new_low>..<new_high>
  fees paid:  3 ix · ~0.000015 SOL (single atomic tx)
Tap to sign: <solana-action URL>
```

Then watch the position account via `getProgramAccounts` (or `getAccountInfo`
on the position pubkey) and DM:

```
✓ #<id> rebalanced → range <new_low>..<new_high>  ref: <sig>
```

## Why a durable nonce, not a recent blockhash

Approval flow:

1. Agent posts Action URL → user opens Phantom
2. User reviews tx → walks to kitchen / takes a call
3. Returns 5 minutes later → signs

A recent blockhash is invalid in ~60-90 s. The nonce account is funded,
the agent POSTs once, the user signs at their pace, the tx is valid until
the user signs and submits.

One **caveat**: a single nonce account serializes one in-flight tx. If two
`rebalance` approvals are pending at once, they need two nonce accounts.
For daily use this is rare. Document the constraint in `SUBMISSION.md`.

## Do not

- **Do not** sign or submit.
- **Do not** reuse the same nonce account for two concurrent pending txs.
- **Do not** propose a new range wider than ±15% without a `wide` override from
  the user — auto-rebalancing into a 30% range burns fee efficiency.
- **Do not** auto-broadcast under any circumstance.
- **Do not** call `removeLiquidity` without the matching `addLiquidityByStrategy`
  in the same atomic tx. A user with bins cleared and no replacement position
  has stopped earning and is exposed to plain price action.
