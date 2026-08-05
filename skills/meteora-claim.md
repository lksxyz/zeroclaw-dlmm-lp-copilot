---
name: meteora-claim
version: 1
custody: T1
summary: Build an unsigned `claimFee` transaction for a DLMM position, return a Solana Action URL.
---

# Claim Pending DLMM Fees (T1 — Build Unsigned)

The user said `claim #<id>` (or `claim` with the last-mentioned position).
The agent **does not sign or submit**. It builds an unsigned transaction,
hands it to a self-hosted Solana Action endpoint, and returns the URL so the
user can sign in their wallet.

## When to use

- User DM: `claim #4821` / `claim 4821` / `claim` (default: last-reported position)
- After `meteora-report` suggests `claim`

## Steps

### 1. Fetch the position

Call `meteora-position` for `#<id>`. You need:

- `pool_address`
- `position_account_pubkey`
- `operator_wallet_pubkey`
- `claimable_fee_x_amount`, `claimable_fee_y_amount` (display only — not used in tx)

### 2. Build the unsigned transaction

The DLMM `claimFee` instruction is a single ix. The exact layout lives in
Meteora's IDL — we use the TypeScript SDK via the action endpoint, not the
agent, because the encoding is non-trivial (Anchor-style with multiple
remaining accounts).

The action endpoint exposes:

```
GET  ${ACTION_ENDPOINT_BASE}/actions/claim?pos=<position_pubkey>&pool=<pool_address>
POST ${ACTION_ENDPOINT_BASE}/actions/claim?pos=<position_pubkey>&pool=<pool_address>
     body: { "account": "<user_wallet_pubkey>" }
```

`GET` returns the action metadata (title, icon, description, label).
`POST` returns `{ "transaction": "<base64>", "message": "<...>" }`.

The endpoint's tx builder uses the Meteora SDK `claimFee` helper. **It runs on
the worker, not the agent** — the agent never touches signing keys or raw
instruction encoding.

### 3. Return a Solana Action URL

The link Telegram needs to render an inline action button:

```
solana-action:${ACTION_ENDPOINT_BASE}/actions/claim?pos=${position_pubkey}&pool=${pool_address}&label=Claim
```

In Telegram, post a message with a `text` field containing this URL.
Telegram's link-preview will pick up the metadata and the wallet (Phantom /
Solflare) on the user's phone will render the action. **The agent does not
need a third-party shortener** — Phantom recognizes the `solana-action:` URI
directly when the device has the wallet installed.

If the user is on desktop, the same URL opens in their default wallet.
If no wallet is available, the URL is still copy-pasteable into Phantom
or Solflare manually.

### 4. Reply to the user

```
#<id> claim prepared → <amount> <X> + <amount> <Y> (~$<usd>)
Tap to sign in your wallet: <solana-action URL>
I will auto-confirm once the signature lands on-chain.
```

Then start a 60-second poll on `${SOLANA_RPC_URL}` with
`getSignaturesForAddress(position_pubkey, { limit: 1 })` and `getTransaction`
to confirm the claim landed. When confirmed, DM:

```
✓ #<id> claimed: <amount> <X> + <amount> <Y>  (~$<usd>)
   ref: <signature>
```

## Response shaping

The reply must be **one Telegram message, ≤ 1,000 chars**. If the action URL
gets truncated by a Telegram client, send the URL as a separate message.

## Do not

- **Do not** sign anything.
- **Do not** submit anything. The user's wallet does that.
- **Do not** store or transmit the user's private key. We never see it.
- **Do not** retry the build on a transient worker error without asking the
  user first — if the worker 5xxs twice, surface the error and stop.
- **Do not** auto-broadcast even if the user types "yes" or "go" in a follow-up
  DM. The Action URL is the only path. This is the **fail-closed** boundary.

## Failure modes

- Worker returns 4xx (bad position) → reply with the worker error verbatim,
  no retry.
- Worker returns 5xx → retry once after 3 s, then surface.
- User wallet not detected (Phantom/Solflare not installed) → the link still
  works for desktop; reply with a copy-paste hint for the mobile wallet.
- Blockhash stale (rare — claimFee is a single ix with no approval queue,
  but if the user delays signing for > 90 s the worker rebuilds on POST) →
  the worker returns a fresh tx on the next POST; we ask the user to tap again.
