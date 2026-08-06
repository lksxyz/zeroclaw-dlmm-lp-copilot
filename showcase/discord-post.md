# DLMM LP Copilot — Discord showcase post

This file is the **literal body to paste** into `#solana-bounty` in the
[ZeroClaw Discord](https://discord.gg/zeroclaw). Fill the bracketed
placeholders (`[…]`), then post. The checklist below each section maps to a
specific scoring axis in the bounty ("Use case 30% / Safety 25% / Craft 20% /
Reproducibility 15% / Showcase 10% / Build-in-public tiebreak").

---

## Post body (copy from here ↓)

```
DLMM LP Copilot — self-hosted T0/T1 DLMM guardian on Telegram 🦞

A ZeroClaw agent that watches your Meteora DLMM positions and only pings you
when something needs attention:
  • 08:00 [TZ] daily report — range, active bin, claimable fees
  • every 30 min — out-of-range + fee-milestone alerts
  • DM `claim #<id>` → unsigned claimFee tx → sign in Phantom
  • DM `rebalance #<id> [wide|tight]` → atomic 3-ix tx on a durable nonce
    → sign in Phantom

All the Solana work runs inside two WASM plugins: dlmm_reader (T0 — fetch RPC
in-wasm via waki, decode PositionV2, discover positions) and dlmm_builder
(T1 — mechanical validation, then encode the unsigned tx with
AdvanceNonceAccount first and the live nonce hash as recentBlockhash). It
returns a full solana-action: URL — tap, sign in Phantom, done. No keys held
by the agent, ever.

The relay (Cloudflare Worker) is stateless: previews the tx from the bytes in
the URL path, echoes it back, holds zero secrets, makes zero RPC calls.
http_request is in excluded_tools — the LLM has no outbound at all; RPC runs
in-wasm under a host-injected anti-spoof __config. Durable nonce means a
Phantom approval that takes 5 minutes is fine — blockhash-expiry trap solved
at construction. Byte-for-byte ground truth vs the official @meteora-ag/dlmm
SDK: make fixtures-check, CI fails on drift.

Repo: https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot
Video: [youtube/loom URL or attached .mp4]
Runbook: showcase/demo-transcript.md (copy-pasteable Devnet commands)
Threat model + 7-scenario injection suite: prompts/injection-tests.md

Custody tier (honest):
  T0 — read/report/alert           no signing
  T1 — build claimFee / rebalance  user wallet signs (Action URL)
  T2 — auto-compound               disabled by construction, not by detection

Reproduce in an evening: README.md → Quick start. Operator-specific bits:
RPC URL, Telegram bot token, and your wallet pubkey — all from ${ENV_VAR}
placeholders in the plugin config sections. Worker deploy has no secrets.
```

---

## Checklist (jot before posting)

- [ ] Video uploaded; final URL in `[…]` placeholder above
- [ ] Repo URL confirmed reachable (https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot)
- [ ] `make validate` + `make fixtures-check` green on the version you're posting from
- [ ] Relay URL captured into `showcase/demo-transcript.md` if you ran live
- [ ] Fail-closed auto-compound refusal recorded for the video (Scenario 3 in
      `prompts/injection-tests.md`)
- [ ] One X post linked in thread for the build-in-public tiebreak
      (see `BUILD_LOG.md` for the milestones)

## Scoring-axis map

| Bounty axis | Where it lives |
|---|---|
| Use case (30%) | First paragraph + video |
| Safety & custody (25%) | Custody tier block + threat model link + injection suite link |
| Craft (20%) | Repo layout, `plugins/*/tests/host_integration.rs`, fixture suite, relay `parseTx` |
| Reproducibility (15%) | "Reproduce in an evening" line + README quickstart + `make fixtures-check` |
| Showcase (10%) | Video itself + this Discord post structure |
| Tiebreak — build-in-public | One X post linking this thread, timestamped during the bounty window |
