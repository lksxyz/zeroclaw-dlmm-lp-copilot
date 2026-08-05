# DLMM LP Copilot — Superteam Brasil bounty submission

> Showcase post for `#solana-bounty` in the ZeroClaw Discord.
> [Video (≤ 3 min)](./showcase/video-script.md) · [Repo](./) · [README](./README.md)

## What it does

A self-hosted ZeroClaw agent that lives in your **Telegram** and watches your
[Meteora DLMM](https://app.meteora.ag/) positions. Every morning at 08:00 it
sends a formatted report. Every 30 minutes it checks for out-of-range
positions. When you tell it to `claim #<id>` or `rebalance #<id>`, it
builds an **unsigned** transaction and hands it to you as a Solana Action
URL — you sign in Phantom, the agent never sees your key.

It is the *"DeFi guardian that wakes you only when your position needs
you"* pattern from the bounty brief, applied to Meteora DLMM and
delivered through chat.

## Who it's for

An active Solana LP holding 3-5 DLMM positions who:

- Doesn't want to log into a dashboard every day
- Wants the out-of-range alert to reach them, not the other way around
- Prefers to sign a transaction they can preview over a bot that
  auto-compounds on their behalf
- Runs their own machine and prefers their agent to do the same

Brazilian LPs running USDC/SOL or stable-stable pools get a daily report
in `America/Sao_Paulo` timezone (configurable); the skill is timezone-agnostic.

## Which ZeroClaw features it uses

- **Telegram channel plugin** — `plugins/telegram` from the upstream
  `zeroclaw-labs/zeroclaw-plugins` registry
- **SOP engine with cron triggers** — `sops/dlmm-daily-report/` and
  `sops/dlmm-range-monitor/`
- **Agent-driven on-demand DMs** — Telegram DMs matching
  `^(claim|rebalance|report)\s*#?\d*\s*$` are handled by the agent reading
  the meteora skills (`meteora-position` for report, `meteora-claim` /
  `meteora-rebalance` for T1 tx building). ZeroClaw's `ChannelMessage`
  only populates `internal_sop_event` for git/forge channels, so Telegram
  DMs flow through the agent runtime (not channel-triggered SOPs) by
  design. On-demand operations follow the same skill pipeline as cron.
- **Memory** — position value baselines + alert dedupe (4h TTL)
- **Built-in `http_request` / `web_fetch`** — every external call goes
  through the host's HTTP egress, with the standard private-host block
  for SSRF safety
- **Skill sandboxing** — skills are markdown files loaded from
  `~/.zeroclaw/skills/`, no compile step

## What we had to build

The off-the-shelf ZeroClaw release is enough for the read path. Two
artifacts are submission-specific:

1. **Four skills** under `skills/`:
   - `meteora-position.md` — T0 read + IL math
   - `meteora-report.md` — T0 format for Telegram
   - `meteora-claim.md` — T1 build unsigned claim, return Action URL
   - `meteora-rebalance.md` — T1 build atomic remove+add on durable nonce
2. **A self-hosted Solana Action endpoint** at `action-endpoint/`
   (Cloudflare Worker) that:
   - Serves the [Solana Action spec](https://github.com/solana-labs/solana-pay/blob/master/SPEC.md) GET/POST
   - Encodes the DLMM `claimFee` and `removeLiquidity + addLiquidityByStrategy`
     instructions via the Meteora SDK
   - Wraps the rebalance in a **durable nonce** for the user's signing window
   - No keys, no signing — returns base64 unsigned txs only
   - Single-operator deployment: the rebalance operator IS the nonce
     authority (the LP's own wallet); the endpoint rejects any other
     signer with 403, and the nonce path is fully covered by unit tests
     (see `action-endpoint/test/nonce.test.ts`)

The skills are pure markdown; the action endpoint is the only non-stock
piece. Both are reproducible from this repo.

## Custody tier

| Operation | Tier | Secrets held | Who signs |
|-----------|------|--------------|-----------|
| Read position, fetch pool state, mark price | **T0** | RPC key (in `config_read`) | none |
| Daily report, OOR alert, fee-milestone DM | **T0** | RPC key | none |
| Build unsigned `claimFee` tx, return Action URL | **T1** | none | **user wallet** |
| Build atomic `removeLiquidity + addLiquidityByStrategy` on durable nonce | **T1** | none | **user wallet** |
| Auto-compound | *T2, designed but disabled* | n/a | n/a |

**T2 reasoning**: the bounty says T2 is "where safety is most often lost".
The submitted config does not include a session key, no `autocompound`
block, no sign capability. If a judge prompt-injects the agent and asks
it to auto-compound, the agent refuses by *construction* — the
`meteora-*` skills have no `sign_and_submit` capability, and the worker's
T2 path is not exposed. The T2 design (Subscriptions & Allowances
on-chain caps + session key + SOP checkpoint) is sketched in
`prompts/injection-tests.md` Scenario 3 and `SUBMISSION.md` § Future work.

## Threat model

**Channel = prompt-injection surface.** Telegram DMs are user-controlled.
The agent must not authorize any fund-moving action from a DM alone; it
only *proposes* via an Action URL, the user's wallet signs. The full
adversarial transcript is in [`prompts/injection-tests.md`](./prompts/injection-tests.md)
(six scenarios, including an attacker replaying the Action URL and an
attacker asking for auto-signing).

**Blockhash expiry.** Approval queues routinely outlive the ~60-90 s
recent-blockhash window. The rebalance path uses a **durable nonce**:
`AdvanceNonceAccount` is the first ix, the stored nonce is the
"blockhash", and the tx stays valid as long as the user needs to sign.
One nonce account per concurrent pending rebalance — a known limitation,
documented in `skills/meteora-rebalance.md` and the worker comments.

**Pyth deprecation 2026-07-31.** Jupiter Price API is the primary
price feed. Public, unauthenticated, rate-limited — fine for cron alerts
and on-demand reports. The agent does not demo on Pyth Hermes
(unauthenticated endpoints stop serving 2026-07-31, already past) or
Switchboard Crossbar (DNS went dark 2026-08).

**Third-party trust declared:**

- **Jupiter Price API** (public, read-only feed) — rate-limited, no key
  needed
- **Cloudflare** (hosts the action endpoint) — worker source is in this
  repo, reproducible
- **Helius / user RPC** (Solana JSON-RPC) — user-supplied, declared in
  `config.example.toml`
- **Meteora DLMM SDK** (`@meteora-ag/dlmm`) — used by the worker to
  encode the `claimFee`, `removeLiquidity`, `addLiquidityByStrategy`
  instructions. The SDK is invoked server-side in the worker; the agent
  itself never imports it.

The agent holds no private keys. The wallet holds all funds. The agent
holds an RPC key (encrypted at rest via `config_read`). Price data comes
from Jupiter Price API (public, unauthenticated).

## Reproducibility — set this up in an evening

```bash
# 1. Install ZeroClaw
curl -fsSL https://raw.githubusercontent.com/zeroclaw-labs/zeroclaw/master/install.sh | bash
zeroclaw quickstart

# 2. Copy skills, SOPs, config
git clone https://github.com/<you>/dlmm-lp-copilot
cp dlmm-lp-copilot/skills/*.md  ~/.zeroclaw/skills/
cp -r dlmm-lp-copilot/sops/dlmm-*  ~/.zeroclaw/sops/
cp dlmm-lp-copilot/config.example.toml  ~/.zeroclaw/config.toml
# edit config.toml, fill in RPC + Telegram

# 3. Deploy the action endpoint
cd dlmm-lp-copilot/action-endpoint
npm install
wrangler secret put RPC_URL
wrangler secret put NONCE_ACCOUNT    # rebalance only: your own nonce account
wrangler secret put NONCE_AUTHORITY  # rebalance only: must be YOUR signing wallet
wrangler deploy

# 3b. (Optional) bonus WASM plugin — the .wasm is gitignored, rebuild after clone
make plugin-build

# 4. Start the agent
zeroclaw service install
zeroclaw service start
```

`README.md` has the long-form version; `showcase/demo-transcript.md` has
the exact Devnet commands.

## What we did *not* do (per bounty)

- **No trading bot, sniper, or "buy this token"** — rebalance is
  bin-range movement on the same position, never a swap, never a buy.
- **No raw private key with no caps, no allowlist, no approval gate** —
  there is no raw private key in the system at all.
- **No concept / mockup / slideware** — the agent runs in Devnet in the
  video, and the video shows a real on-chain transaction with a real
  Solana Explorer link.
- **No standalone plugin PR** — the optional `plugins/dlmm-reader` lives
  in this submission's own repo. The bounty says not to open registry
  PRs during judging; we follow that.
- **No thin RPC wrapper in WASM** — every T0 read shapes output to a
  ~200-token per-position block to avoid flooding the model context
  (the bounty explicitly calls this out as a trap).

## What we built beyond the brief

- **Self-hosted Action endpoint** instead of relying on Dialect. The
  bounty endorses self-hosting as the "no third party" path; we did it.
- **Durable nonces for the rebalance path.** The bounty flags blockhash
  expiry as a trap and calls it "worth points" to solve well. We solved
  it.
- **Jupiter Price API as the price feed.** Pyth Core deprecated
  2026-07-31 and Switchboard Crossbar DNS went dark 2026-08. We use
  Jupiter with no API key, no third party, and we call this out in the
  threat model.
- **File-system hard-block via `excluded_tools`.** The risk profile
  removes `memory_recall`, `content_search`, `glob_search`, `file_read`,
  `file_write`, `file_edit`, and `data_management` from the agent when
  operating on non-CLI channels. This prevents the agent from hunting
  for `.env` files, config secrets, or credentials in the workspace —
  a real incident we caught and hardened against (see
  `prompts/injection-tests.md` Scenario 7).
- **Seven-scenario prompt-injection transcript.** The bounty requires
  one; we shipped seven, covering the LLM, the wallet, the file-system,
  and the on-chain program boundaries.

## Future work (designed, not implemented)

- **T2 auto-compound** behind Subscriptions & Allowances on-chain caps,
  with a session key that holds a fixed SOL/USDC allowance, a
  `mint_allowlist = [DLMM_PROGRAM]`, and an SOP checkpoint before each
  compound. Disabled by default in the submitted config.
- **Squads v4 Proposer integration** — the strongest pattern the
  bounty calls out. Heavy to build (Anchor-style instruction encoding
  + PDA derivation), and the bounty itself recommends "get your use
  case working end to end with a simpler approval path first; add
  Squads last." That's the next iteration.
- **BRL reporting** — Brazilian-first flows are welcomed. We report in
  USD; adding BRL is a one-evening `economia.awesomeapi.com.br` lookup
  and a number formatter, with a config flag.
- **x402 oracle** — selling the LP analysis as a paid service
  ($0.001/query, paid via x402, hard per-day cap in code). Out of
  scope here; sketched in the brainstorm notes.

## How to verify

- **Code:** this repo. Skills, SOPs, action endpoint, threat model —
  all in-tree.
- **Action endpoint tests:** `cd action-endpoint && npm test` — unit
  tests for the durable-nonce serialization (`nonce.test.ts`);
  `npm run typecheck` must stay green. The worker has also been smoke-tested
  end-to-end locally (`wrangler dev`): `/health` 200, metadata GETs 200,
  missing account → 400, unauthorized rebalance operator → 403, invalid bin
  range → 400, upstream failure → generic 500 with no internals leaked.
- **WASM plugin:** `make plugin` (7 host-side cargo tests) and
  `make plugin-build` (compile `dlmm_reader.wasm`, gitignored).
- **Config:** `config.example.toml` is the redacted version; copy to
  `~/.zeroclaw/config.toml` and fill in the env vars.
- **Demo:** `showcase/video-script.md` is the video plan;
  `showcase/demo-transcript.md` is the exact Devnet command sequence
  to reproduce it.
- **Injection tests:** `prompts/injection-tests.md` — six scenarios
  with expected behavior, ready to replay against any deployment.

## Links

- **Repo:** https://github.com/&lt;you&gt;/dlmm-lp-copilot
- **Video:** see `showcase/video-script.md` (script) + final cut link when posted
- **Discord showcase post:** #solana-bounty in the ZeroClaw Discord
- **Bounty brief:** Superteam Earn — Build Solana-native plugins for ZeroClaw
