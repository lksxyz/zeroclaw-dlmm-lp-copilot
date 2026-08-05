#!/usr/bin/env bash
# Pre-submit checklist. Prints a copy-pasteable list to fill in before posting
# the showcase to #solana-bounty in the ZeroClaw Discord (https://discord.gg/zeroclaw).

cat <<'EOF'
DLMM LP Copilot — pre-submit checklist
======================================

Repo
----
[ ] Repo public at https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot
[ ] `make validate` passes locally
[ ] No secrets in git history (rotate any key that ever touched a commit)
[ ] LICENSE present (MIT)
[ ] config.example.toml uses ${ENV_VAR} placeholders, never inline secrets

Worker
------
[ ] `make worker-deploy` succeeded
[ ] `curl ${ACTION_ENDPOINT_BASE}/health` returns 200
[ ] Both /actions/claim and /actions/rebalance return valid Solana Action JSON
[ ] Jupiter Price API reachable (price feed) — curl https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112
[ ] wrangler tail shows no 5xx in the last 24h

Devnet demo
-----------
[ ] Wallet funded with at least 2 SOL on Devnet
[ ] Two DLMM positions open (one stable, one volatile)
[ ] Position pubkeys captured in showcase/demo-transcript.md
[ ] Worker URL captured and stable
[ ] Fail-closed auto-compound refusal recorded (compare to prompts/injection-tests.md Scenario 3)

Showcase artifacts
------------------
[ ] Video: ≤ 3 min, terminal + phone, no slides, captions
[ ] Video uploaded (YouTube unlisted / Loom / direct file)
[ ] Write-up covers: what / who / which ZeroClaw features / what we built /
      custody tier / threat model / link to repo + skills + SOPs + worker
[ ] Prompt-injection transcript in the post OR linked from it
[ ] Custody tier table included verbatim from SUBMISSION.md

Discord post (#solana-bounty in ZeroClaw — https://discord.gg/zeroclaw)
----------------------------------------------------------------------
[ ] Title: "DLMM LP Copilot — T0/T1 DLMM guardian on Telegram"
[ ] Body: link to video + link to repo + 5-line summary + custody tier line
[ ] Reply to any maintainer questions within 24h
[ ] Build-in-public log on X (Tiebreak points — bounty explicit — see BUILD_LOG.md)

Things we did NOT do (and won't)
---------------------------------
[ ] No PR opened against zeroclaw-labs/zeroclaw-plugins
[ ] No claim of a T2 path that isn't actually disabled
[ ] No inline real keys anywhere
EOF
