---
name: meteora-report
version: 5
custody: T0
summary: Format dlmm_reader output as Telegram-friendly report
---

# meteora-report

Takes `meteora-position` output (from `dlmm_reader`). Formats for Telegram (4096 char limit).

Tools: use `send_message_to_peer`, `read_skill`, `memory_recall`. No network tools — data comes from the plugin.

## When

- Cron `dlmm-daily-report` at ${DAILY_REPORT_HOUR}:00
- DM `report`

Execute now. No clarifying questions.

## Markdown rules

Telegram supports: `*bold*`, `_italic_`, `` `code` ``, `\n`, emoji, `·`, `═`, `─`, `−`. No `#`, `|`, `- `, `**`.

## Template

### With positions

```
🦞 DLMM Daily — <YYYY-MM-DD> <HH:MM> <TZ>
═══════════════════════════════════════
*n* positions · claimable `$<sum>`

✅ *#<id>* `bin <lo>..<hi>` · *in*
   claimable `$<claimable>` · owner ✓
   ⤷ <action> — <reason ≤90 chars>

─────────────────────────────────────────────────

⚠️ *#<id>* `bin <lo>..<hi>` · *out-of-range*
   active `$<active_bin_id>` · claimable `$<claimable>`
   ⤷ rebalance — active bin at <active_id>, suggest recentering

═══════════════════════════════════════
↳ claim #<id> · rebalance #<id> · report
```

### Empty state

```
🦞 DLMM Daily — <YYYY-MM-DD> <HH:MM> <TZ>
═══════════════════════════════════════
No DLMM positions on this wallet.

I only watch one wallet (locked by plugin config).
To get started:
  1. Open https://app.meteora.ag/dlmm?cluster=devnet
  2. Pick a pool, deposit X + Y → position NFT lands in your wallet
  3. DM `report` again — I'll pick it up

↳ match Phantom's network to the bot's RPC.
```

## Icons

✅ in-range · ⚠️ out-of-range · ⛔ owner mismatch · ⏸️ stale price

## Number format

USD: `$1,847.32` / `$0.93` (comma, 0 or 2 decimals).
Empty: `--`, never `null`/`0`/`N/A`.

## Rules

- No signing, no tx building, no URLs in report
- No hedging, no apologising. Mirror user's language.
- Report only what `dlmm_reader` returned. Never invent claimable or range values.
