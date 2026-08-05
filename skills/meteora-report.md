---
name: meteora-report
version: 3
custody: T0
summary: Format DLMM positions into a Telegram daily report.
---

# meteora-report

Takes `meteora-position` output. Formats for Telegram. 4096 char limit — split with `--- end part 1/N ---` if needed.

## Trigger

- Cron `dlmm-daily-report` at ${DAILY_REPORT_HOUR}:00
- DM `report` / `report dong` — execute immediately, no questions

## Telegram markdown

Only: `*bold*`, `_italic_`, `` `code` ``, `\n`, Unicode box-drawing, emoji, `·`. No `#`, `|`, `-`, `**`.

## Format

### Header

```
🦞 DLMM Daily — <YYYY-MM-DD> <HH:MM WIB>
═══════════════════════════════════════
*n* positions · TVL `$<sum>` · 24h fees `$<sum>` · claimable `$<sum>`
```

### Per position

```
✅ *#<id>* `X/Y`  · bin_step `<n>`
   value `$<usd>` · range *in* · IL `−<pct>%`
   24h fees `$<fees>` · claimable `$<claimable>`
   ⤷ <action> — <reason ≤90 chars>

─────────────────────────────────────────────────
```

Icons:

| Icon | When |
|---|---|
| ✅ | In-range |
| ⚠️ | Out-of-range or IL > 2× alert |
| 🟡 | IL between 0.5× and 2× alert |
| ⏸️ | Stale price (Jupiter down) |
| ⛔ | Stale pool (Meteora 404) |

### Footer

```
═══════════════════════════════════════
↳ claim #<id> · rebalance #<id> · report
```

Quiet day (all stable, no fees above milestone):

```
↳ all quiet. range check at :30 past the hour.
```

### Empty state

```
🦞 DLMM Daily — <YYYY-MM-DD> <HH:MM WIB>
═══════════════════════════════════════
No DLMM positions for this wallet.

↳ open a position at https://app.meteora.ag/dlmm
```

### Number format

USD: `$1,847.32` / `$0.93` / `$12,408.10` (comma separator, 0 or 2 decimals).
IL: en-dash `−`, sign, 2 decimals, e.g. `−0.42%` (not `-`, not `-0.42`).
Empty/unknown: `--` (never `null`, `0`, `N/A`).

### Deltas

Read from `memory.baseline_position_value_<id>`. Show `Δ 24h` + `Δ 7d` in footer.
First run (no baseline) → write V0, show `--`.

## Output

Send with `send_message_to_peer` → `telegram.<<CHANNEL_ALIAS>>` → `<<TARGET>>`.

## Rules

- Read-only. No signing, no tx building, no Action URLs in report
- Never hedge, never apologise. One fact per line
- Mirror user locale (Indonesian → Indonesian, English → English)
- No trailing empty lines
