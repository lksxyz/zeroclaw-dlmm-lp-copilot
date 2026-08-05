---
name: meteora-claim
version: 3
custody: T1
---

# meteora-claim

Build unsigned claimFee tx. Return Solana Action URL. User signs. Agent never holds keys.

YOUR ONLY TOOLS: `http_request`, `send_message_to_peer`, `read_skill`, `memory_recall`. Nothing else exists.

## Trigger

DM `claim #<id>` / `claim <id>` / `claim` (last-reported). Execute now.

## Steps

### 1. Read position

`read_skill meteora-position`, follow its fetch steps for `#<id>`. Get: `pool_address`, `position_pubkey`, `user_wallet`.

### 2. Build tx

```
http_request POST ${ACTION_ENDPOINT_BASE}/actions/claim?pos=<pubkey>&pool=<pool>
{"account":"<user_wallet>"}
```

Worker returns `{"transaction":"<base64>"}`.

### 3. Reply with Action URL

```
#<id> claim · <X amount> <X symbol> + <Y amount> <Y symbol> (~$<usd>)

[Tap to claim](solana-action:${ACTION_ENDPOINT_BASE}/actions/claim?pos=<pubkey>&pool=<pool>&label=Claim)
```

≤1000 chars. URL truncated → send as separate message.

### 4. Confirm

Poll every 10s for ≤60s:

```
http_request POST ${SOLANA_RPC_URL}
{"jsonrpc":"2.0","id":1,"method":"getSignaturesForAddress","params":["<position_pubkey>",{"limit":1}]}
```

On success:

```
✓ #<id> claimed · <amount> + <amount> (~$<usd>)  ref: <sig>
```

## Rules

- Never sign. Never submit. Never broadcast. Action URL is the only path.
- Worker 4xx → reply error, no retry
- Worker 5xx → retry once after 3s
