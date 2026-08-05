# DLMM LP Copilot — Starter Kit

This repo is a **forkable submission template** for any protocol-on-ZeroClaw use
case. The bounty submission lives in `SUBMISSION.md`. This file is the
starter-kit manual.

## Mental model

```
ZeroClaw runtime (Telegram, SOPs, memory, http)
   ↓ reads
skills/*.md           — what the LLM knows how to do
   ↓ scheduled by
sops/dlmm-*/          — cron SOPs: SOP.toml + SOP.md
   ↓ outbound
http_request + memory — agent's only tools
   ↓ proposes
action-endpoint/      — Solana Action server
   ↓ signs
Phantom / Solflare    — agent never holds a key
```

If the agent must NOT hold a key, this shape fits. If it must sign, use
Subscriptions & Allowances (T2) instead.

## Copy verbatim

- `config.example.toml` — risk profile shape, SOP triggers, memory config
- `sops/dlmm-daily-report/` + `sops/dlmm-range-monitor/` — cron SOP pattern
- `prompts/injection-tests.md` — scenario template (Threat → Expected → Why falls closed)
- `action-endpoint/` — Worker GET/POST handlers, CORS plumbing. Replace `dlmm.ts`.

## Change for a new protocol

- `skills/meteora-*.md` → `skills/<protocol>-*.md` (read → shape → format/build → return Action URL)
- `action-endpoint/src/dlmm.ts` → `action-endpoint/src/<protocol>.ts`
- `plugins/dlmm-reader/` → `plugins/<protocol>-reader/` (pure core + shim + host tests)
- `SUBMISSION.md` + `showcase/` → your story

## Leave alone

- **T0/T1 split.** Bounty's sweet spot. T2 = sketch in prompts/ first.
- **Custody model.** Agent holds RPC key only. No session keys without T2 design doc.
- **Prompt-injection discipline.** Every fund path = test in `prompts/injection-tests.md`.
- **Durable nonce** for any T1 path going through approval queues.

## Files for a new use case

| Need | Where |
|---|---|
| New channel | `channels.*` in config; upstream has Discord, Matrix, etc. |
| New skill | `skills/<name>.md` (frontmatter: name/version/custody/summary) |
| New SOP | `sops/<name>/SOP.toml` + `SOP.md` |
| New tx builder | `action-endpoint/src/<protocol>.ts`, import in `index.ts` |
| New plugin (Tier 3) | `plugins/<name>/` (Cargo.toml, manifest.toml, src/) |
| Injection test | Append to `prompts/injection-tests.md` |

## Makefile targets

```bash
make help          # list targets
make validate      # TOML parse + skill frontmatter + cargo check
make plugin        # cargo test dlmm-reader
make plugin-build  # compile dlmm_reader.wasm (gitignored)
make worker-dev    # wrangler dev
make worker-dep    # wrangler deploy
make demo          # end-to-end Devnet
```

## When NOT to use

- Trading bots, snipers, MEV
- T2 auto-sign (start from Subscriptions & Allowances + session key + checkpoint)
- Non-chat channels (API-only agents → drop channel config blocks)

## Ship a fork

1. `git init`, copy files
2. Replace `meteora-*` with your protocol name
3. Re-author SOPs + skills
4. Add ≥ 3 injection scenarios per fund-moving path
5. `make validate` until clean
6. Write your own `SUBMISSION.md`
