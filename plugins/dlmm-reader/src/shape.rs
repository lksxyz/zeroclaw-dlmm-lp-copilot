//! Pure shaping for the dlmm-reader tool — host-testable, no RPC.
//!
//! The wasm entry (lib.rs) does the RPC fetching and hands decoded values to
//! the functions here; `cargo test` exercises them with fixture bytes.

use dlmm_core::decoder::{
    claimable_fees_with_bins, total_liquidity_shares, BinArray, LbPair, PositionV2,
};
use serde::Deserialize;
use serde_json::json;

/// Host-injected config (ZeroClaw injects the plugin's config section under
/// the reserved `__config` key; caller-supplied values are stripped by the
/// host, so these cannot be spoofed by the LLM).
#[derive(Deserialize, Debug, Default, Clone)]
pub struct PluginConfig {
    pub rpc_url: String,
    pub owner_pubkey: Option<String>,
    #[serde(default)]
    pub dlmm_program: Option<String>,
}

/// Tool arguments (mode `report` or `status`).
#[derive(Deserialize, Debug, Default)]
pub struct Args {
    pub mode: String,
    #[serde(default)]
    pub position_ids: Vec<String>,
    #[serde(default)]
    pub nonce_address: Option<String>,
    #[serde(default)]
    pub previous_nonce_hash: Option<String>,
    #[serde(rename = "__config")]
    pub config: Option<PluginConfig>,
}

/// A compact per-position report (~200 tokens total for a few positions).
pub fn shape_report(
    position_id: &str,
    position: &PositionV2,
    lb_pair: &LbPair,
    owner: Option<&str>,
    price_x: Option<f64>,
    price_y: Option<f64>,
    dec_x: u8,
    dec_y: u8,
    bin_arrays: &[BinArray],
) -> serde_json::Value {
    let (fee_x, fee_y) = claimable_fees_with_bins(position, bin_arrays);
    let in_range = position.lower_bin_id <= lb_pair.active_id
        && lb_pair.active_id <= position.upper_bin_id;

    // Raw → human units, then USD via Jupiter prices (if available).
    // Clamp to finite f64: `json!` serializes via `to_value(...).unwrap()`,
    // which panics on NaN/Infinity (the wasm panic we were chasing).
    let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
    let human = |raw: u64, dec: u8, price: Option<f64>| -> (f64, Option<f64>) {
        let h = finite(raw as f64 / 10f64.powi(dec as i32));
        (h, price.map(|p| finite(h * p)))
    };
    let (fee_x_h, fee_x_usd) = human(fee_x, dec_x, price_x);
    let (fee_y_h, fee_y_usd) = human(fee_y, dec_y, price_y);
    let usd_approx = match (fee_x_usd, fee_y_usd) {
        (Some(a), Some(b)) => Some(finite(a + b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    let owner_match = match owner {
        Some(o) => o == position.owner.to_string(),
        None => true, // no owner configured → skip the check (report-only)
    };

    json!({
        "id": position_id,
        "lb_pair": position.lb_pair.to_string(),
        "range": [position.lower_bin_id, position.upper_bin_id],
        "active_bin_id": lb_pair.active_id,
        "in_range": in_range,
        "owner_match": owner_match,
        "claimable": { "x": fee_x_h, "y": fee_y_h, "usd_approx": usd_approx },
        // Raw share can exceed u64::MAX (serde_json "number out of range" on
        // the json! macro's to_value). Emit as a string to stay JSON-safe.
        "liquidity_shares": total_liquidity_shares(position).to_string(),
    })
}

/// Durable-nonce settlement status: did the nonce's stored blockhash change
/// since the last recorded value? If it did, the pending tx landed (or was
/// replaced) and a new proposal is safe.
pub fn shape_status(
    nonce_address: &str,
    current_hash: &str,
    previous_hash: Option<&str>,
) -> serde_json::Value {
    let settled = previous_hash.map(|p| p != current_hash).unwrap_or(false);
    json!({
        "nonce_address": nonce_address,
        "current_hash": current_hash,
        "previous_hash": previous_hash,
        "settled": settled,
    })
}

/// Ownership check used by status mode — fail-closed: without a configured
/// owner the check is skipped in the report (the builder enforces it hard).
pub fn owner_is(position: &PositionV2, owner: Option<&str>) -> bool {
    match owner {
        Some(o) => o == position.owner.to_string(),
        None => false,
    }
}

/// One discovered position — the compact shape `mode=positions` returns
/// (id + pool + range only; the report mode adds the full detail).
pub fn shape_discovery(position_id: &str, position: &PositionV2) -> serde_json::Value {
    json!({
        "id": position_id,
        "lb_pair": position.lb_pair.to_string(),
        "range": [position.lower_bin_id, position.upper_bin_id],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use dlmm_core::decoder::{decode_bin_array, decode_lb_pair, decode_position};

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../dlmm-core/tests/fixtures.json")).unwrap()
    }

    fn b64(v: &serde_json::Value) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(v.as_str().unwrap())
            .unwrap()
    }

    #[test]
    fn report_shapes_claimable_and_range() {
        let f = fixture();
        let pos = decode_position(&b64(&f["position_v2"])).unwrap();
        let lb = decode_lb_pair(&b64(&f["lb_pair"])).unwrap();
        // Array 120 covers position bins 8450..8469; the pending fees live
        // in fee_infos[0] (bin 8450), and the aligned bin has no per-token
        // stored fee, so claimable = pending sum (same as before the fix).
        let bin_arr = decode_bin_array(&b64(&f["bin_array_aligned"])).unwrap();

        let r = shape_report(
            "7fTxDfcWTMVJg2r26Jv496HsuEBi6Hc77QEsHE9NSVZ1",
            &pos,
            &lb,
            Some("2KDS5vtFQJyYJApNGoPVaBSaYP7Vp4R3txRM9SYj13BW"),
            Some(150.0),
            Some(1.0),
            9,
            6,
            &[bin_arr],
        );
        assert_eq!(r["id"], "7fTxDfcWTMVJg2r26Jv496HsuEBi6Hc77QEsHE9NSVZ1");
        assert_eq!(r["lb_pair"], f["keys"]["pool"]);
        assert_eq!(r["range"], json!([8450, 8520]));
        assert_eq!(r["active_bin_id"], 8500);
        assert_eq!(r["in_range"], true);
        assert_eq!(r["owner_match"], true);
        // 1_000_000 raw x / 1e9 * $150 = $0.15; 2_000_000 / 1e6 * $1 = $2
        let usd = r["claimable"]["usd_approx"].as_f64().unwrap();
        assert!((usd - 2.15).abs() < 1e-9, "usd_approx = {usd}");
        assert_eq!(r["liquidity_shares"], json!("1500000000"));
    }

    #[test]
    fn report_flags_owner_mismatch() {
        let f = fixture();
        let pos = decode_position(&b64(&f["position_v2"])).unwrap();
        let lb = decode_lb_pair(&b64(&f["lb_pair"])).unwrap();
        let bin_arr = decode_bin_array(&b64(&f["bin_array_aligned"])).unwrap();
        let r = shape_report(
            "pos",
            &pos,
            &lb,
            Some("11111111111111111111111111111111"),
            None,
            None,
            9,
            6,
            &[bin_arr],
        );
        assert_eq!(r["owner_match"], false);
        assert!(r["claimable"]["usd_approx"].is_null());
    }

    #[test]
    fn status_settlement_detects_hash_change() {
        let s = shape_status("nonce123", "hashA", Some("hashA"));
        assert_eq!(s["settled"], false);
        let s2 = shape_status("nonce123", "hashB", Some("hashA"));
        assert_eq!(s2["settled"], true);
        // no previous → cannot determine
        let s3 = shape_status("nonce123", "hashA", None);
        assert_eq!(s3["settled"], false);
    }

    #[test]
    fn discovery_shape_is_compact() {
        let f = fixture();
        let pos = decode_position(&b64(&f["position_v2"])).unwrap();
        let d = shape_discovery("pos123", &pos);
        assert_eq!(d["id"], "pos123");
        assert_eq!(d["lb_pair"], f["keys"]["pool"]);
        assert_eq!(d["range"], json!([8450, 8520]));
        // no claimable/price fields — the report mode adds those
        assert!(d.get("claimable").is_none());
    }

    #[test]
    fn owner_is_fails_closed_without_config() {
        let f = fixture();
        let pos = decode_position(&b64(&f["position_v2"])).unwrap();
        assert!(!owner_is(&pos, None));
        assert!(owner_is(&pos, Some("2KDS5vtFQJyYJApNGoPVaBSaYP7Vp4R3txRM9SYj13BW")));
    }
}
