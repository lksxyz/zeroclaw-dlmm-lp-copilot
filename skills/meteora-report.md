---
name: meteora-report
version: 4
custody: T0
summary: Format position data as Telegram-friendly report
---

# meteora-report

Takes `meteora-position` output. Formats for Telegram (4096 char limit).

Tools: use `send_message_to_peer`, `read_skill`, `memory_recall`, `http_request`. Avoid `web_fetch`, `web_search_tool`, `browser` — stick to `http_request` for network calls.

## When

- Cron `dlmm-daily-report` at ${DAILY_REPORT_HOUR}:00
- DM `report`

Execute now. No clarifying questions.

## Markdown rules

Telegram supports: `*bold*`, `_italic_`, `` `code` ``, `\n`, emoji, `·`, `═`, `─`, `−`. No `#`, `|`, `- `, `**`.

## Template

### With positions

```
🦞 DLMM Daily — <YYYY-MM-DD> <HH:MM> WIB
═══════════════════════════════════════
*n* positions · TVL `$<sum>` · 24h fees `$<sum>` · claimable `$<sum>`

✅ *#<id>* `X/Y` · bin_step `<n>`
   value `$<usd>` · range *in* · IL `−<pct>%`
   24h fees `$<24h>` · claimable `$<claimable>`
   ⤷ <action> — <reason ≤90 chars>

─────────────────────────────────────────────────

⚠️ *#<id>* `X/Y` · bin_step `<n>` · *out-of-range*
   value `$<usd>` · range *out* · IL `−<pct>%`
   24h fees `$<24h>` · claimable `$<claimable>`
   ⤷ rebalance — active bin at <active_id>, suggest recentering

═══════════════════════════════════════
Δ 24h `+$<delta>` · Δ 7d `+$<delta>`
↳ claim #<id> · rebalance #<id> · report
```

### Empty state

```
🦞 DLMM Daily — <YYYY-MM-DD> <HH:MM> WIB
═══════════════════════════════════════
No DLMM positions for this wallet.

↳ open a position at https://app.meteora.ag/dlmm
```

## Icons

✅ in-range · ⚠️ out-of-range · 🟡 IL warning · ⏸️ stale price · ⛔ stale pool

## Number format

USD: `$1,847.32` / `$0.93` (comma, 0 or 2 decimals).
IL: en-dash `−`, sign, 2 decimals (`−0.42%`).
Empty: `--`, never `null`/`0`/`N/A`.

## Rules

- No signing, no tx building, no Action URLs in report
- No hedging, no apologising. Mirror user's language.
- Deltas from `memory.baseline_value_<id>`. First run = write V0, show `--`.
