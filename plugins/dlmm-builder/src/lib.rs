//! ZeroClaw WIT tool plugin: build unsigned Meteora DLMM claim/rebalance
//! transactions **in-wasm** (waki) and return the raw unsigned tx (base64).
//!
//! ## Structure
//!
//! - `build` — pure unsigned-tx building (host-testable): args parsing,
//!   mechanical validation, tx encoding.
//! - wasm entry (behind `#[cfg(target_family = "wasm")]`) — `execute` parses
//!   args, fetches accounts + the durable-nonce hash via `dlmm_core::rpc`
//!   (waki), decodes with `dlmm_core::decoder`, and calls `build_action`.
//!
//! ## Security model
//!
//! The RPC endpoint, owner pubkey, and DLMM program id all come from the
//! plugin config section, which the host injects as `__config` (caller-supplied
//! values are stripped). The LLM can only pick *which* position to act on and
//! the *target range* — it cannot redirect the RPC, change the signer, or
//! tamper with the destination program. Mechanical validation ("LLM proposes,
//! plugin verifies") rejects anything that would touch a position the
//! operator doesn't own.

pub mod build;

#[cfg(target_family = "wasm")]
mod component {
    use super::build::{build_action, parse_key, resolve_nonce, ActionInput, Args};
    use dlmm_core::decoder::{decode_bin_array, decode_lb_pair, decode_position};
    use dlmm_core::pda;
    use dlmm_core::rpc::RpcClient;
    use dlmm_core::tx::bin_array_indexes_for_range;
    use solana_pubkey::Pubkey;

    wit_bindgen::generate!({
        world: "tool-plugin",
        path: "wit",
        features: ["plugins-wit-v0"],
    });

    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};

    struct DlmmBuilder;

    impl PluginInfo for DlmmBuilder {
        fn plugin_name() -> String {
            "dlmm-builder".to_string()
        }

        fn plugin_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }
    }

    impl Tool for DlmmBuilder {
        fn name() -> String {
            "dlmm_builder".to_string()
        }

        fn description() -> String {
            "Build unsigned Meteora DLMM transactions. mode=claim: collect fees. \
             mode=rebalance: move a position to new_low/new_high. Validates mechanically \
             (ownership, claimable, range contains the active bin) and returns the \
             raw unsigned tx as base64."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["claim", "rebalance"],
                        "description": "claim: collect fees; rebalance: move range."
                    },
                    "position_id": {
                        "type": "string",
                        "description": "DLMM PositionV2 account address to act on."
                    },
                    "nonce_address": {
                        "type": "string",
                        "description": "Durable nonce account (recentBlockhash source). Optional: when __config.nonce_address is set, the host-injected value is used instead and this arg is ignored. When both are empty, falls back to the latest blockhash (demo path; expires ~90 s)."
                    },
                    "new_low": {
                        "type": "integer",
                        "description": "New lower bin id (mode=rebalance only)."
                    },
                    "new_high": {
                        "type": "integer",
                        "description": "New upper bin id (mode=rebalance only)."
                    },
                    "label": {
                        "type": "string",
                        "description": "Optional label shown in the wallet."
                    }
                },
                "required": ["mode", "position_id"]
            })
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            let args: Args = serde_json::from_str(&args).map_err(|e| format!("bad args: {e}"))?;
            let config = args
                .config
                .as_ref()
                .ok_or("missing __config (config_read permission?)")?;
            if config.rpc_url.is_empty() {
                return Err("__config.rpc_url is empty — configure the plugin section".to_string());
            }
            let owner = config
                .owner_pubkey
                .as_deref()
                .ok_or("__config.owner_pubkey missing — builder refuses to run without it")?;
            let owner = parse_key(owner, "owner_pubkey")?;
            let program = match config.dlmm_program.as_deref() {
                Some(p) => parse_key(p, "dlmm_program")?,
                None => parse_key(super::build::DEFAULT_DLMM_PROGRAM, "default dlmm_program")?,
            };

            let rpc = RpcClient::new(config.rpc_url.clone());

            let position_id = args.position_id.as_deref().ok_or("position_id required")?;
            let position_key = parse_key(position_id, "position_id")?;

            // Fetch + decode position and pool.
            let position = decode_position(
                &rpc.get_account_info(&position_key)?
                    .ok_or_else(|| format!("position not found: {position_id}"))?,
            )
            .map_err(|e| format!("position {position_id} decode: {e}"))?;
            let lb_pair = decode_lb_pair(
                &rpc.get_account_info(&position.lb_pair)?
                    .ok_or_else(|| format!("pool not found: {}", position.lb_pair))?,
            )
            .map_err(|e| format!("pool {} decode: {e}", position.lb_pair))?;

            // Two paths for recentBlockhash:
            //   __config.nonce_address set  → durable-nonce hash (production;
            //   survives approval delays; one pending tx per nonce account).
            //   __config.nonce_address unset → latest blockhash (demo path;
            //   expires ~90 s, user must sign promptly).
            //
            // Precedence: the host-injected `__config.nonce_address` wins
            // over any `args.nonce_address` the LLM passes. The runtime
            // strips caller-supplied `__config` (anti-spoof), so the LLM
            // can never substitute the operator's nonce for an
            // attacker-controlled pubkey — see `prompts/injection-tests.md`
            // scenario 6 for the threat model.
            let nonce_addr = resolve_nonce(
                config.nonce_address.as_deref(),
                args.nonce_address.as_deref(),
            );
            let (nonce_hash, use_nonce, nonce) = match nonce_addr.as_deref() {
                Some(addr) => {
                    let key = parse_key(addr, "nonce_address")?;
                    let hash = rpc.get_nonce_hash_bytes(&key)?;
                    (hash, true, key)
                }
                None => (rpc.get_latest_blockhash_bytes()?, false, Pubkey::default()),
            };

            // Rebalance: fetch the bin arrays covering the CURRENT range to
            // derive the amounts to re-deposit. Missing arrays are skipped
            // (empty bins); decode errors abort.
            // Fetch the bin arrays covering the position's CURRENT range.
            // Rebalance derives the amounts to re-deposit from them; claim
            // needs each bin's per-token stored fees for the claimable
            // validation. Missing arrays are skipped (empty bins); decode
            // errors abort.
            let mut bin_arrays = Vec::new();
            for idx in bin_array_indexes_for_range(position.lower_bin_id, position.upper_bin_id) {
                let key = pda::bin_array(&position.lb_pair, idx, &program).0;
                if let Some(bytes) = rpc.get_account_info(&key)? {
                    let ba = decode_bin_array(&bytes)
                        .map_err(|e| format!("bin array {idx} decode: {e}"))?;
                    bin_arrays.push(ba);
                }
            }

            let out = build_action(&ActionInput {
                mode: &args.mode,
                position_id,
                position_key: &position_key,
                position: &position,
                lb_pair: &lb_pair,
                bin_arrays: &bin_arrays,
                nonce_hash,
                owner: &owner,
                nonce: &nonce,
                program: &program,
                new_low: args.new_low,
                new_high: args.new_high,
                label: args.label.clone(),
                use_nonce,
            })?;

            Ok(ToolResult {
                success: true,
                output: serde_json::to_string(&out).map_err(|e| format!("serialize: {e}"))?,
                error: None,
            })
        }
    }

    export!(DlmmBuilder);
}
