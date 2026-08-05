# DLMM LP Copilot — Superteam Brasil bounty submission

> `#solana-bounty` showcase post. [Video](./showcase/video-script.md) · [Repo](https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot)

## What it does

A ZeroClaw agent in your Telegram that watches Meteora DLMM positions: daily
report at 08:00, out-of-range alerts every 30 min, and on-demand `claim` /
`rebalance` via Solana Action URL. Agent builds unsigned txs — you sign in
Phantom. No keys held.

## Who it's for

Active Solana LP with 3-5 DLMM positions who doesn't want to check dashboards.
Brazilian LPs get `America/Sao_Paulo` timezone (configurable).

## ZeroClaw features used

- Telegram channel plugin (registry)
- SOP engine with cron triggers (`dlmm-daily-report`, `dlmm-range-monitor`)
- Agent-driven DM handling — Telegram DMs flow through agent runtime (ZeroClaw
  only sets `internal_sop_event` for git/forge channels)
- Memory (position baselines, alert dedupe)
- `http_request` tool with domain allowlist
- `read_skill` sandboxing

## What we built

1. **Four skills** (`skills/meteora-*.md`): T0 read + IL math, T0 report format,
   T1 claim tx builder, T1 rebalance tx builder with durable nonce proposal.
2. **Self-hosted Solana Action endpoint** (`action-endpoint/`): Cloudflare Worker
   serving GET/POST per Solana Actions spec. Encodes `claimFee` and
   `removeLiquidity + addLiquidityByStrategy` via Meteora SDK. Wraps rebalance in
   durable nonce. No keys, no signing — returns base64 unsigned txs only.
   Single-operator: nonce authority = LP's wallet, rejects other signers with 403.

Skills are pure markdown. Action endpoint is the only custom code. Both
reproducible from this repo.

## Custody tier

| Operation | Tier | Secrets | Signs |
|---|---|---|---|
| Read / pool / price | T0 | RPC key | none |
| Report / OOR alert / milestone | T0 | RPC key | none |
| Build unsigned claimFee | T1 | none | user wallet |
| Build atomic rebalance (durable nonce) | T1 | none | user wallet |
| Auto-compound | T2 | disabled | n/a |

T2 reasoning: bounty warns "safety most often lost" at T2. Submitted config has
no session key, no autocompound block, no sign capability. Skills have no
`sign_and_submit`. The agent *refuses by construction*.

## Threat model

**Channel = prompt-injection surface.** Agent only proposes Action URLs. User
signs. Full adversarial transcript: `prompts/injection-tests.md` (7 scenarios).

**Blockhash expiry.** Rebalance uses durable nonce — tx stays valid past 90s
blockhash window. One nonce per concurrent pending tx.

**Price feed.** Jupiter Price API (public, unauthenticated). Pyth deprecated
2026-07, Switchboard dead 2026-08.

**Third-party trust:** Jupiter (read-only), Cloudflare (worker host), Helius/RPC
(user-supplied). Declared.

**Filesystem tools denied outright.** `content_search`, `glob_search`,
`file_read`, `file_write`, `file_edit`, `data_management`, `memory_export`,
`cron_list`, and the web tools (`web_fetch`, `browser`, `web_search_tool`,
`weather`) sit in `risk_profiles.dlmm.excluded_tools` — not "approval-gated",
**denied**. Approval-gating alone is bypassable when a Telegram approval is
rushed or missed; deny-by-default removes the question. `memory_recall`
remains (position baselines, no secrets).

## What we did NOT do

- No trading bot, sniping, buy recommendations
- No raw private key — no key in the system at all
- No concept/slideware — runs on Devnet in the video
- No registry PR — plugin lives in this repo
- No thin RPC wrapper in WASM — output shaped to ~200 tokens/position

## Beyond the brief

- Self-hosted Action endpoint (no Dialect dependency)
- Durable nonces (blockhash trap solved)
- Jupiter feed (no Pyth/Switchboard dependency)
- Filesystem + web tools **denied outright** (not just approval-gated) + 7-scenario injection suite
- Triple-gated defense: LLM (skill rules) + Tool (denied-by-default) + Cryptographic (on-chain auth)

## Reproduce

```bash
curl -fsSL https://raw.githubusercontent.com/zeroclaw-labs/zeroclaw/master/install.sh | bash
zeroclaw quickstart
git clone https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot
cd zeroclaw-dlmm-lp-copilot
cp skills/*.md ~/.zeroclaw/skills/
cp -r sops/dlmm-* ~/.zeroclaw/sops/
cp config.example.toml ~/.zeroclaw/config.toml
# edit ~/.zeroclaw/config.toml, fill env vars
cd action-endpoint && npm install
npx wrangler secret put RPC_URL
npx wrangler secret put NONCE_ACCOUNT && npx wrangler secret put NONCE_AUTHORITY
npx wrangler deploy
cd ..
zeroclaw service install && zeroclaw service start
```

## Verify

- Code: this repo (skills, SOPs, action endpoint, threat model — all in-tree)
- Action endpoint: `npm test && npm run typecheck`
- WASM plugin: `make plugin && make plugin-build`
- Config: `make validate`
- Demo: `showcase/demo-transcript.md` (exact Devnet command sequence)
- Injection: `prompts/injection-tests.md` (7 scenarios, expected behavior per scenario)

## Future work

- T2 auto-compound behind Subscriptions & Allowances + session key + caps
- Squads v4 Proposer integration
- BRL reporting (economia.awesomeapi.com.br lookup)
- x402 oracle (paid LP analysis, $0.001/query)

## Links

- Repo: <https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot>
- Showcase video script: `showcase/video-script.md` (video produced separately)
- Demo runbook: `showcase/demo-transcript.md`
- Discord: <https://discord.gg/zeroclaw> → `#solana-bounty`
- Bounty: Superteam Earn — Build Solana-native plugins for ZeroClaw
