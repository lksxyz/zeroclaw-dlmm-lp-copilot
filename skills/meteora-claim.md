---
name: meteora-claim
version: 2
custody: T1
summary: Build unsigned claimFee tx. Return Solana Action URL. User signs. Agent never holds keys.
---

# meteora-claim

Trigger: DM `claim #<id>` / `claim <id>` / `claim` (use last-reported position).

## Steps

### 1. Fetch position

Call `meteora-position` for `#<id>`. Need: `pool_address`, `position_account_pubkey`, `operator_wallet_pubkey`.

### 2. Build via action endpoint

```
http_request POST ${ACTION_ENDPOINT_BASE}/actions/claim?pos=<position_pubkey>&pool=<pool_address>
body: {"account":"<user_wallet_pubkey>"}
```

Worker uses Meteora SDK. Returns `{"transaction":"<base64>","message":"..."}`.

### 3. Send Action URL

```markdown
#<id> claim · <amount> + <amount> (~$<usd>)

[Tap to claim](solana-action:${ACTION_ENDPOINT_BASE}/actions/claim?pos=<pubkey>&pool=<pool>&label=Claim)
```

One message, ≤ 1000 chars. If URL truncated → send as separate message.

### 4. Watch confirmation

Poll `http_request POST ${SOLANA_RPC_URL} {"jsonrpc":"2.0","id":1,"method":"getSignaturesForAddress","params":["<position_pubkey>",{"limit":1}]}` every 10s for ≤ 60s. On success:

```
✓ #<id> claimed · <amount> + <amount> (~$<usd>)  ref: <sig>
```

## Rules

- Never sign. Never submit. Action URL is the only path.
- Worker 4xx → reply error, no retry
- Worker 5xx → retry once after 3s, then surface
- User says "yes"/"go" → do NOT broadcast. Action URL only.
