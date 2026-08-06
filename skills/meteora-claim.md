---
name: meteora-claim
version: 5
custody: T1
summary: Build unsigned claim tx in-wasm (dlmm_builder), present the raw unsigned tx (base64) for the operator to sign in any wallet
---

# meteora-claim

Build the unsigned claim tx with the `dlmm_builder` plugin (in-wasm: fetch +
decode + mechanical validation + wire encoding). Present the **raw unsigned
transaction (base64)** — the operator signs it in their own wallet
(Phantom, Solflare, CLI, etc.). The agent never signs, never submits, never
holds keys.

Tools: `dlmm_builder`, `dlmm_reader`, `send_message_to_peer`, `read_skill`,
`memory_recall`. `http_request` is DENIED.

## Trigger

DM `claim #<id>` / `claim <id>` / `claim` (last-reported). Execute now.

## Steps

### 1. Confirm the position

`dlmm_reader {"mode":"report","position_ids":["<id>"]}` — confirm
`owner_match: true` (else stop: "ownership mismatch — refusing to build") and
claimable > 0 (the plugin rejects zero claims anyway).

### 2. Build tx

```
dlmm_builder {"mode":"claim","position_id":"<id>"}
```

The nonce address comes from `__config.nonce_address` (host-injected,
anti-spoof) — do NOT ask the operator for it and do NOT pass it as an arg.
The plugin validates mechanically (owner == `__config.owner_pubkey`,
claimable > 0, bin-array range) and returns:

```
{"tx_base64":"...","instructions":2,
 "recent_blockhash":"<nonce hash>","label":"Claim DLMM fees",
 "summary":{"kind":"claim","range":[...]}}
```

### 3. Reply with the unsigned tx

Reply with the raw `tx_base64` from the plugin output. The operator signs it
in their wallet (Solflare/Phantom/CLI). Don't include the `action_url`.

```
#<id> claim · <x> X + <y> Y (~$<usd>)

Unsigned tx (base64) — sign in your wallet:
<tx_base64>
```

Show `tx_base64` in full (own message if Telegram truncates). `recent_blockhash`
is the durable-nonce hash when `__config.nonce_address` is set, so the tx
doesn't expire in 90s; if `instructions` is 1 (no nonce), tell the operator
it expires in ~90s. Echo only the plugin's fields.

### 4. Confirm settlement

Poll every 10s for ≤60s: `dlmm_reader {"mode":"status","nonce_address":"${NONCE_ACCOUNT}","previous_nonce_hash":"<recent_blockhash from step 2>"}`.

`settled: true` → the nonce advanced → tx landed:

```
✓ #<id> claimed — nonce advanced (settled)
```

`settled: false` after 60s → "still pending; nothing lost — the nonce guard
rejects double-spends".

## Rules

- Never sign. Never submit. Never broadcast. The operator signs the unsigned
  tx in their own wallet.
- Plugin error → reply verbatim, no retry
- One pending tx per nonce account: if `status` says unsettled, do NOT build
  another tx on the same nonce.
