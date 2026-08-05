//! ZeroClaw plugin: read Meteora DLMM positions and return a shaped,
//! ~200-token-per-position summary.
//!
//! ## Structure
//!
//! - `decoder` — pure core, always compiled. Host-testable.
//! - wasm entry point (behind `#[cfg(target_family = "wasm")]`) — `Guest::execute`
//!   deserialises args, calls `decoder::decode_position` + `decoder::shape`, and
//!   returns a JSON array of `PositionSummary`.
//!
//! The heavy lifting (RPC fetch, pool state, Switchboard read) is done by the
//! *host* (the agent, via skill markdown + built-in `http_request`). The plugin
//! is the *shaping* step that fits the model context.

pub mod decoder;

#[cfg(target_family = "wasm")]
use crate::exports::zeroclaw::tool::plugin::Guest;
#[cfg(target_family = "wasm")]
use serde::Deserialize;
#[cfg(target_family = "wasm")]
use wit_bindgen::generate;

#[cfg(target_family = "wasm")]
export!(Component);

#[cfg(target_family = "wasm")]
generate!({
    world: "tool-plugin",
    path: "wit",
});

#[cfg(target_family = "wasm")]
#[derive(Deserialize)]
struct Args {
    positions_b64: Vec<String>,
    sol_usd: f64,
    active_bin_by_position: Vec<(String, i32)>,
    entry_value_usd_by_position: Vec<(String, f64)>,
    hodl_value_usd_by_position: Vec<(String, f64)>,
    fees_24h_by_position: Vec<(String, f64)>,
    fees_7d_by_position: Vec<(String, f64)>,
    pair_label_by_position: Vec<(String, String)>,
    id_label_by_position: Vec<(String, String)>,
}

#[cfg(target_family = "wasm")]
struct Component;

#[cfg(target_family = "wasm")]
impl Guest for Component {
    fn execute(args: String) -> Result<String, String> {
        let args: Args = serde_json::from_str(&args).map_err(|e| format!("bad args: {e}"))?;

        let mut summaries: Vec<decoder::PositionSummary> = Vec::with_capacity(args.positions_b64.len());

        for (i, b64) in args.positions_b64.iter().enumerate() {
            use base64::Engine as _;
            let data = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| format!("bad base64 at position {i}: {e}"))?;

            let position = decoder::decode_position(&data)
                .map_err(|e| format!("bad position at {i}: {e}"))?;

            let active_bin_id = args.active_bin_by_position.get(i).map(|(_, b)| *b).unwrap_or(0);
            let entry_value = args.entry_value_usd_by_position.get(i).map(|(_, v)| *v).unwrap_or(0.0);
            let hodl_value = args.hodl_value_usd_by_position.get(i).map(|(_, v)| *v).unwrap_or(0.0);
            let fees_24h = args.fees_24h_by_position.get(i).map(|(_, v)| *v).unwrap_or(0.0);
            let fees_7d = args.fees_7d_by_position.get(i).map(|(_, v)| *v).unwrap_or(0.0);
            let pair_label = args.pair_label_by_position.get(i).map(|(_, l)| l.as_str()).unwrap_or("?");
            let id_label = args.id_label_by_position.get(i).map(|(_, l)| l.as_str()).unwrap_or("?");

            let summary = decoder::shape(
                &position,
                args.sol_usd,
                active_bin_id,
                entry_value,
                hodl_value,
                fees_24h,
                fees_7d,
                pair_label,
                id_label,
            );

            summaries.push(summary);
        }

        serde_json::to_string(&summaries).map_err(|e| format!("serialize: {e}"))
    }
}