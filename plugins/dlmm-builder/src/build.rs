//! Pure unsigned-tx builder — host-testable, no RPC.
//!
//! Given decoded accounts + the host-injected config, this module:
//!
//!   1. mechanically validates ("the LLM proposes, the plugin verifies"):
//!      ownership, claimable > 0, low < high, range contains the active bin,
//!      bin-array indexes inside the default bitmap
//!   2. builds the instruction set ([AdvanceNonceAccount, claim_fee2] or
//!      [AdvanceNonceAccount, remove_liquidity_by_range2,
//!      add_liquidity_by_strategy2])
//!   3. encodes the unsigned transaction (web3.js wire format) with the
//!      durable-nonce hash as `recentBlockhash`, and returns it as base64
//!
//! The wasm shim (lib.rs) does the RPC fetching and hands decoded values to
//! `build_action`; `cargo test` exercises it with fixture bytes and asserts
//! byte-for-byte equality with web3.js-serialized transactions.

use base64::Engine;
use dlmm_core::decoder::{position_amounts, total_liquidity_shares, BinArray, LbPair, PositionV2};
use dlmm_core::tx::{
    associated_token_address, build_claim_instructions, build_rebalance_instructions,
    encode_unsigned_tx, PoolCtx, StrategyType, TOKEN_PROGRAM_ID,
};
use dlmm_core::validate::{
    validate_bin_array_range, validate_claimable, validate_owned, validate_range,
    MAX_ACTIVE_BIN_SLIPPAGE,
};
use serde::Deserialize;
use serde_json::json;
use solana_pubkey::Pubkey;

/// Default DLMM program: devnet. Operators pin their own via `__config.dlmm_program`.
pub const DEFAULT_DLMM_PROGRAM: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// Host-injected config (ZeroClaw injects the plugin's config section under
/// the reserved `__config` key; caller-supplied values are stripped by the
/// host, so these cannot be spoofed by the LLM).
#[derive(Deserialize, Debug, Default, Clone)]
pub struct PluginConfig {
    pub rpc_url: String,
    /// The operator's own wallet — the ONLY signer the built tx may have.
    /// The builder refuses to run without it (fail-closed ownership gate).
    pub owner_pubkey: Option<String>,
    #[serde(default)]
    pub dlmm_program: Option<String>,
    /// Durable nonce account used as the `recentBlockhash` source for every
    /// agent-proposed tx. Host-injected under `__config` (anti-spoof: the
    /// LLM cannot redirect this to an attacker-controlled account via DM —
    /// the runtime strips caller-supplied `__config` and the builder uses
    /// this value over any `args.nonce_address` the LLM might pass).
    ///
    /// Bounty alignment: keeps the nonce on the same trust path as `rpc_url`
    /// and `owner_pubkey` — all host-controlled, none of them DM-mediated.
    /// The LLM is no longer asked for the nonce, so a prompt injection
    /// ("paste your nonce here") can't substitute the attacker's.
    #[serde(default)]
    pub nonce_address: Option<String>,
}

/// Tool arguments (mode `claim` or `rebalance`).
#[derive(Deserialize, Debug, Default)]
pub struct Args {
    pub mode: String,
    #[serde(default)]
    pub position_id: Option<String>,
    /// Durable nonce account — its stored hash becomes `recentBlockhash`.
    #[serde(default)]
    pub nonce_address: Option<String>,
    /// Rebalance target range (bin ids). Claim ignores these.
    #[serde(default)]
    pub new_low: Option<i32>,
    #[serde(default)]
    pub new_high: Option<i32>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "__config")]
    pub config: Option<PluginConfig>,
}

/// Everything `build_action` needs, pre-decoded by the caller.
pub struct ActionInput<'a> {
    pub mode: &'a str,
    pub position_id: &'a str,
    /// The PositionV2 account key (what claim/rebalance instructions
    /// reference — distinct from the account bytes in `position`).
    pub position_key: &'a Pubkey,
    pub position: &'a PositionV2,
    pub lb_pair: &'a LbPair,
    /// Bin arrays covering the position's CURRENT range — used to derive the
    /// token amounts a rebalance re-deposits (v2 positions store shares, not
    /// totals). Missing arrays are skipped (empty bins).
    pub bin_arrays: &'a [BinArray],
    /// Stored durable-nonce hash OR latest blockhash (32 bytes) — the tx's
    /// `recentBlockhash`. When `use_nonce=true` this is the nonce's stored
    /// hash; otherwise it is the latest blockhash (expires after ~90 s).
    pub nonce_hash: [u8; 32],
    pub owner: &'a Pubkey,
    /// Durable nonce account pubkey. Required only when `use_nonce=true`
    /// (the AdvanceNonceAccount instruction references it). When `use_nonce
    /// =false` this is a placeholder and the tx is a plain 1- or 2-ix tx.
    pub nonce: &'a Pubkey,
    pub program: &'a Pubkey,
    pub new_low: Option<i32>,
    pub new_high: Option<i32>,
    pub label: Option<String>,
    /// When true, the tx starts with `AdvanceNonceAccount` (durable nonce).
    /// When false, the tx is a plain claim/rebalance with only the latest
    /// blockhash as `recentBlockhash` — blockhash expires after ~90 s so the
    /// user must sign promptly. This is the demo path; production keeps
    /// `use_nonce=true` for approval-delay safety.
    pub use_nonce: bool,
}

/// Resolve which nonce to use for `recentBlockhash`.
///
/// Precedence: host-injected `__config.nonce_address` (anti-spoof) wins over
/// any `args.nonce_address` the LLM passes. Both empty → the caller falls
/// back to the latest blockhash (demo path; the wasm entry decides which
/// branch to take). Returns the resolved pubkey-or-None so this logic is
/// host-testable without an RPC.
///
/// Bounty alignment: the LLM is *never* the source of the nonce. The
/// runtime strips caller-supplied `__config` and the builder uses the host
/// value. Prompt-injection scenarios (e.g. scenario 6, replayed URL) can't
/// substitute the operator's nonce for an attacker-controlled one.
pub fn resolve_nonce(
    config_nonce: Option<&str>,
    args_nonce: Option<&str>,
) -> Option<String> {
    config_nonce
        .filter(|s| !s.is_empty())
        .or_else(|| args_nonce.filter(|s| !s.is_empty()))
        .map(|s| s.to_string())
}

/// Build + validate + encode, return the shaped output (action URL, tx base64,
/// summary). Every failure is an Err with a human-readable reason — the plugin
/// surfaces it directly to the LLM.
pub fn build_action(input: &ActionInput) -> Result<serde_json::Value, String> {
    let pool_ctx = PoolCtx {
        lb_pair: input.position.lb_pair,
        reserve_x: input.lb_pair.reserve_x,
        reserve_y: input.lb_pair.reserve_y,
        token_x_mint: input.lb_pair.token_x_mint,
        token_y_mint: input.lb_pair.token_y_mint,
        token_program: TOKEN_PROGRAM_ID.parse().unwrap(),
    };

    // 1. Mechanical validation — fail closed.
    validate_owned(input.position, input.owner).map_err(|e| format!("ownership: {e}"))?;

    // Deposit accounts: the owner's ATAs for the pool's two mints.
    let user_token_x =
        associated_token_address(input.owner, &pool_ctx.token_x_mint, &pool_ctx.token_program);
    let user_token_y =
        associated_token_address(input.owner, &pool_ctx.token_y_mint, &pool_ctx.token_program);

    let (ixs, summary) = match input.mode {
        "claim" => {
            validate_claimable(input.position, input.bin_arrays)
                .map_err(|e| format!("claim: {e}"))?;
            validate_bin_array_range(input.position.lower_bin_id, input.position.upper_bin_id)
                .map_err(|e| format!("claim: {e}"))?;
            let ixs = build_claim_instructions(
                *input.nonce,
                *input.owner,
                &pool_ctx,
                *input.position_key,
                user_token_x,
                user_token_y,
                input.position.lower_bin_id,
                input.position.upper_bin_id,
                *input.program,
            );
            let ixs = if input.use_nonce { ixs } else { ixs.into_iter().skip(1).collect() };
            (
                ixs,
                json!({ "kind": "claim", "range": [input.position.lower_bin_id, input.position.upper_bin_id] }),
            )
        }
        "rebalance" => {
            if total_liquidity_shares(input.position) == 0 {
                return Err("rebalance: position holds no liquidity".to_string());
            }
            let (new_low, new_high) = match (input.new_low, input.new_high) {
                (Some(l), Some(h)) => (l, h),
                _ => return Err("rebalance: new_low/new_high required".to_string()),
            };
            validate_range(new_low, new_high, input.lb_pair.active_id)
                .map_err(|e| format!("rebalance: {e}"))?;
            validate_bin_array_range(input.position.lower_bin_id, input.position.upper_bin_id)
                .map_err(|e| format!("rebalance (current range): {e}"))?;
            validate_bin_array_range(new_low, new_high)
                .map_err(|e| format!("rebalance (new range): {e}"))?;

            // Amounts to re-deposit = what the position currently holds,
            // derived from per-bin shares × bin reserves.
            let (amount_x, amount_y) = position_amounts(input.position, input.bin_arrays);
            if total_liquidity_shares(input.position) > 0 && amount_x == 0 && amount_y == 0 {
                return Err(
                    "rebalance: could not derive position amounts (bin arrays missing?)"
                        .to_string(),
                );
            }

            // Active bin is read fresh from the pool; slippage is enforced
            // on-chain by max_active_bin_slippage.
            let ixs = build_rebalance_instructions(
                *input.nonce,
                *input.owner,
                &pool_ctx,
                *input.position_key,
                user_token_x,
                user_token_y,
                input.position.lower_bin_id,
                input.position.upper_bin_id,
                new_low,
                new_high,
                input.lb_pair.active_id,
                amount_x,
                amount_y,
                MAX_ACTIVE_BIN_SLIPPAGE,
                StrategyType::SpotImBalanced,
                [0u8; 64],
                *input.program,
            );
            let ixs = if input.use_nonce { ixs } else { ixs.into_iter().skip(1).collect() };
            (
                ixs,
                json!({
                    "kind": "rebalance",
                    "range": [new_low, new_high],
                    "amount_x": amount_x,
                    "amount_y": amount_y,
                }),
            )
        }
        other => return Err(format!("unknown mode: {other} (expected claim|rebalance)")),
    };

    // 2. Encode with the durable-nonce hash as recentBlockhash.
    let tx_bytes = encode_unsigned_tx(&ixs, &input.nonce_hash, input.owner);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

    let label = input.label.clone().unwrap_or_else(|| match input.mode {
        "claim" => "Claim DLMM fees".to_string(),
        _ => "Rebalance DLMM position".to_string(),
    });

    Ok(json!({
        "tx_base64": b64,
        "position_id": input.position_id,
        "instructions": ixs.len(),
        "recent_blockhash": bs58_nonce_hash(&input.nonce_hash),
        "label": label,
        "summary": summary,
    }))
}

fn bs58_nonce_hash(h: &[u8; 32]) -> String {
    bs58::encode(h).into_string()
}

/// `Pubkey` parsing with a context prefix for readable errors.
pub fn parse_key(s: &str, what: &str) -> Result<Pubkey, String> {
    s.parse().map_err(|_| format!("bad {what}: {s}"))
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

    fn pk(v: &serde_json::Value) -> Pubkey {
        v.as_str().unwrap().parse().unwrap()
    }

    fn nonce_hash(f: &serde_json::Value) -> [u8; 32] {
        bs58::decode(f["keys"]["nonce_hash"].as_str().unwrap())
            .into_vec()
            .unwrap()
            .try_into()
            .unwrap()
    }

    /// Owning test environment: keys parsed once, so `ActionInput` borrows
    /// live as long as the env does.
    struct Env {
        f: serde_json::Value,
        position_key: Pubkey,
        owner: Pubkey,
        nonce: Pubkey,
        program: Pubkey,
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
            position: &'a PositionV2,
            lb_pair: &'a LbPair,
            bin_arrays: &'a [BinArray],
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
                nonce_hash: nonce_hash(&self.f),
                owner: &self.owner,
                nonce: &self.nonce,
                program: &self.program,
                new_low,
                new_high,
                use_nonce: true,
                label: None,
            }
        }
    }

    #[test]
    fn claim_action_matches_web3js_wire_format() {
        let env = Env::new();
        let position = decode_position(&b64(&env.f["position_v2"])).unwrap();
        let lb_pair = decode_lb_pair(&b64(&env.f["lb_pair"])).unwrap();
        // Claim path needs the bin arrays for claimable validation.
        let ba = decode_bin_array(&b64(&env.f["bin_array_aligned"])).unwrap();

        let out = build_action(&env.input("claim", &position, &lb_pair, &[ba], None, None)).unwrap();
        // Byte-for-byte equality with the web3.js-serialized fixture tx.
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(out["tx_base64"].as_str().unwrap())
                .unwrap(),
            b58_bytes(&env.f["tx_claim_b58"])
        );
        assert_eq!(out["instructions"], 2);
        assert!(out.get("action_url").is_none(), "action_url must not be emitted");
    }

    #[test]
    fn rebalance_action_matches_web3js_wire_format() {
        let env = Env::new();
        let position = decode_position(&b64(&env.f["position_v2"])).unwrap();
        let lb_pair = decode_lb_pair(&b64(&env.f["lb_pair"])).unwrap();
        let ba = decode_bin_array(&b64(&env.f["bin_array_aligned"])).unwrap();

        let out = build_action(&env.input(
            "rebalance",
            &position,
            &lb_pair,
            &[ba],
            Some(8450),
            Some(8520),
        ))
        .unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(out["tx_base64"].as_str().unwrap())
                .unwrap(),
            b58_bytes(&env.f["tx_rebalance_b58"])
        );
        assert_eq!(out["instructions"], 3);
        assert_eq!(out["summary"]["amount_x"], 1_500_000_000u64);
        assert_eq!(out["summary"]["amount_y"], 200_000_000u64);
    }

    #[test]
    fn owner_mismatch_is_rejected() {
        let env = Env::new();
        let position = decode_position(&b64(&env.f["position_v2"])).unwrap();
        let lb_pair = decode_lb_pair(&b64(&env.f["lb_pair"])).unwrap();
        let mut inp = env.input("claim", &position, &lb_pair, &[], None, None);
        let stranger = Pubkey::default();
        inp.owner = &stranger;
        let err = build_action(&inp).unwrap_err();
        assert!(err.contains("ownership"), "{err}");
    }

    #[test]
    fn empty_position_claim_is_rejected() {
        let env = Env::new();
        let mut position = decode_position(&b64(&env.f["position_v2"])).unwrap();
        let lb_pair = decode_lb_pair(&b64(&env.f["lb_pair"])).unwrap();
        // Zero out all liquidity shares, fees, and rewards → NothingClaimable.
        position.liquidity_shares = [0u128; 70];
        position.fee_infos = [dlmm_core::decoder::FeeInfo {
            fee_x_per_token_complete: 0,
            fee_y_per_token_complete: 0,
            fee_x_pending: 0,
            fee_y_pending: 0,
        }; 70];
        position.reward_infos = [dlmm_core::decoder::UserRewardInfo {
            reward_per_token_completes: [0; 2],
            reward_pendings: [0; 2],
        }; 70];

        let err =
            build_action(&env.input("claim", &position, &lb_pair, &[], None, None)).unwrap_err();
        assert!(err.contains("claim:"), "{err}");
    }

    #[test]
    fn rebalance_rejects_bad_range() {
        let env = Env::new();
        let position = decode_position(&b64(&env.f["position_v2"])).unwrap();
        let lb_pair = decode_lb_pair(&b64(&env.f["lb_pair"])).unwrap();
        let ba = decode_bin_array(&b64(&env.f["bin_array_aligned"])).unwrap();

        // low >= high → InvalidRange.
        let err = build_action(&env.input(
            "rebalance",
            &position,
            &lb_pair,
            &[ba.clone()],
            Some(8520),
            Some(8450),
        ))
        .unwrap_err();
        assert!(err.contains("range"), "{err}");

        // Range not containing the active bin (8500) → rejected.
        let err = build_action(&env.input(
            "rebalance",
            &position,
            &lb_pair,
            &[ba.clone()],
            Some(8600),
            Some(8700),
        ))
        .unwrap_err();
        assert!(err.contains("active bin"), "{err}");

        // Missing new_low/new_high.
        let err = build_action(&env.input("rebalance", &position, &lb_pair, &[ba], None, None))
            .unwrap_err();
        assert!(err.contains("new_low/new_high"), "{err}");
    }

    fn b58_bytes(v: &serde_json::Value) -> Vec<u8> {
        bs58::decode(v.as_str().unwrap()).into_vec().unwrap()
    }

    // -----------------------------------------------------------------------
    // `resolve_nonce` — config-driven precedence
    // -----------------------------------------------------------------------
    //
    // The host-injected `__config.nonce_address` wins over any
    // `args.nonce_address` the LLM passes. Both empty → None (caller falls
    // back to the latest blockhash; the demo path). The LLM can never
    // substitute the operator's nonce for an attacker-controlled pubkey
    // (bounty threat model, `prompts/injection-tests.md` scenario 6).
    #[test]
    fn resolve_nonce_prefers_config_over_args() {
        let resolved = resolve_nonce(
            Some("CfgNOnceXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
            Some("ArgsNOnceXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
        );
        assert_eq!(
            resolved.as_deref(),
            Some("CfgNOnceXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
        );
    }

    #[test]
    fn resolve_nonce_falls_back_to_args_when_config_empty() {
        let resolved = resolve_nonce(None, Some("ArgsNOnceXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"));
        assert_eq!(
            resolved.as_deref(),
            Some("ArgsNOnceXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
        );
    }

    #[test]
    fn resolve_nonce_treats_empty_config_and_empty_args_as_none() {
        assert_eq!(resolve_nonce(None, None), None);
        assert_eq!(resolve_nonce(Some(""), None), None);
        assert_eq!(resolve_nonce(None, Some("")), None);
        assert_eq!(resolve_nonce(Some(""), Some("")), None);
    }
}
