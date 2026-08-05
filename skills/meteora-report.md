---
name: meteora-report
version: 1
custody: T0
summary: Format the daily DLMM position report for Telegram.
---

# Daily DLMM Position Report (Telegram)

Read-only. Consumes the output of `meteora-position` and renders a Telegram-friendly
summary. **Telegram limit: 4096 chars per message** — if the report is longer,
split into multiple messages with `--- end part 1/2 ---` style markers.

## Trigger

`daily-report` SOP at `${DAILY_REPORT_HOUR}:00` in the operator's timezone
(default `America/Sao_Paulo`). The SOP calls `meteora-position` first to fetch
state, then this skill formats and sends.

## Header

```
🦞 DLMM Daily — <YYYY-MM-DD>
<n> positions | TVL $<sum_tvl> | 24h fees $<sum_24h>
```

## Body — one block per position

```
• #<id> <X>/<Y>  (<status>)
  $<value_usd>  range <in|out>  IL <pct>%
  24h fees: $<n>  claimable: $<m>
  → <action>: <one-line rationale>
```

`<status>` is one of `in-range`, `out-of-range`, `stale-price`, `stale-pool`.

`<action>` is one of `hold`, `rebalance`, `claim`, `review` (the last means
IL is in a soft-warning band — not an alert, just a heads-up).

## Footer

Two short lines:

```
Δ 24h: $<delta_value>  |  Δ 7d: $<delta_value_7d>
Range check at :30 past the hour. Reply with: claim #<id> · rebalance #<id> · report
```

The deltas come from `memory.baseline_position_value_<id>`. Persist that on
every report run. If no baseline exists yet (first run), print `--` and
establish the baseline.

## Tone

Short. One line per fact. No exclamation marks. Telegram renders markdown so
use `*bold*` sparingly, never `**`. No emoji beyond the `🦞` header.

## Example (3 positions, mixed state)

```
🦞 DLMM Daily — 2026-07-30
3 positions | TVL $12,408 | 24h fees $1.84

• #4821 SOL/USDC  (in-range)
  $4,210.50  range in  IL -0.42%
  24h fees: $0.93  claimable: $2.18
  → claim: above $1 milestone

• #4822 JUP/USDC  (out-of-range)
  $1,120.10  range out  IL -3.71%
  24h fees: $0.04  claimable: $0.07
  → rebalance: JUP moved 8% in 24h, suggest ±5% range

• #4830 BONK/SOL  (in-range)
  $7,077.40  range in  IL -1.18%
  24h fees: $0.87  claimable: $1.61
  → hold

Δ 24h: +$28.40  |  Δ 7d: +$312.10
Range check at :30 past the hour. Reply with: claim #<id> · rebalance #<id> · report
```

## Do not

- Do not include any base64 or signature data.
- Do not include any action URL in the report — that goes in `meteora-claim`
  and `meteora-rebalance` responses, only when the user explicitly asks.
- Do not call any tx-building tool. This skill is read-only.
