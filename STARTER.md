# DLMM LP Copilot — Starter Kit

This repo is two things at once:

1. **A bounty submission** for the Superteam Brasil *"Build Solana-native
   plugins for ZeroClaw"* bounty (Telegram LP guardian for Meteora DLMM,
   T0 read + T1 unsigned tx, no keys held).
2. **A starter kit** for any DLMM-on-ZeroClaw use case — a forkable,
   submission-shaped template you can adapt to other protocols, other
   channels, other guardian patterns.

The submission is in `SUBMISSION.md`. This file is the starter-kit
manual: how the pieces fit, what to change for your own use case,
and what to leave alone.

---

## The 5-line mental model

```
ZeroClaw (Rust runtime, Telegram, SOPs, memory, MCP, http)
   ↓ reads
skills/*.md           — what the LLM knows how to do
   ↓ scheduled by
sops/*.toml           — when to do it (cron, channel trigger, approval)
   ↓ falls back to
http_request / web_fetch + memory    — agent's only outbound tools
   ↓ proposes actions through
action-endpoint/      — a self-hosted Solana Action (Blink) server
   ↓ user signs in
Phantom / Solflare    — agent never holds a key
```

If a use case needs to do something on Solana and the agent should not
hold a key, this is the shape. If a use case needs the agent to sign,
this shape is wrong — go to T2 with Subscriptions & Allowances instead.

## What to copy verbatim

These are the pieces that are *boring infrastructure* and should not
need to change between use cases:

- `config.example.toml` — the ZeroClaw config shape. Rename agents,
  channels, and `skills.meteora.*` blocks; leave `[risk_profiles.*]`,
  `[sops.triggers]`, `[memory]` alone.
- `sops/daily-report.toml` — the cron pattern (fetch → format → send →
  persist). Rename skills, keep the structure.
- `sops/range-monitor.toml` — the alert pattern (fetch → dedupe → branch
  → channel_send → persist_alert). The `branch` + `memory_check`
  dedupe is the key trick — copy it.
- `prompts/injection-tests.md` — the six scenarios. Add more for
  your use case; keep the same shape ("Threat → Expected behavior →
  Why it fails closed").
- `action-endpoint/` — the worker. Replace `dlmm.ts` with the protocol
  you're integrating. The `index.ts` GET/POST handlers, the cors
  plumbing, and the `prices.ts` Switchboard reader are reusable.

## What to change for a different protocol

- `skills/meteora-*.md` → `skills/<your-protocol>-*.md`. The shape
  is: read → shape → format-or-build → return Action URL.
- `action-endpoint/src/dlmm.ts` → `action-endpoint/src/<protocol>.ts`.
  The tx-builder structure (`buildClaim` / `buildRebalance` → base64
  unsigned tx) generalizes.
- `plugins/dlmm-reader/` → `plugins/<protocol>-reader/`. The pure
  core + thin shim + host tests pattern is the one the upstream
  README says to use.
- `SUBMISSION.md` and `showcase/` — your use case's story.

## What to leave alone

- The T0/T1 split. Bounty says T0 and T1 are the sweet spot; if
  you find yourself reaching for T2, sketch it in `prompts/`
  as a designed-but-disabled path first.
- The custody model. The agent holds an RPC key. Period. No session
  keys unless you've sketched the full T2 design in writing.
- The prompt-injection discipline. Every fund-moving path has a
  test in `prompts/injection-tests.md`. New actions = new tests.
- The durable nonce for any T1 path that goes through an approval
  queue. Blockhash expiry is a real trap, not a theoretical one.

## Files you'll need to author for a new use case

| You need | Author it in |
|----------|--------------|
| A new channel (e.g. Discord) | Add a `channels.*` block to `config.example.toml`; the upstream `zeroclaw-labs/zeroclaw-plugins` already has Discord, Matrix, etc. |
| A new skill | Add `skills/<name>.md` with frontmatter `name`/`version`/`custody`/`summary` |
| A new SOP | Add `sops/<name>.toml` with a `[trigger]` and `[steps.*]` list |
| A new protocol tx builder | Add `action-endpoint/src/<protocol>.ts` and import in `index.ts` |
| A new plugin (Tier 3) | Add `plugins/<name>/` with `Cargo.toml`, `manifest.toml`, `src/lib.rs`, `src/<core>.rs`, `tests/` |
| A new prompt-injection test | Append to `prompts/injection-tests.md` |

## What `Makefile` gives you

```bash
make help        # list targets
make validate    # TOML parse + skill frontmatter + plugin cargo check
make plugin      # cargo test the dlmm-reader plugin
make plugin-build  # compile dlmm_reader.wasm (gitignored — rebuild after clone)
make worker-dev  # wrangler dev the action endpoint locally
make worker-dep  # wrangler deploy the action endpoint
make demo        # run the end-to-end Devnet demo
```

## What the CI checks (`.github/workflows/ci.yml`)

- All TOML files parse
- All skill markdown has the required frontmatter
- `plugins/dlmm-reader` builds with `cargo check --target wasm32-wasip2`
  (best-effort — WASI target is optional, soft-fails on non-wasi runners)
- `action-endpoint` type-checks with `tsc --noEmit`

If you fork this for another use case, keep the same CI shape. It is
the cheapest way to catch a malformed config before judging.

## When to NOT use this starter

- **Trading bots, snipers, MEV.** The starter is for guardians,
  not for agents that need to win races.
- **T2 with auto-sign.** This starter is T0/T1 by design. If you
  need auto-sign, start from a different shape: Subscriptions &
  Allowances + session key + SOP checkpoint, all sketched in
  writing before any code.
- **Use cases that don't fit Telegram/Discord/etc.** The starter
  assumes chat channels. If your agent is API-only, drop the
  channel blocks and the `sops/triggers.dm` block.

## How to ship a fork

1. `git init` and copy the files you want
2. Replace `meteora-*` with your protocol name throughout
3. Re-author the SOPs and skills for your use case
4. Add at least 3 prompt-injection scenarios for any fund-moving path
5. Run `make validate` until clean
6. Write your own `SUBMISSION.md` (the structure here is a template,
   not a fill-in)
