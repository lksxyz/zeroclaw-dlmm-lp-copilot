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
[ ] `make plugin-build` succeeds (cargo build --target wasm32-wasip2 --release)
[ ] `make fixtures-check` green (no byte drift on fixtures.json)
[ ] No secrets in git history (rotate any key that ever touched a commit)
[ ] LICENSE present (MIT)
[ ] config.example.toml uses ${ENV_VAR} placeholders, never inline secrets
[ ] config.toml never committed (still in .gitignore)

Plugins
-------
[ ] dlmm_reader.wasm + dlmm_builder.wasm built and checked in
[ ] __config.nonce_addresses is a pool (≥ 2 slots) — concurrent pending approvals
[ ] Plugin manifest permissions = ["http_client", "config_read"] only
[ ] `make plugin` runs all host tests (dlmm-core + dlmm-reader + dlmm-builder)

Mainnet smoke
-------------
[ ] Wallet funded on mainnet (small amount, e.g. $20)
[ ] At least one DLMM position open
[ ] Daily 08:00 SOP has run on mainnet (Telegram DM received)
[ ] Range-monitor SOP has run on mainnet (every 30 min)
[ ] claim + rebalance round-trip on mainnet: unsigned tx → sign in wallet → on-chain confirmed
[ ] Fail-closed auto-compound refusal recorded (compare to prompts/injection-tests.md Scenario 3)

Showcase artifacts
------------------
[ ] Video: ≤ 3 min, terminal + phone, no slides, captions
[ ] Video uploaded (YouTube unlisted / Loom / direct file)
[ ] Write-up covers: what / who / which ZeroClaw features / what we built /
      custody tier / threat model / link to repo + skills + SOPs
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
