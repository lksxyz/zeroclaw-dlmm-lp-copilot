# DLMM LP Copilot 🦞

A self-hosted ZeroClaw agent that watches your [Meteora DLMM](https://app.meteora.ag/) positions
on **Telegram** and only pings you when something needs attention. Daily report, out-of-range
alerts, and one-tap rebalance / fee-claim via a self-hosted Solana Action endpoint.

> Submission for the **Build Solana-native plugins for ZeroClaw** bounty on Superteam Earn
> (Superteam Brasil). T0 (read) + T1 (build unsigned tx). No keys held by the agent.

## What it does

You have 3-5 DLMM positions (SOL/USDC, JUP/USDC, etc.). The agent runs in your Telegram:

- **Daily 08:00** — formatted position report (value, fees 24h, range status, IL vs HODL)
- **Every 30 min** — out-of-range check; urgent alert if any position slipped out
- **On demand** — DM the agent `report`, `claim #1234`, or `rebalance #1234` and the
  agent reads the meteora skills, fetches on-chain state, and responds. For claim and
  rebalance it prepares an unsigned transaction behind a Solana Action URL. Tap it in
  Telegram → Phantom opens → preview → sign in your wallet. **The agent never holds keys.**
  On-demand operations flow through the agent (not cron SOPs) because ZeroClaw's Telegram
  channel does not populate `internal_sop_event` — only the git/forge channel does.
- **Fee milestone** — pings you when claimable fees cross a threshold

## Why T0 + T1, not T2

Bounty says T0 and T1 are the sweet spot, and T2 is "where safety is most often lost".
For an LP guardian, the user-controlled signing path is also the right one — you see
exactly what the agent is proposing before any funds move. T2 (auto-compound) is
**designed but disabled by default** in the write-up; it can be enabled with on-chain
caps via the Subscriptions & Allowances program if desired. Out of scope here.

## Repo layout

```
dlmm-lp-copilot/
├── README.md                       # this file
├── LICENSE                         # MIT
├── config.example.toml             # ZeroClaw config, redacted — copy to ~/.zeroclaw/config.toml
├── skills/                         # markdown files the agent reads
│   ├── meteora-position.md         # T0: read position, compute IL, in/out-of-range
│   ├── meteora-report.md           # T0: daily report format (Telegram)
│   ├── meteora-claim.md            # T1: build unsigned claimFee tx, return Action URL
│   └── meteora-rebalance.md        # T1: build unsigned removeLiquidity + addLiquidity, durable nonce
├── sops/                           # ZeroClaw cron SOPs (TOML + SOP.md)
│   ├── dlmm-daily-report/          # cron 08:00 daily position report
│   └── dlmm-range-monitor/         # cron */30 minute out-of-range alert
├── action-endpoint/                # self-hosted Solana Action endpoint (Cloudflare Worker)
│   ├── src/
│   │   ├── index.ts                # GET (metadata) + POST (unsigned tx)
│   │   └── dlmm.ts                 # tx builders (claim, rebalance)
│   ├── wrangler.toml
│   └── package.json
├── prompts/
│   └── injection-tests.md          # adversarial transcripts — required for safety scoring
├── showcase/
│   ├── video-script.md             # 3-min phone + terminal walkthrough
│   └── demo-transcript.md          # copy-pasteable runbook for the video
├── SUBMISSION.md                   # showcase write-up (what/who/feats/built/tier/threat model)
└── plugins/
    └── dlmm-reader/                # T3 bonus: wasm32-wasip2 plugin (own repo, no registry PR)
        ├── Cargo.toml
        ├── manifest.toml
        ├── src/
        │   ├── lib.rs              # thin shim
        │   └── decoder.rs          # pure core (host-testable)
        ├── wit/world.wit
        └── tests/
```

## Quick start (an evening)

> Tested on a fresh Linux box with stock ZeroClaw `master` binary.

### 1. Install ZeroClaw

```bash
curl -fsSL https://raw.githubusercontent.com/zeroclaw-labs/zeroclaw/master/install.sh | bash
zeroclaw quickstart
```

Pick Anthropic (or any provider). The agent's name will be `dlmm-copilot`.

### 2. Set up Telegram

Talk to [@BotFather](https://t.me/BotFather), create a bot, copy the token. Find your
chat id via [@userinfobot](https://t.me/userinfobot). Store both in
`~/.zeroclaw/secrets/telegram.env`:

```bash
TELEGRAM_BOT_TOKEN=<your-bot-token>
TELEGRAM_CHAT_ID=<your-chat-id>
```

### 3. Configure RPC + price feed

Pyth Hermes unauthenticated endpoints **stop serving on 2026-07-31**. We use
**Jupiter Price API** as the primary price feed. Public, unauthenticated,
rate-limited — fine for cron alerts.

```bash
# config.example.toml already has placeholders; copy and fill in:
cp config.example.toml ~/.zeroclaw/config.toml
# edit RPC URL (Helius / Triton / QuickNode / your own)
```

The RPC URL may carry the provider's API key as a query parameter (e.g.
Helius `?api-key=...`). To keep that key out of approval cards, the
agent's risk profile must auto-approve `http_request` — see the risk
profile section in `config.example.toml`. `[http_request.secrets]` is for
providers that accept Bearer auth (custom headers like `x-api-key` are not
supported by ZeroClaw's `http_request` tool — only `Authorization`).

### 4. Deploy the Action endpoint

The agent returns Solana Action URLs; the endpoint that serves them is a tiny
Cloudflare Worker you self-host. **No Dialect / no third party.**

```bash
cd action-endpoint
npm install
npx wrangler deploy
# → https://dlmm-copilot.<your-subdomain>.workers.dev
```

Set the secrets before/after deploy (rebalance is disabled until
`NONCE_ACCOUNT`/`NONCE_AUTHORITY` are set; `NONCE_AUTHORITY` **must be your
own signing wallet** — the endpoint 403s anyone else):

```bash
npx wrangler secret put RPC_URL
npx wrangler secret put NONCE_ACCOUNT    # rebalance only
npx wrangler secret put NONCE_AUTHORITY  # rebalance only
```

Local dev: `npx wrangler dev` reads the same vars from `.dev.vars`
(gitignored, format `NAME=value` per line). Smoke-tested locally:
`/health` 200, metadata GETs 200, missing account → 400, unauthorized
rebalance operator → 403, invalid bin range → 400, upstream failure → 500
generic (no internals leaked).

Put the worker URL in `config.toml` under `[skills.meteora]`.

### 5. Load the skills + SOPs

```bash
mkdir -p ~/.zeroclaw/skills ~/.zeroclaw/sops
cp skills/*.md ~/.zeroclaw/skills/
cp -r sops/dlmm-* ~/.zeroclaw/sops/
```

### 6. Run

```bash
zeroclaw service install
zeroclaw service start
```

Wait for 08:00. The first daily report arrives in Telegram.

### 7. (Bonus) Build the WASM plugin

`plugins/dlmm-reader` is the Tier-3 bonus — a real `wasm32-wasip2` plugin
(decoder + IL calculator) that could be loaded by any ZeroClaw runtime that
supports the `tool` capability. The compiled `.wasm` is **gitignored**, so a
fresh clone must build it once:

```bash
make plugin-build    # rustup target add wasm32-wasip2 + cargo build --release
make plugin          # host-side cargo tests (no wasm toolchain needed)
```

Output lands at `plugins/dlmm-reader/dlmm_reader.wasm`, matching
`manifest.toml`'s `wasm_path`. Without this step the main agent still runs —
T0/T1 never depend on the plugin — the plugin is pure bonus.

## Custody tier

| Operation | Tier | Secrets held | Who signs |
|-----------|------|--------------|-----------|
| Read position / price / IL | **T0** | RPC key (in `config_read`) | none |
| Daily report | **T0** | RPC key | none |
| Out-of-range alert | **T0** | RPC key | none |
| Build unsigned claim / rebalance tx | **T1** | none | **user wallet** via Action |
| Auto-compound (designed, off by default) | T2 | session key + caps | session key |

## Threat model (short)

- **Channel = prompt-injection surface.** Telegram DMs are user-controlled. The agent
  must not authorize any fund-moving action from a DM alone; it only **proposes** via
  an Action URL, the user's wallet signs. The risk profile hard-blocks `memory_recall`,
  `content_search`, `glob_search`, `file_read`, `file_write`, `file_edit`, and
  `data_management` from non-CLI channels via `excluded_tools` — the agent cannot hunt
  for `.env` files or config secrets through the Telegram channel. Only `read_skill`,
  `http_request`, and `send_message_to_peer` are auto-approved. See
  `prompts/injection-tests.md`.
- **RPC key exposure.** Provider API keys (Helius) live in the RPC URL as
  a query param. Auto-approving `http_request` in the agent's risk profile
  prevents the URL from being displayed in approval cards; the key remains
  on-disk in the skill file and `zeroclaw.env`, same level as the Telegram
  bot token. ZeroClaw's `http_request` tool only supports secrets for the
  `Authorization` header, so any other header key would have to be inlined
  into every LLM tool call — worse than the URL form. Rotate the key on
  the provider dashboard if it ever leaks elsewhere (chat, screenshots).
- **Third-party trust.** Jupiter (public, read-only), Cloudflare (hosting the
  Action endpoint), and Helius/your RPC. Declared in `SUBMISSION.md` § Threat model.
- **Blockhash expiry.** T1 rebalance uses **durable nonces** — approval queues can
  outlive the ~90 s blockhash window. One nonce account per concurrent pending tx.
- **Feed reliability.** Jupiter Price API is the primary feed (public,
  unauthenticated). Pyth Hermes deprecated 2026-07-31; Switchboard Crossbar
  DNS went dark 2026-08. No demo runs on a dead endpoint.

## Reproducing the demo

1. `git clone https://github.com/<you>/dlmm-lp-copilot`
2. Follow Quick start above
3. On Devnet, fund a wallet, open a DLMM position on the public DLMM program
4. The agent reads it, sends a daily report, prepares a rebalance Action URL
5. `showcase/demo-transcript.md` walks through the exact commands

## What this is not

- Not a trading bot. No token recommendations, no sniping.
- Not a T2 auto-compounder. T2 is designed but disabled (see write-up).
- Not a PR to `zeroclaw-labs/zeroclaw-plugins`. The optional `plugins/dlmm-reader`
  lives here in this repo; the bounty says not to open registry PRs during judging.

## License

MIT — see `LICENSE`.

## Also a starter kit

`STARTER.md` documents the project as a **forkable template** for other
DLMM-on-ZeroClaw (or any protocol-on-ZeroClaw) use cases. What to copy
verbatim, what to change, what to leave alone, and what the validation
script will catch.

```bash
make help          # list targets
make validate      # parse TOML + skill frontmatter + cargo test
make plugin        # cargo test the bonus wasm plugin
make worker-dev    # wrangler dev the action endpoint locally
make worker-dep    # wrangler deploy
make demo          # run the end-to-end Devnet plumbing checks
make submit-checklist
```
