# DLMM LP Copilot 🦞

Self-hosted ZeroClaw agent watching [Meteora DLMM](https://app.meteora.ag/) positions via
**Telegram**. Pings you only when something needs attention.

> Superteam Brasil bounty: T0 (read) + T1 (build unsigned tx). No keys held.

## Features

- **Daily 08:00 WIB** — position report (value, fees, IL vs HODL)
- **Every 30 min** — out-of-range alert
- **DM `report`** — on-demand position report
- **DM `claim #<id>`** → Solana Action URL → sign in Phantom
- **DM `rebalance #<id>`** → durable-nonce atomic tx → sign in Phantom
- **Fee milestone** — DM when claimable ≥ threshold

## Repo layout

```
skills/           # Agent-readable markdown (loaded every invoke)
  meteora-position.md    T0: fetch + compute IL
  meteora-report.md      T0: Telegram format
  meteora-claim.md       T1: unsigned claimFee → Action URL
  meteora-rebalance.md   T1: remove + add on durable nonce
sops/             # Cron definitions
  dlmm-daily-report/     08:00 report
  dlmm-range-monitor/    */30 OOR check
action-endpoint/  # Cloudflare Worker (Solana Actions server)
config.example.toml
prompts/injection-tests.md
```

## Quick start

```bash
# 1. Install ZeroClaw
curl -fsSL https://raw.githubusercontent.com/zeroclaw-labs/zeroclaw/master/install.sh | bash
zeroclaw quickstart

# 2. Telegram bot (BotFather → bot token; userinfobot → chat id)
#    Store in ~/.zeroclaw/secrets/telegram.env

# 3. Copy config and fill placeholders
cp config.example.toml ~/.zeroclaw/config.toml

# 4. Deploy action endpoint
cd action-endpoint && npm install
npx wrangler secret put RPC_URL
npx wrangler secret put NONCE_ACCOUNT       # rebalance only
npx wrangler secret put NONCE_AUTHORITY     # must be your signing wallet
npx wrangler deploy

# 5. Load skills + SOPs
mkdir -p ~/.zeroclaw/skills ~/.zeroclaw/sops
cp skills/*.md ~/.zeroclaw/skills/
cp -r sops/dlmm-* ~/.zeroclaw/sops/

# 6. Start
zeroclaw service install && zeroclaw service start
```

WASM plugin (Tier 3 bonus, optional): `make plugin-build && make plugin`.

## Custody tier

| Operation | Tier | Signs |
|---|---|---|
| Read / report / alert | T0 | none |
| Build claim tx | T1 | user wallet |
| Build rebalance tx | T1 | user wallet (durable nonce) |
| Auto-compound | T2 | disabled by default |

## Threat model (short)

**Channel = prompt-injection surface.** Agent only *proposes* — user wallet signs.
`excluded_tools` hard-blocks `content_search`, `glob_search`, `file_read`,
`file_write`, `file_edit`, `data_management`, `memory_export`, `cron_list`,
`web_fetch`, `browser`, `web_search_tool` from Telegram. `memory_recall` reads
position baselines (no secrets). See `prompts/injection-tests.md` (7 scenarios).

**RPC key in URL query param.** ZeroClaw's `http_request` only supports
`Authorization` header — Helius needs `x-api-key`. Key stays on disk.

**Durable nonce.** Rebalance tx stays valid through approval delays.

**Price feed.** Jupiter Price API (public). Pyth deprecated 2026-07.
Switchboard dead 2026-08.

## What this is not

- Not a trading bot. No sniping, no token picks.
- Not T2 auto-compounder. Designed, disabled, out of scope.
- No registry PR. Plugin lives in this repo per bounty rules.

## Also a starter kit

See `STARTER.md` — forkable template for any protocol-on-ZeroClaw use case.

```bash
make help       # targets
make validate   # TOML + skill frontmatter + cargo test
make demo       # end-to-end Devnet check
```

License: MIT.
