---
name: meteora-claim
version: 4
custody: T1
summary: Build unsigned claim tx in-wasm (dlmm_builder), return solana-action URL
---

# meteora-claim

Build the unsigned claim tx with the `dlmm_builder` plugin (in-wasm: fetch +
decode + mechanical validation + wire encoding). Return the `solana-action:`
URL. User signs. Agent never holds keys.

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
dlmm_builder {"mode":"claim","position_id":"<id>","nonce_address":"${NONCE_ACCOUNT}"}
```

Plugin validates mechanically (owner == `__config.owner_pubkey`, claimable > 0,
bin-array range) and returns:

```
{"action_url":"solana-action:...","tx_base64":"...","instructions":2,
 "recent_blockhash":"<nonce hash>","label":"Claim DLMM fees"}
```

### 3. Reply with the action URL

```
#<id> claim · <x> X + <y> Y (~$<usd>)

Tap to sign: <action_url>
```

≤1000 chars. URL truncated → send as separate message.

### 4. Confirm settlement

Poll every 10s for ≤60s: `dlmm_reader {"mode":"status","nonce_address":"${NONCE_ACCOUNT}","previous_nonce_hash":"<recent_blockhash from step 2>"}`.

`settled: true` → the nonce advanced → tx landed:

```
✓ #<id> claimed — nonce advanced (settled)
```

`settled: false` after 60s → "still pending; nothing lost — the nonce guard
rejects double-spends".

## Rules

- Never sign. Never submit. Never broadcast. Action URL is the only path.
- Plugin error → reply verbatim, no retry
- One pending tx per nonce account: if `status` says unsettled, do NOT build
  another tx on the same nonce.
