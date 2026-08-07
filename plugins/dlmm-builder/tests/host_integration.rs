//! Host-run integration tests for the dlmm-builder plugin's pure core.
//! No wasm toolchain required — `cargo test` from this directory is enough.
//!
//! Exercises the args parsing + config-injection contract the ZeroClaw host
//! provides (plugin config section injected under `__config`), and the
//! fail-closed configuration rules.

use dlmm_builder::build::{build_action, parse_key, resolve_nonce, ActionInput, Args, DEFAULT_DLMM_PROGRAM};

#[test]
fn args_parse_with_config_injection() {
    let args: Args = serde_json::from_str(
        r#"{
            "mode": "rebalance",
            "position_id": "7fTxDfcWTMVJg2r26Jv496HsuEBi6Hc77QEsHE9NSVZ1",
            "nonce_address": "46Zno59Ksbc6fEXzfQr9aqbJdc4s4zi4JeXumwgXzFwX",
            "new_low": 8450,
            "new_high": 8520,
            "__config": {
                "rpc_url": "https://api.mainnet-beta.solana.com",
                "owner_pubkey": "2KDS5vtFQJyYJApNGoPVaBSaYP7Vp4R3txRM9SYj13BW",
                "dlmm_program": "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
                "nonce_addresses": [
                    "46Zno59Ksbc6fEXzfQr9aqbJdc4s4zi4JeXumwgXzFwX",
                    "8xWjKvR3TzP4mNs6qLpYbD5hGfJcE7aA9bC2dE1fG3hJ"
                ]
            }
        }"#,
    )
    .unwrap();
    assert_eq!(args.mode, "rebalance");
    assert_eq!(args.new_low, Some(8450));
    let cfg = args.config.expect("__config injected");
    assert_eq!(cfg.rpc_url, "https://api.mainnet-beta.solana.com");
    // Args nonce hint matches the first pool entry.
    assert_eq!(
        resolve_nonce(
            cfg.nonce_addresses.as_deref().unwrap_or(&[]),
            args.nonce_address.as_deref()
        )
        .as_deref(),
        Some("46Zno59Ksbc6fEXzfQr9aqbJdc4s4zi4JeXumwgXzFwX"),
    );
}

#[test]
fn args_without_config_are_flagged() {
    let args: Args = serde_json::from_str(r#"{"mode": "claim"}"#).unwrap();
    assert!(args.config.is_none());
    // the wasm shim rejects this: "missing __config (config_read permission?)"
}

#[test]
fn owner_pubkey_is_required_for_any_action() {
    // The builder hard-requires owner_pubkey — unlike the reader, there is no
    // "report-only" mode; every action needs a signer identity to gate on.
    let args: Args = serde_json::from_str(
        r#"{
            "mode": "claim",
            "position_id": "pos",
            "nonce_address": "nonce",
            "__config": { "rpc_url": "https://x" }
        }"#,
    )
    .unwrap();
    let cfg = args.config.unwrap();
    assert!(cfg.owner_pubkey.is_none());
}

#[test]
fn nonce_address_alias_accepts_singular_legacy_form() {
    // Backward compatibility: a single `nonce_address` (singular) deserializes
    // into the `nonce_addresses` pool as a one-element vector.
    let args: Args = serde_json::from_str(
        r#"{
            "mode": "claim",
            "position_id": "pos",
            "__config": {
                "rpc_url": "https://x",
                "owner_pubkey": "2KDS5vtFQJyYJApNGoPVaBSaYP7Vp4R3txRM9SYj13BW",
                "nonce_address": "46Zno59Ksbc6fEXzfQr9aqbJdc4s4zi4JeXumwgXzFwX"
            }
        }"#,
    )
    .unwrap();
    let cfg = args.config.unwrap();
    let pool = cfg.nonce_addresses.expect("legacy alias should populate pool");
    assert_eq!(pool.len(), 1);
    assert_eq!(
        pool[0],
        "46Zno59Ksbc6fEXzfQr9aqbJdc4s4zi4JeXumwgXzFwX"
    );
}

#[test]
fn default_program_is_devnet_dlmm() {
    assert_eq!(
        DEFAULT_DLMM_PROGRAM,
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo"
    );
    // And it parses — so the shim's fallback can't panic on a typo.
    assert!(parse_key(DEFAULT_DLMM_PROGRAM, "dlmm_program").is_ok());
}

#[test]
fn build_action_requires_valid_mode() {
    // Exercises the pure entry without touching RPC: unknown mode must fail
    // before any account data is needed. A minimal input is enough.
    let env = Env::new();
    let pos = dlmm_core::decoder::decode_position(&b64(&env.f["position_v2"])).unwrap();
    let lb = dlmm_core::decoder::decode_lb_pair(&b64(&env.f["lb_pair"])).unwrap();
    let err = build_action(&env.input("swap", &pos, &lb, &[], None, None)).unwrap_err();
    assert!(err.contains("unknown mode"), "{err}");
}

// --- shared fixture helpers (mirrors dlmm-reader/tests/host_integration.rs) ---

use base64::Engine;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../dlmm-core/tests/fixtures.json")).unwrap()
}

fn b64(v: &serde_json::Value) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(v.as_str().unwrap())
        .unwrap()
}

struct Env {
    f: serde_json::Value,
    position_key: solana_pubkey::Pubkey,
    owner: solana_pubkey::Pubkey,
    nonce: solana_pubkey::Pubkey,
    program: solana_pubkey::Pubkey,
}

impl Env {
    fn new() -> Self {
        let f = fixture();
        Env {
            position_key: pk(&f["keys"]["position"]),
            owner: pk(&f["keys"]["owner"]),
            nonce: pk(&f["keys"]["nonce"]),
            program: pk(&f["keys"]["program"]),
            f,
        }
    }

    fn input<'a>(
        &'a self,
        mode: &'a str,
        position: &'a dlmm_core::decoder::PositionV2,
        lb_pair: &'a dlmm_core::decoder::LbPair,
        bin_arrays: &'a [dlmm_core::decoder::BinArray],
        new_low: Option<i32>,
        new_high: Option<i32>,
    ) -> ActionInput<'a> {
        ActionInput {
            mode,
            position_id: "pos",
            position_key: &self.position_key,
            position,
            lb_pair,
            bin_arrays,
            nonce_hash: bs58::decode(self.f["keys"]["nonce_hash"].as_str().unwrap())
                .into_vec()
                .unwrap()
                .try_into()
                .unwrap(),
            owner: &self.owner,
            nonce: &self.nonce,
            program: &self.program,
            token_program_x: dlmm_core::tx::TOKEN_PROGRAM_ID.parse().unwrap(),
            token_program_y: dlmm_core::tx::TOKEN_PROGRAM_ID.parse().unwrap(),
            new_low,
            new_high,
            use_nonce: true,
            label: None,
        }
    }
}

fn pk(v: &serde_json::Value) -> solana_pubkey::Pubkey {
    v.as_str().unwrap().parse().unwrap()
}
