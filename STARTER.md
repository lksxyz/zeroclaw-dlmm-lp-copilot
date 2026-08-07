# DLMM LP Copilot — Starter Kit

This repo is a **forkable submission template** for any protocol-on-ZeroClaw use
case. The bounty submission lives in `SUBMISSION.md`. This file is the
starter-kit manual.

## Mental model

```
ZeroClaw runtime (Telegram, SOPs, memory)
   ↓ tool calls (in-wasm, host-injected __config)
plugins/<protocol>-reader     plugins/<protocol>-builder
  fetch + decode in-wasm        validate mechanically + encode tx
   ↓ propose
raw unsigned tx (base64)      — agent never holds a key
   ↓ signs
Phantom / Solflare / tools/execute (CLI)
```

If the agent must NOT hold a key, this shape fits. If it must sign, use
Subscriptions & Allowances (T2) instead.

Key rule: **the LLM has no outbound tools** (`http_request` etc. sit in
`excluded_tools`). All Solana traffic runs in plugins under an anti-spoof
`__config` injected by the host from `[plugins.entries.<name>.config]` — the
model can't redirect the RPC or read secrets it wasn't granted.

## Copy verbatim

- `config.example.toml` — risk profile shape, SOP triggers, plugin sections
  (`[plugins.entries.*.config]`), `excluded_tools` list
- `plugins/dlmm-core/` — pure core pattern (host-testable Rust, no wasm in tests)
- `plugins/dlmm-reader/` + `plugins/dlmm-builder/` — T0/T1 shim pattern
  (wit/v0 registry, `http_client` + `config_read`, parameters-schema)
- `sops/dlmm-daily-report/` + `sops/dlmm-range-monitor/` — cron SOP pattern
- `prompts/injection-tests.md` — scenario template (Threat → Expected → Why falls closed)
- `tools/execute` — sign/submit CLI pattern (operator keypair, stdin-pipe friendly)
- `tools/gen-fixtures.cjs` + `tools/package.json` — ground-truth generator
  pattern (independent SDK sources, `make fixtures-check` fails on drift)

## Change for a new protocol

- `skills/meteora-*.md` → `skills/<protocol>-*.md` (read → shape → format/build → return unsigned base64 tx)
- `plugins/dlmm-reader/` → `plugins/<protocol>-reader/` (pure core + shim + host tests)
- `plugins/dlmm-builder/` → `plugins/<protocol>-builder/` (validate + encode → raw unsigned tx base64)
- `config.example.toml` — plugin entries + config sections
- `SUBMISSION.md` + `showcase/` → your story

## Leave alone

- **T0/T1 split.** Bounty's sweet spot. T2 = sketch in prompts/ first.
- **Custody model.** Plugins hold the config, not the keys. No session keys without T2 design doc.
- **Prompt-injection discipline.** Every fund path = test in `prompts/injection-tests.md`.
- **Durable nonce** for any T1 path going through approval queues.
- **Deny-by-default.** No outbound tool for the LLM; plugin-only traffic.

## Files for a new use case

| Need | Where |
|---|---|
| New channel | `channels.*` in config; upstream has Discord, Matrix, etc. |
| New skill | `skills/<name>.md` (frontmatter: name/version/custody/summary) |
| New SOP | `sops/<name>/SOP.toml` + `SOP.md` |
| New tx builder | `plugins/<protocol>-builder/` (shim + manifest + parameters-schema) |
| New reader | `plugins/<protocol>-reader/` (same shape) |
| Injection test | Append to `prompts/injection-tests.md` |

## Makefile targets

```bash
make help          # list targets
make validate      # TOML parse + skill frontmatter + all plugin tests
make plugin        # cargo test dlmm-core + dlmm-reader + dlmm-builder
make plugin-build  # compile both .wasm (gitignored)
make fixtures      # regenerate ground-truth fixtures from SDK sources
make fixtures-check# regenerate and fail on drift
```

## When NOT to use

- Trading bots, snipers, MEV
- T2 auto-sign (start from Subscriptions & Allowances + session key + checkpoint)
- Non-chat channels (API-only agents → drop channel config blocks)

## Ship a fork

1. `git init`, copy files
2. Replace `meteora-*` with your protocol name
3. Re-author SOPs + skills + plugin manifests
4. Add ≥ 3 injection scenarios per fund-moving path
5. `make validate && make fixtures-check` until clean
6. Write your own `SUBMISSION.md`
