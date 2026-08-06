# DEPLOY.md — Self-host on a clean VPS (Rocky Linux) via OpenRouter

Walkthrough for running the copilot on a fresh Rocky Linux VPS, with
**OpenRouter** as the LLM provider (no direct Anthropic key needed).

## 0. Prereqs

| Need | Requirement |
|---|---|
| RAM | ≥ 4 GiB (8 GiB better) — source build peaks at ~4–7 GiB |
| Disk | ≥ 10 GiB free |
| Firewall | Telegram uses outbound long-polling → **no inbound ports required** (unless you expose the web dashboard) |
| Accounts | OpenRouter key, Telegram bot token, Solana RPC URL (Helius/any), Cloudflare account for the relay |

Run everything as a non-root user (e.g. `zeroclaw`). SELinux is enforcing on
Rocky by default — see §8 if the service misbehaves.

## 1. Base OS

```bash
sudo dnf update -y
sudo dnf install -y curl tar git gcc make
```

## 2. ZeroClaw — build from source (required)

> **Gotcha:** the prebuilt binary from the installer is built *without* the
> plugin host (`plugins-wasm` feature). `zeroclaw plugin …` would be an
> unrecognized subcommand and WASM plugins would never load. Build from source
> with a plugin execution backend instead.

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y
source ~/.cargo/env
rustup target add wasm32-wasip2        # needed for the copilot plugins later

git clone https://github.com/zeroclaw-labs/zeroclaw ~/.zeroclaw/src
cd ~/.zeroclaw/src
cargo build --release --features plugins-wasm-cranelift
cp target/release/zeroclaw ~/.cargo/bin/
zeroclaw --version
```

## 3. Copilot repo — plugins

```bash
git clone https://github.com/lksxyz/zeroclaw-dlmm-lp-copilot ~/copilot
cd ~/copilot
make plugin-build                     # dlmm-reader + dlmm-builder → wasm32-wasip2

zeroclaw plugin install plugins/dlmm-reader/
zeroclaw plugin install plugins/dlmm-builder/
zeroclaw plugin list                  # both must appear; else check §10
```

Each plugin lands in `~/.zeroclaw/plugins/<name>/` (`manifest.toml` + `.wasm`).

## 4. Credentials

| Secret | From | Where |
|---|---|---|
| `OPENROUTER_API_KEY` | openrouter.ai → Keys (`sk-or-…`) | env var (see §5) |
| `TELEGRAM_BOT_TOKEN` | @BotFather | env var |
| `TELEGRAM_CHAT_ID` | @userinfobot | config.toml |

OpenRouter notes:

- Pick a model ID from <https://openrouter.ai/models> — the config below uses
  `anthropic/claude-sonnet-4-5` (a `:free` suffix variant also exists).
- OpenRouter is prepaid: top up a few USD before the first invoke.
- The runtime profile already caps spend (`max_cost_per_day_cents = 500`).

## 5. Config

```bash
zeroclaw quickstart
cp ~/copilot/config.example.toml ~/.zeroclaw/config.toml
```

The example uses `${ENV_VAR}` placeholders read from the environment
(`env_overrides`). Either export them in the shell before starting the service,
or inline the real values into `~/.zeroclaw/config.toml`:

| Placeholder | Example |
|---|---|
| `OPENROUTER_API_KEY` | `sk-or-v1-…` |
| `TELEGRAM_BOT_TOKEN` | `123456:ABC-…` |
| `TELEGRAM_CHAT_ID` | `-1001234567890` |
| `SOLANA_RPC_URL` | `https://mainnet.helius-rpc.com/?api-key=…` |
| `OPERATOR_WALLET_PUBKEY` | your Phantom pubkey |
| `ACTION_ENDPOINT_BASE` | `https://dlmm-relay.<subdomain>.workers.dev` |

> The `[plugins.entries.*]` sections must be hand-added — `zeroclaw config set`
> can't materialize a `plugins.entries` node for a freshly installed plugin.
> The example ships them pre-filled, so this is normally a no-op.

## 6. Skills + SOPs

```bash
mkdir -p ~/.zeroclaw/skills ~/.zeroclaw/sops
cp ~/copilot/skills/*.md ~/.zeroclaw/skills/
cp -r ~/copilot/sops/dlmm-* ~/.zeroclaw/sops/
```

Cron runs inside ZeroClaw (`[sops.triggers.*]`) — no system cron needed.

## 7. Relay (action-endpoint)

Stateless Cloudflare Worker, no secrets. Deploy from your laptop (not the VPS):

```bash
cd action-endpoint && npm install
CLOUDFLARE_API_TOKEN=<token> npx wrangler deploy
```

Paste the resulting `*.workers.dev` URL into `ACTION_ENDPOINT_BASE`.

## 8. Service

```bash
zeroclaw service install && zeroclaw service start
zeroclaw service status
```

Rocky-specific:

- **SELinux:** if the unit fails to start, check `ausearch -m avc -ts recent`
  for denials; `sudo setsebool -P httpd_can_network_connect 1` covers outbound
  connects if the policy blocks them.
- **systemd user unit:** enable linger so the service runs without a login —
  `sudo loginctl enable-linger $USER`.
- If the service runs under systemd, env vars from §5 must be visible to the
  unit (inline them in `config.toml`, or add an `EnvironmentFile=`).

## 9. Verify

```bash
journalctl -u zeroclaw -f        # or the user-unit log path
zeroclaw plugin list             # reader + builder present
```

Then DM the bot: `report` → position summary; `claim #<id>` → Solana Action URL.

## 10. Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `zeroclaw plugin: unrecognized subcommand` | Prebuilt binary — rebuild with `--features plugins-wasm-cranelift` (§2) |
| `plugin list` empty | Check startup log for skip warning: malformed manifest, missing `wasm_path`, or signature-policy rejection |
| Agent can't reach RPC/relay | `http_request` is excluded by design — traffic must flow through plugins; check `plugins.entries` config values |
| Service fails to start | SELinux denials (`ausearch -m avc`), missing env vars, or linger not enabled |
| No Telegram replies | Bot token/chat id wrong, or `sender_match = "handle"` mismatch |
