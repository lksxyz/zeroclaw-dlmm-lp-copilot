# DLMM LP Copilot — Superteam Brasil bounty submission

> `#solana-bounty` showcase post. [Video](./showcase/video-script.md) · [Repo](https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot)

## What it does

A ZeroClaw agent in your Telegram that watches Meteora DLMM positions: daily
report at 08:00, out-of-range alerts every 30 min, and on-demand `claim` /
`rebalance`. The agent builds an **unsigned transaction** — fetch, decode,
validate, and encode all run in two WASM plugins. The agent only proposes; you
sign with your own wallet. No keys held.

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
   and the **live nonce hash** as `recentBlockhash`, and returns the raw
   unsigned tx (base64). LLM proposes — plugin verifies.
4. **Sign + submit CLI** (`tools/execute`): signs the raw unsigned base64 tx
   with the operator keypair and broadcasts — stdin-pipe friendly for the
   demo flow (`./execute "$(bot reply)"`). Detects durable-nonce txs (keeps
   the stored nonce hash, never rewrites the blockhash) vs latest-blockhash
   txs (fresh blockhash). The agent never signs — the operator does, in any
   wallet or this CLI.
5. **Four skills** (`skills/meteora-*.md`): T0 read/report, T1 claim, T1
   rebalance with durable-nonce pool settlement confirmation.
6. **Ground-truth fixture suite** (`tools/gen-fixtures.cjs`): txs built with the
   official `@meteora-ag/dlmm` + `@solana/web3.js` SDK, then cross-checked
   byte-for-byte by the Rust core — `make fixtures` regenerates, CI fails on
   drift.

Plugins are the only code that talks to Solana — and the only HTTP surface.
Fully reproducible from this repo.

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

**Channel = prompt-injection surface.** Agent only proposes unsigned
transactions. User signs. Full adversarial transcript: `prompts/injection-tests.md`
(7 scenarios).

**No LLM outbound at all.** `http_request`, `web_fetch`, `browser`,
`web_search_tool`, and every filesystem tool sit in
`risk_profiles.dlmm.excluded_tools` — denied, not approval-gated. RPC calls
happen only in-wasm under the host-injected `__config`, which strips any
caller-supplied `__config` (anti-spoof). The builder refuses to run without
`owner_pubkey` and rejects positions it doesn't own.

**Blockhash expiry.** Every proposed tx uses a durable nonce from a
host-injected pool — valid past the 90s blockhash window. The pool
(`__config.nonce_addresses`, N entries) serializes N concurrent pending
approvals; the LLM hints which pool slot via `args.nonce_address` and the
plugin validates the hint is inside the pool (anti-spoof). Settlement is
verified by the nonce hash changing on-chain before a new proposal on
the same slot is allowed.

**Third-party trust:** Jupiter (read-only price), RPC (user-supplied, in
plugin config). Declared.

**Mechanical validation.** Ownership, claimable > 0, liquidity > 0, low < high,
range contains the active bin, bin-array indexes inside the default bitmap —
checked in-wasm before a single tx byte is serialized.

## What we did NOT do

- No trading bot, sniping, buy recommendations
- No raw private key — no key in the system at all
- No concept/slideware — runs on mainnet (demo runbook in-tree)
- No registry PR — plugins live in this repo (bounty rule)
- No relay / no extra HTTP surface — outbound is only plugin `http_client`
  (RPC + price), everything else is denied by policy

## Beyond the brief

- **All Solana work in WASM plugins** — the bounty's core ask (quorum/palinurus
  pattern): fetch, decode, validate, encode, all in-wasm with anti-spoof config
- **Unsigned tx as the deliverable** — the plugin hands back the raw unsigned
  tx; the operator signs in any wallet (Phantom, Solflare, CLI)
- **Sign/submit CLI** — `tools/execute` signs the raw base64 with the operator
  keypair and broadcasts; stdin-pipe friendly for the demo flow
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
# and ${DURABLE_NONCE_ADDRESS_1..N} (used by the [plugins.entries.*.config] sections)

zeroclaw service install && zeroclaw service start
```

## Verify

- Code: this repo (plugins, skills, SOPs, threat model — all in-tree)
- Ground truth: `make fixtures && make fixtures-check` (byte-for-byte)
- Plugins: `make plugin` (19 core + 9 reader + 16 builder host tests)
- Sign/submit: `tools/execute` (operator keypair, stdin-pipe friendly)
- Config: `make validate`
- Demo: `showcase/demo-transcript.md` (exact mainnet command sequence)
- Injection: `prompts/injection-tests.md` (7 scenarios, expected behavior per scenario)

## Future work

- T2 auto-compound behind Subscriptions & Allowances + session key + caps
- Squads v4 Proposer integration
- BRL reporting (economia.awesomeapi.com.br lookup)
- x402 oracle (paid LP analysis, $0.001/query)

## Links

- Repo: <https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot>
- Showcase video: https://youtu.be/Kt5I4j2-Qm0 (script: `showcase/video-script.md`)
- Live on-chain proof: https://solscan.io/tx/5WQ9Wie2doasd9aaeyoABbpisChAAZmas6D9GgDWDnrzg5eoYCFFPYaj7uFmM29XdGFUsUgZnCRYkSyTVpK6CVAD
- Demo runbook: `showcase/demo-transcript.md`
- Discord: <https://discord.gg/zeroclaw> → `#solana-bounty`
- Bounty: Superteam Earn — Build Solana-native plugins for ZeroClaw
