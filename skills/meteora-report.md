---
name: meteora-report
version: 2
custody: T0
summary: Format the daily DLMM position report for Telegram.
---

# Daily DLMM Position Report (Telegram)

Read-only. Consumes the output of `meteora-position` and renders a Telegram-friendly
summary. **Telegram limit: 4096 chars per message** — if the report is longer,
split into multiple messages with `--- end part 1/2 ---` style markers.

## Trigger

`daily-report` SOP at `${DAILY_REPORT_HOUR}:00` in the operator's timezone
(the user has chosen `Asia/Jakarta` in `<<DAILY_REPORT_TZ>>`). The SOP calls
`meteora-position` first to fetch state, then this skill formats and sends.

## Footer CTAs

Append one line at the end of every report so the operator knows what to do:

```
↳ reply with: claim #<id> · rebalance #<id> · report
```

When the day is quiet (no positions, or all positions stable with no fee above
the milestone), the footer becomes:

```
↳ all quiet. range check at :30 past the hour.
```

## Format

Telegram renders a small subset of Markdown. Use **only** these markers:

- `*bold*` (single asterisk, NOT `**`)
- `_italic_`
- `` `code` `` for numbers that should not wrap
- line breaks (`\n`) for spacing
- Unicode box-drawing, dots, and emojis listed below

Do **not** use headings (`#`), tables (`|...|`), lists (`- `), or `**bold**` —
they render as raw text or break on older Telegram clients.

### Header

```
🦞 DLMM Daily — <YYYY-MM-DD> <HH:MM TZ>
═══════════════════════════════════════
*n* positions · TVL `$<sum_tvl>` · 24h fees `$<sum_24h>` · claimable `$<sum_claimable>`
```

`<HH:MM TZ>` is the operator's local time at the moment the SOP runs — example
`08:00 WIB`. The `═` line is 27 characters wide to match the emoji-anchored
header; Telegram renders it as a hairline box.

### Per-position block

One block per position. Use a leading status icon and a thin `─` divider
between blocks:

```
✅ *#4821* `SOL/USDC`  · bin_step `<n>`
   value `$4,210.50` · range *in* · IL `−0.42%`
   24h fees `$0.93` · claimable `$2.18`
   ⤷ claim — above $1 milestone

─────────────────────────────────────────────────
```

Status icons:

| icon | when |
|------|------|
| ✅ | in-range, healthy |
| ⚠️ | out-of-range OR IL worse than `2 × IL_ALERT_PCT` |
| 🟡 | soft warning (IL between `0.5 ×` and `2 × IL_ALERT_PCT`) |
| ⏸️ | stale-price (could not compute IL, Jupiter feed down) |
| ⛔ | stale-pool (Meteora API 404 for the pool) |

`<action>` is one of `hold`, `rebalance`, `claim`, `review`. The `⤷ <action> — <reason>`
line is the only place the agent gives free-form advice; keep it under 90 chars.

### Number formatting

- USD values: thousands separator, no decimals under $1, two decimals above
  - `$1,847.32` · `$0.93` · `$12,408.10`
- IL: always sign, two decimals, en-dash for the negative sign (Telegram renders
  `−` and `-` differently — `−` is narrower and reads better)
  - `IL -0.42%` → render as `IL `−`0.42%`
- Percentages: trailing `%`, no space
- Empty / unknown: `--` (em-dash), not `null` or `0`

### Footer

```
═══════════════════════════════════════
Δ 24h `+$28.40` · Δ 7d `+$312.10`
↳ reply with: claim #<id> · rebalance #<id> · report
```

If 24h/7d delta is unknown (first run, no baseline), show `--` for that
field and drop the line:

```
═══════════════════════════════════════
Δ 24h `+ $28.40` · Δ 7d `--`
↳ reply with: claim #<id> · rebalance #<id> · report
```

The deltas come from `memory.baseline_position_value_<id>`. Persist that on
every report run. If no baseline exists yet (first run), print `--` and
establish the baseline.

### Empty-state (no positions)

When the wallet has zero DLMM positions (a valid outcome, not an error):

```
🦞 DLMM Daily — 2026-08-05 08:00 WIB
═══════════════════════════════════════
No DLMM positions for this wallet.

TVL `$0` · 24h fees `$0` · claimable `$0`
═══════════════════════════════════════
↳ open a position at https://app.meteora.ag/dlmm
```

Do **not** invent fake positions. Do **not** say "all clear" — that's the
range-monitor's vocabulary, this is the daily report.

## Tone

- No exclamation marks. Emoji are functional (status icons + header), not
  decorative.
- One fact per line. If two facts look like they belong together, separate
  them with `·` (middle dot, not `|`).
- Never apologise. Never hedge. "No positions" is the truth, not a failure.
- Same language as the operator's chat locale. If the operator writes in
  Indonesian, reply in Indonesian; if English, English. Mirror, don't translate.

## Example (3 positions, mixed state)

```
🦞 DLMM Daily — 2026-07-30 08:00 WIB
═══════════════════════════════════════
*3* positions · TVL `$12,408.10` · 24h fees `$1.84` · claimable `$3.86`

✅ *#4821* `SOL/USDC`  · bin_step `10`
   value `$4,210.50` · range *in* · IL `−0.42%`
   24h fees `$0.93` · claimable `$2.18`
   ⤷ claim — above $1 milestone

─────────────────────────────────────────────────

⚠️ *#4822* `JUP/USDC`  · bin_step `20`  · out-of-range
   value `$1,120.10` · range *out* · IL `−3.71%`
   24h fees `$0.04` · claimable `$0.07`
   ⤷ rebalance — JUP moved 8% in 24h, suggest ±5% range

─────────────────────────────────────────────────

✅ *#4830* `BONK/SOL`  · bin_step `100`
   value `$7,077.40` · range *in* · IL `−1.18%`
   24h fees `$0.87` · claimable `$1.61`
   ⤷ hold

═══════════════════════════════════════
Δ 24h `+$28.40` · Δ 7d `+$312.10`
↳ reply with: claim #<id> · rebalance #<id> · report
```

## Do not

- Do not include any base64 or signature data.
- Do not include any action URL in the report — that goes in `meteora-claim`
  and `meteora-rebalance` responses, only when the user explicitly asks.
- Do not call any tx-building tool. This skill is read-only.
- Do not use `**bold**`, `# heading`, `- bullet`, or `| table |` — Telegram
  renders them as literal text.
- Do not pad the message with trailing empty lines.
