# Discord showcase post — DLMM LP Copilot

This file is the **literal body to paste** into `#solana-bounty` in the
[ZeroClaw Discord](https://discord.gg/zeroclaw). Fill the bracketed
placeholders (`[…]`), then post. The checklist below each section maps to a
specific scoring axis in the bounty ("Use case 30% / Safety 25% / Craft 20% /
Reproducibility 15% / Showcase 10% / Build-in-public tiebreak").

---

## Post body (copy from here ↓)

```
DLMM LP Copilot — self-hosted T0/T1 DLMM guardian on Telegram 🦞

A ZeroClaw agent that watches your Meteora DLMM positions on mainnet and only
pings you when something needs attention:
  • 08:00 [TZ] daily report — range, active bin, claimable fees
  • every 30 min — out-of-range + fee-milestone alerts
  • DM `claim #<id>` → raw unsigned claim tx (base64) → sign in any wallet
  • DM `rebalance #<id> [wide|tight]` → atomic 3-ix unsigned tx on a durable
    nonce from a host-injected pool → sign in any wallet

All the Solana work runs inside two WASM plugins: dlmm_reader (T0 — fetch
RPC in-wasm via waki, decode PositionV2, discover positions) and dlmm_builder
(T1 — mechanical validation, then encode the unsigned tx with
AdvanceNonceAccount first and the live nonce hash as recentBlockhash). The
plugin returns the raw unsigned tx as base64 — the operator signs in
Phantom/Solflare/CLI. No keys held by the agent, ever.

Blockhash trap solved at construction: every proposed tx uses a durable
nonce from a host-injected pool (N slots → N parallel pending approvals;
bounty "one nonce per in-flight tx" warning). The LLM hints which pool slot
via args.nonce_address; the plugin validates the hint is inside the pool
(anti-spoof — a prompt injection can't substitute an attacker-controlled
nonce). http_request is in excluded_tools — the LLM has no outbound at all;
RPC runs in-wasm under a host-injected anti-spoof __config. Byte-for-byte
ground truth vs the official @meteora-ag/dlmm SDK: make fixtures-check,
CI fails on drift.

Repo: https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot
Video: https://youtu.be/Kt5I4j2-Qm0
Runbook: showcase/demo-transcript.md (copy-pasteable mainnet commands)
Threat model + 7-scenario injection suite: prompts/injection-tests.md

Custody tier (honest):
  T0 — read/report/alert           no signing
  T1 — build claimFee / rebalance  user wallet signs (raw base64)
  T2 — auto-compound               disabled by construction, not by detection

Reproduce in an evening: README.md → Quick start. Operator-specific bits:
RPC URL, Telegram bot token, wallet pubkey, and the durable-nonce pool
addresses — all from ${ENV_VAR} placeholders in the plugin config sections.
```

---

## Checklist (jot before posting)

- [ ] Video uploaded; final URL in `[…]` placeholder above
- [ ] Repo URL confirmed reachable (https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot)
- [ ] `make validate` + `make fixtures-check` green on the version you're posting from
- [ ] Fail-closed auto-compound refusal recorded for the video (Scenario 3 in
      `prompts/injection-tests.md`)
- [ ] One X post linked in thread for the build-in-public tiebreak
      (see `BUILD_LOG.md` for the milestones)

## Scoring-axis map

| Bounty axis | Where it lives |
|---|---|
| Use case (30%) | First paragraph + mainnet claim + video |
| Safety & custody (25%) | Custody tier block + threat model link + injection suite link + nonce-pool anti-spoof |
| Craft (20%) | Repo layout, `plugins/*/tests/host_integration.rs`, fixture suite, modular solana crates |
| Reproducibility (15%) | "Reproduce in an evening" line + README quickstart + `make fixtures-check` |
| Showcase (10%) | Video itself + this Discord post structure |
| Tiebreak — build-in-public | One X post linking this thread, timestamped during the bounty window |
