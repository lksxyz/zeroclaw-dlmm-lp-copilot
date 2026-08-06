# DLMM LP Copilot — Superteam Brasil bounty submission

> `#solana-bounty` showcase post. [Video](./showcase/video-script.md) · [Repo](https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot)

## What it does

A ZeroClaw agent in your Telegram that watches Meteora DLMM positions: daily
report at 08:00, out-of-range alerts every 30 min, and on-demand `claim` /
`rebalance` via Solana Action URL. **The Solana work runs in two WASM plugins**
— fetch, decode, validate, and encode — the agent only proposes, you sign in
Phantom. No keys held.

## Who it's for

Active Solana LP with 3-5 DLMM positions who doesn't want to check dashboards.
Brazilian LPs get `America/Sao_Paulo` timezone (configurable).

## ZeroClaw features used

- Telegram channel plugin (registry)
- SOP engine with cron triggers (`dlmm-daily-report`, `dlmm-range-monitor`)
- Agent-driven DM handling — Telegram DMs flow through agent runtime (ZeroClaw
  only sets `internal_sop_event` for git/forge channels)
- Memory (position baselines, alert dedupe, pending-tx ledger)
- **WASM tool plugins** (wit/v0 registry): `dlmm_reader` + `dlmm_builder` with
  `http_client` + `config_read` grants, host-injected anti-spoof `__config`
- `read_skill` sandboxing
- `http_request` **denied** — all outbound runs in-wasm under plugin config

## What we built

1. **`plugins/dlmm-core`** — shared pure Rust core, host-testable: borsh
   `PositionV2`/`LbPair`/`BinArray` decoding, durable-nonce helpers, claim and
   rebalance instruction encoding, `waki` wasi:http RPC client (getAccountInfo,
   getProgramAccounts, nonce status), and the mechanical validation rules.
2. **`plugins/dlmm-reader`** — T0 shim: positions discovery (getProgramAccounts
   + owner memcmp), per-position reports (range, active bin, claimable, owner
   match), and nonce settlement-status checks for the SOP cleanup path.
3. **`plugins/dlmm-builder`** — T1 shim: validates mechanically (ownership,
   claimable > 0, liquidity > 0, range contains the live active bin, bin-array
   indexes in-bounds), builds the unsigned tx with `AdvanceNonceAccount` first
   and the **live nonce hash** as `recentBlockhash`, and returns a complete
   `solana-action:` URL. LLM proposes — plugin verifies.
4. **Stateless Solana Action relay** (`action-endpoint/`): Cloudflare Worker
   with **zero secrets, zero RPC, zero SDK deps** (~150 lines). GET renders the
   wallet preview *from the tx bytes* (instruction count, program IDs, nonce
   first, signer); POST echoes the tx and rejects any signer ≠ fee payer with
   403. Single-operator by construction.
5. **Four skills** (`skills/meteora-*.md`): T0 read/report, T1 claim, T1
   rebalance with durable-nonce settlement confirmation.
6. **Ground-truth fixture suite** (`tools/gen-fixtures.cjs`): txs built with the
   official `@meteora-ag/dlmm` + `@solana/web3.js` SDK, then cross-checked
   byte-for-byte by the Rust core — `make fixtures` regenerates, CI fails on
   drift.

Plugins are the only code that talks to Solana. Relay is the only HTTP surface.
Both reproducible from this repo.

## Custody tier

| Operation | Tier | Secrets | Signs |
|---|---|---|---|
| Read / report / alert | T0 | plugin `__config` (RPC, owner) | none |
| Build unsigned claimFee | T1 | none | user wallet |
| Build atomic rebalance (durable nonce) | T1 | none | user wallet |
| Auto-compound | T2 | disabled | n/a |

T2 reasoning: bounty warns "safety most often lost" at T2. Submitted config has
no session key, no autocompound block, no sign capability. Skills have no
`sign_and_submit`. The agent *refuses by construction*.

## Threat model

**Channel = prompt-injection surface.** Agent only proposes Action URLs. User
signs. Full adversarial transcript: `prompts/injection-tests.md` (7 scenarios).

**No LLM outbound at all.** `http_request`, `web_fetch`, `browser`,
`web_search_tool`, and every filesystem tool sit in
`risk_profiles.dlmm.excluded_tools` — denied, not approval-gated. RPC calls
happen only in-wasm under the host-injected `__config`, which strips any
caller-supplied `__config` (anti-spoof). The builder refuses to run without
`owner_pubkey` and rejects positions it doesn't own.

**Blockhash expiry.** Every proposed tx uses a durable nonce — valid past the
90s blockhash window. One nonce per concurrent pending tx; settlement is
verified by the nonce hash changing on-chain before a new proposal is allowed.

**Third-party trust:** Jupiter (read-only price), Cloudflare (relay host),
RPC (user-supplied, in plugin config). Declared.

**Mechanical validation.** Ownership, claimable > 0, liquidity > 0, low < high,
range contains the active bin, bin-array indexes inside the default bitmap —
checked in-wasm before a single tx byte is serialized.

## What we did NOT do

- No trading bot, sniping, buy recommendations
- No raw private key — no key in the system at all
- No concept/slideware — runs on Devnet in the video
- No registry PR — plugins live in this repo (bounty rule)
- No secrets in the relay — the worker is stateless; it can't leak what it
  doesn't hold

## Beyond the brief

- **All Solana work in WASM plugins** — the bounty's core ask (quorum/palinurus
  pattern): fetch, decode, validate, encode, all in-wasm with anti-spoof config
- **Solana Actions UX** — no submission pairs Blinks with an approval-gated agent
- **Stateless relay** — preview rendered from bytes; zero secret/KV/SDK surface
- **Durable nonces** (blockhash trap solved) for claim *and* rebalance
- **Byte-for-byte ground truth** — independent SDK sources, regenerable, CI-checked
- **Triple-gated defense**: LLM (skill rules) + Tool (denied-by-default) + Cryptographic (on-chain auth)

## Reproduce

```bash
curl -fsSL https://raw.githubusercontent.com/zeroclaw-labs/zeroclaw/master/install.sh | bash
zeroclaw quickstart
git clone https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot
cd zeroclaw-dlmm-lp-copilot

make plugin-build            # compile dlmm_reader.wasm + dlmm_builder.wasm

cp skills/*.md ~/.zeroclaw/skills/
cp -r sops/dlmm-* ~/.zeroclaw/sops/
cp config.example.toml ~/.zeroclaw/config.toml
# edit ~/.zeroclaw/config.toml — fill ${SOLANA_RPC_URL}, ${OPERATOR_WALLET_PUBKEY},
# ${ACTION_ENDPOINT_BASE} (used by the [plugins.entries.*.config] sections)

cd action-endpoint && npm install && npx wrangler deploy   # no secrets
cd ..
zeroclaw service install && zeroclaw service start
```

## Verify

- Code: this repo (plugins, skills, SOPs, relay, threat model — all in-tree)
- Ground truth: `make fixtures && make fixtures-check` (byte-for-byte)
- Plugins: `make plugin` (16 core + 9 reader + 10 builder host tests)
- Relay: `npm test && npm run typecheck` (4 tests, parse-vs-fixture)
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
