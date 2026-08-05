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
  • 08:00 [TZ] daily report — value, fees, IL vs HODL, action suggestion
  • every 30 min — out-of-range + IL + fee-milestone alerts
  • DM `claim #<id>` → unsigned claimFee tx → sign in Phantom
  • DM `rebalance #<id> [wide|tight]` → atomic 3-ix tx on a durable nonce
    → sign in Phantom

No keys held by the agent. Rebalance uses a durable nonce so a Phantom
approval that takes 5 minutes is fine — blockhash-expiry trap solved at
construction. Pyth deprecation sidestepped by Jupiter Price API. Filesystem
+ web tools denied outright (excluded_tools), not just approval-gated.

Repo: https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot
Video: [youtube/loom URL or attached .mp4]
Runbook: showcase/demo-transcript.md (copy-pasteable Devnet commands)
Threat model + 7-scenario injection suite: prompts/injection-tests.md

Custody tier (honest):
  T0 — read/report/alert           no signing
  T1 — build claimFee / rebalance  user wallet signs (Action URL)
  T2 — auto-compound               disabled by construction, not by detection

Reproduce in an evening: README.md → Quick start (~25 commands). The only
operator-specific bits are RPC URL (Helius/Triton/QuickNode), Telegram bot
token, and your wallet pubkey — all from ${ENV_VAR} placeholders.

Self-hosted Action endpoint (Cloudflare Worker, ~300 lines, no Dialect
dependency). WASM plugin (Tier 3 bonus) shapes the per-position summary to
~200 tokens so a 20-position portfolio stays inside the model context.
```

---

## Checklist (jot before posting)

- [ ] Video uploaded; final URL in `[…]` placeholder above
- [ ] Repo URL confirmed reachable (https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot)
- [ ] `make validate` green on the version you're posting from
- [ ] Worker URL captured into `showcase/demo-transcript.md` if you ran live
- [ ] Fail-closed auto-compound refusal recorded for the video (Scenario 3 in
      `prompts/injection-tests.md`)
- [ ] One X post linked in thread for the build-in-public tiebreak
      (see `BUILD_LOG.md` for the milestones)

## Scoring-axis map

| Bounty axis | Where it lives |
|---|---|
| Use case (30%) | First paragraph + video |
| Safety & custody (25%) | Custody tier block + threat model link + injection suite link |
| Craft (20%) | Repo layout, plugin `tests/host_integration.rs`, action endpoint types |
| Reproducibility (15%) | "Reproduce in an evening" line + README quickstart |
| Showcase (10%) | Video itself + this Discord post structure |
| Tiebreak — build-in-public | One X post linking this thread, timestamped during the bounty window |