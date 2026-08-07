//! ZeroClaw WIT tool plugin: fetch Meteora DLMM positions + durable-nonce
//! settlement status **in-wasm** (waki), decode, and emit a shaped report.
//!
//! ## Structure
//!
//! - `shape` — pure shaping (host-testable): args parsing, report/status
//!   shaping from decoded values.
//! - wasm entry (behind `#[cfg(target_family = "wasm")]`) — `execute` parses
//!   args, fetches accounts via `dlmm_core::rpc` (waki), decodes with
//!   `dlmm_core::decoder`, and shapes the result.
//!
//! ## Security model
//!
//! The RPC endpoint and owner pubkey come from the plugin config section,
//! which the host injects as `__config` (caller-supplied values are
//! stripped). The LLM can only pick which positions to read — it cannot
//! redirect the RPC endpoint.
//!
//! Modes:
//!   * `positions` — discovery: all PositionV2 accounts owned by
//!     `__config.owner_pubkey` (id, pool, range). Replaces the old
//!     `getProgramAccounts` http_request flow now that http_request is denied.
//!   * `report` — per-position: range, active bin, in/out of range,
//!     claimable fees (raw + human + USD via Jupiter), owner match.
//!   * `status` — durable-nonce settlement: did the stored blockhash change
//!     since `previous_nonce_hash`? (SOP cleanup evidence.)

pub mod shape;

#[cfg(target_family = "wasm")]
mod component {
    use super::shape::{shape_discovery, shape_report, shape_status, Args};
    use dlmm_core::decoder::{decode_bin_array, decode_lb_pair, decode_position};
    use dlmm_core::pda;
    use dlmm_core::rpc::RpcClient;
    use dlmm_core::tx::bin_array_indexes_for_range;
    use serde_json::json;

    /// Default DLMM program for discovery: devnet. Operators pin their own via
    /// `__config.dlmm_program`.
    const DEFAULT_DLMM_PROGRAM: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

    wit_bindgen::generate!({
        world: "tool-plugin",
        path: "wit",
        features: ["plugins-wit-v0"],
    });

    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};

    struct DlmmReader;

    impl PluginInfo for DlmmReader {
        fn plugin_name() -> String {
            "dlmm-reader".to_string()
        }

        fn plugin_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }
    }

    impl Tool for DlmmReader {
        fn name() -> String {
            "dlmm_reader".to_string()
        }

        fn description() -> String {
            "Meteora DLMM position data for the configured wallet. \
             mode=positions: discover owned positions (id, pool, configured range) — \
             quick list. mode=report: FULL per-position report — fetches the pool, \
             token decimals, and the bin arrays covering the range, then returns \
             active_bin_id, in_range, owner_match, claimable fees (x, y, usd_approx) \
             and liquidity_shares. Use mode=report when the operator asks for a \
             report, live fees, claimable amount, active bin, or 'in range' status. \
             mode=status: durable-nonce settlement check."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["positions", "report", "status"],
                        "description": "positions: discover owned positions; report: position summaries; status: nonce settlement check."
                    },
                    "position_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "DLMM PositionV2 account addresses (mode=report)."
                    },
                    "nonce_address": {
                        "type": "string",
                        "description": "Durable nonce account address (mode=status)."
                    },
                    "previous_nonce_hash": {
                        "type": "string",
                        "description": "Last recorded nonce hash (mode=status; settlement evidence)."
                    }
                },
                "required": ["mode"]
            })
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            let args: Args = serde_json::from_str(&args).map_err(|e| format!("bad args: {e}"))?;
            let config = args.config.as_ref().ok_or("missing __config (config_read permission?)")?;
            if config.rpc_url.is_empty() {
                return Err("__config.rpc_url is empty — configure the plugin section".to_string());
            }
            let rpc = RpcClient::new(config.rpc_url.clone());
            let owner = config.owner_pubkey.as_deref();

            // Wrap in catch_unwind so a Rust panic (which otherwise surfaces
            // as an opaque wasm trap "tool.execute trapped: ...") returns the
            // actual panic message. This is a diagnostic net while debugging
            // the report WASM crash; once the panic is gone we can drop it.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let out = match args.mode.as_str() {
                    "positions" => positions_mode(&rpc, &args, owner)?,
                    "report" => report_mode(&rpc, &args, owner)?,
                    "status" => status_mode(&rpc, &args)?,
                    other => {
                        return Err(format!(
                            "unknown mode: {other} (expected positions|report|status)"
                        ))
                    }
                };
                Ok::<String, String>(out)
            }));

            let out = match result {
                Ok(Ok(out)) => out,
                Ok(Err(e)) => return Err(e),
                Err(panic) => {
                    let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else if let Some(s) = panic.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    return Err(format!("internal plugin panic: {msg}"));
                }
            };

            Ok(ToolResult {
                success: true,
                output: out,
                error: None,
            })
        }
    }

    /// mode=positions: discover all PositionV2 accounts owned by the
    /// configured operator wallet (getProgramAccounts with an owner memcmp).
    fn positions_mode(
        rpc: &RpcClient,
        args: &Args,
        owner: Option<&str>,
    ) -> Result<String, String> {
        let owner = owner
            .ok_or("mode=positions requires __config.owner_pubkey (position discovery is by owner)")?;
        let owner_key: solana_pubkey::Pubkey = owner
            .parse()
            .map_err(|_| format!("bad owner_pubkey: {owner}"))?;
        let program = match args.config.as_ref().and_then(|c| c.dlmm_program.as_deref()) {
            Some(p) => p
                .parse()
                .map_err(|_| format!("bad dlmm_program: {p}"))?,
            None => DEFAULT_DLMM_PROGRAM
                .parse()
                .map_err(|e| format!("bad default dlmm_program: {e}"))?,
        };

        // PositionV2: 8120 bytes, owner pubkey at offset 40 (8 discriminator +
        // 32 lb_pair).
        let accounts = rpc
            .get_program_accounts(&program, 8120, 40, owner_key.as_ref())
            .map_err(|e| format!("position discovery: {e}"))?;

        let mut positions = Vec::with_capacity(accounts.len());
        for (id, data) in accounts {
            let pos = decode_position(&data)
                .map_err(|e| format!("position {id} decode: {e}"))?;
            positions.push(shape_discovery(&id.to_string(), &pos));
        }
        Ok(serde_json::to_string(&json!({ "positions": positions }))
            .map_err(|e| format!("serialize: {e}"))?)
    }

    /// mode=report: fetch + decode each position, shape a compact summary.
    fn report_mode(rpc: &RpcClient, args: &Args, owner: Option<&str>) -> Result<String, String> {
        if args.position_ids.is_empty() {
            return Err("mode=report requires position_ids".to_string());
        }
        let program = match args.config.as_ref().and_then(|c| c.dlmm_program.as_deref()) {
            Some(p) => p
                .parse()
                .map_err(|_| format!("bad dlmm_program: {p}"))?,
            None => DEFAULT_DLMM_PROGRAM
                .parse()
                .map_err(|e| format!("bad default dlmm_program: {e}"))?,
        };
        let mut reports = Vec::with_capacity(args.position_ids.len());
        for id in &args.position_ids {
            let pos_key: solana_pubkey::Pubkey = id
                .parse()
                .map_err(|_| format!("bad position id: {id}"))?;

            let pos = decode_position(
                &rpc.get_account_info(&pos_key)?
                    .ok_or_else(|| format!("position not found: {id}"))?,
            )
            .map_err(|e| format!("position {id} decode: {e}"))?;

            let lb = decode_lb_pair(
                &rpc.get_account_info(&pos.lb_pair)?
                    .ok_or_else(|| format!("pool not found: {}", pos.lb_pair))?,
            )
            .map_err(|e| format!("pool {} decode: {e}", pos.lb_pair))?;

            // Mint decimals from RPC.
            let dec_x = rpc.get_mint_decimals(&lb.token_x_mint).unwrap_or(9);
            let dec_y = rpc.get_mint_decimals(&lb.token_y_mint).unwrap_or(9);

            // Jupiter prices for the USD estimate. Best-effort: a price miss
            // (unknown mint, rate limit, host not permitted) leaves
            // usd_approx = null rather than failing the whole report.
            let price_x = dlmm_core::rpc::get_jupiter_price(&lb.token_x_mint).ok();
            let price_y = dlmm_core::rpc::get_jupiter_price(&lb.token_y_mint).ok();

            // Fetch the bin arrays covering the position's CURRENT range —
            // the claimable-fee math needs each bin's per-token stored fees.
            // Missing arrays are skipped (empty bins); decode errors abort.
            let mut bin_arrays = Vec::new();
            for idx in bin_array_indexes_for_range(pos.lower_bin_id, pos.upper_bin_id) {
                let key = pda::bin_array(&pos.lb_pair, idx, &program).0;
                if let Some(bytes) = rpc.get_account_info(&key)? {
                    let ba = decode_bin_array(&bytes)
                        .map_err(|e| format!("bin array {idx} decode: {e}"))?;
                    bin_arrays.push(ba);
                }
            }

            reports.push(shape_report(id, &pos, &lb, owner, price_x, price_y, dec_x, dec_y, &bin_arrays));
        }
        Ok(serde_json::to_string(&json!({ "positions": reports }))
            .map_err(|e| format!("serialize: {e}"))?)
    }

    /// mode=status: current nonce hash + settlement vs previous.
    fn status_mode(rpc: &RpcClient, args: &Args) -> Result<String, String> {
        let nonce = args
            .nonce_address
            .as_ref()
            .ok_or("mode=status requires nonce_address")?;
        let nonce_key: solana_pubkey::Pubkey = nonce
            .parse()
            .map_err(|_| format!("bad nonce address: {nonce}"))?;
        let current = rpc.get_nonce_hash(&nonce_key)?;
        let out = shape_status(nonce, &current, args.previous_nonce_hash.as_deref());
        Ok(serde_json::to_string(&out).map_err(|e| format!("serialize: {e}"))?)
    }

    export!(DlmmReader);
}
