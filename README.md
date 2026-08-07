# DLMM LP Copilot 🦞

Self-hosted ZeroClaw agent watching [Meteora DLMM](https://app.meteora.ag/) positions via
**Telegram**. The Solana work — RPC fetch, on-chain decoding, unsigned tx building —
runs inside two WASM plugins. The agent proposes; you sign in any wallet. No keys held.

> Superteam Brasil bounty: T0 (read) + T1 (build unsigned tx). Winner announcement 2026-08-21.

## Features

- **Daily 08:00 (operator TZ, default `America/Sao_Paulo`)** — position report (range, active bin, claimable fees)
- **Every 30 min** — out-of-range + fee-milestone alert
- **DM `report`** — on-demand position report
- **DM `claim #<id>`** → raw unsigned claim tx (base64) → sign in any wallet
- **DM `rebalance #<id> [wide|tight]`** → atomic 3-ix unsigned tx on a durable nonce from a pool (parallel pending approvals) → sign in any wallet
- **`tools/execute`** — sign + submit the base64 with the operator keypair (or paste it into Phantom/Solflare); durable-nonce txs keep their nonce hash, never rewritten

## Architecture

```
ZeroClaw runtime (Telegram, SOPs, memory)
   ↓ tool calls
dlmm_reader (WASM plugin, T0)          dlmm_builder (WASM plugin, T1)
  fetch RPC in-wasm (waki wasi:http)     fetch position/lb_pair/nonce in-wasm
  decode PositionV2, compute claimable    validate mechanically (owner, claimable,
  discover positions (getProgramAccounts)  range contains active bin)
  status mode: nonce settlement check     encode claim / rebalance (durable nonce)
   ↑ anti-spoof __config (RPC_URL, OWNER_PUBKEY, DLMM_PROGRAM) injected by host
   ↑ shared pure core: plugins/dlmm-core (host-testable Rust)
   ↓
unsigned tx (base64)   →   your wallet signs (Phantom / Solflare / CLI).
Agent never holds a key.
```

All Solana traffic runs inside the plugins under a host-injected `__config` —
the LLM cannot redirect the RPC, cannot read secrets, and `http_request` is
**denied** in `excluded_tools`. Every outbound path is plugin-only.

## Repo layout

```
plugins/
  dlmm-core/        # shared pure core: borsh decode, nonce helpers, tx encoding,
                    #   waki RPC client, mechanical validation (host tests, 19)
  dlmm-reader/      # T0 shim → dlmm_reader tool (positions/report/status modes)
  dlmm-builder/     # T1 shim → dlmm_builder tool (claim/rebalance → unsigned tx)
  wit/              # ZeroClaw registry wit/v0, pinned (see UPSTREAM_REF.md)
skills/             # Agent-readable markdown (loaded every invoke)
  meteora-position.md    T0: discover + report via dlmm_reader
  meteora-report.md      T0: Telegram format
  meteora-claim.md       T1: claim → unsigned tx
  meteora-rebalance.md   T1: rebalance → unsigned tx (durable nonce pool)
sops/               # Cron definitions
  dlmm-daily-report/     08:00 report
  dlmm-range-monitor/    */30 OOR check + settlement cleanup
tools/              # Fixture generator (web3.js + @meteora-ag/dlmm) + ./execute CLI
scripts/            # submit-checklist, validate-config, validate-skills, x-post
prompts/            # injection-tests.md (7 scenarios) + recorded transcripts
showcase/           # Video, demo runbook, Discord post body
config.example.toml
DEPLOY.md           # Self-host walkthrough (Rocky VPS, source build, nonce pool)
SUBMISSION.md       # Bounty submission write-up
STARTER.md          # Forkable template for any protocol-on-ZeroClaw use case
BUILD_LOG.md        # Build-in-public milestones (X tiebreak source)
```

## Showcase

- **Video:** https://youtu.be/Kt5I4j2-Qm0
- **Live on-chain proof** (the mainnet rebalance shown in the video):
  https://solscan.io/tx/5WQ9Wie2doasd9aaeyoABbpisChAAZmas6D9GgDWDnrzg5eoYCFFPYaj7uFmM29XdGFUsUgZnCRYkSyTVpK6CVAD
- Demo runbook: `showcase/demo-transcript.md` (copy-pasteable mainnet commands)
- Discord post body: `showcase/discord-post.md`

## Quick start

> Running this on a clean VPS (Rocky Linux) with OpenRouter as the LLM
> provider? Follow [`DEPLOY.md`](DEPLOY.md) — the prebuilt ZeroClaw binary
> lacks the WASM plugin host, so a source build is required.

```bash
# 1. Install ZeroClaw
curl -fsSL https://raw.githubusercontent.com/zeroclaw-labs/zeroclaw/master/install.sh | bash
zeroclaw quickstart

# 2. Telegram bot (BotFather → bot token; userinfobot → chat id)
#    Store in ~/.zeroclaw/secrets/telegram.env

# 3. Copy config and fill placeholders — the plugin sections take
#    SOLANA_RPC_URL / OPERATOR_WALLET_PUBKEY / DURABLE_NONCE_ADDRESS_{1,2,3}
#    (one nonce account per pool slot; create them with
#    `solana create-nonce-account`, see DEPLOY.md §7)
cp config.example.toml ~/.zeroclaw/config.toml

# 4. Build the WASM plugins (needs rustup target wasm32-wasip2)
make plugin-build

# 5. Load skills + SOPs
mkdir -p ~/.zeroclaw/skills ~/.zeroclaw/sops
cp skills/*.md ~/.zeroclaw/skills/
cp -r sops/dlmm-* ~/.zeroclaw/sops/

# 6. Start
zeroclaw service install && zeroclaw service start

# 7. Sign + submit an unsigned tx from the bot with the operator keypair:
./tools/execute "<base64 from the bot>"
#    or pipe it: echo "<base64>" | ./tools/execute
```

## Custody tier

| Operation | Tier | Signs |
|---|---|---|
| Read / report / alert | T0 | none |
| Build claim tx | T1 | user wallet |
| Build rebalance tx | T1 | user wallet (durable nonce) |
| Auto-compound | T2 | disabled by default |

## Threat model (short)

**Channel = prompt-injection surface.** Agent only *proposes* — user wallet signs.
Filesystem tools (`content_search`, `glob_search`, `file_read`, `file_write`,
`file_edit`, `data_management`, `memory_export`, `cron_list`), the web tools
(`web_fetch`, `browser`, `web_search_tool`, `weather`) and `http_request` sit
in `risk_profiles.dlmm.excluded_tools` — **denied outright**, not just
approval-gated. `memory_recall` reads position baselines (no secrets). See
`prompts/injection-tests.md` (7 scenarios).

**RPC can't be hijacked.** The LLM has no outbound tool; RPC calls only happen
in-wasm with the host-injected `__config` (anti-spoof: caller-supplied
`__config` is stripped). The builder additionally verifies position ownership
against `__config.owner_pubkey` before encoding anything.

**Durable nonce pool.** Every agent-proposed tx leads with `AdvanceNonceAccount`
and uses the live nonce hash as `recentBlockhash` — the tx survives approval
delays. The pool (N nonces, host-injected) allows N parallel pending
approvals; the LLM hints which slot via `args.nonce_address`, the plugin
validates the hint is inside the pool. One pending tx per nonce account;
a second proposal on the same slot fails atomically.

**Mechanical validation.** "LLM proposes, plugin verifies": ownership match,
claimable > 0, liquidity > 0, low < high, range contains the active bin —
before any bytes are serialized.

## What this is not

- Not a trading bot. No sniping, no token picks.
- Not T2 auto-compounder. Designed, disabled, out of scope.
- No registry PR. Plugins live in this repo per bounty rules.

## Also a starter kit

See `STARTER.md` — forkable template for any protocol-on-ZeroClaw use case.

```bash
make help       # targets
make validate   # TOML + skill frontmatter + all plugin tests
make fixtures   # regenerate ground-truth fixtures (byte-for-byte)
```

License: MIT.
