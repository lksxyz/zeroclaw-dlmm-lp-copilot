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
use serde::{Deserialize, Deserializer};
use serde_json::json;
use solana_pubkey::Pubkey;

/// Default DLMM program: devnet. Operators pin their own via `__config.dlmm_program`.
pub const DEFAULT_DLMM_PROGRAM: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// Accept either `"nonce_addresses": ["a", "b"]` (preferred, plural pool)
/// or `"nonce_address": "a"` (legacy single-nonce setups, treated as a
/// one-element pool). Lets operators migrate without breaking their
/// existing `config.toml`.
fn deserialize_nonce_addresses<'de, D>(d: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let v: serde_json::Value = Deserialize::deserialize(d)?;
    match v {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => {
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(vec![s]))
            }
        }
        serde_json::Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                let s = item
                    .as_str()
                    .ok_or_else(|| Error::custom("nonce_addresses: expected string"))?;
                out.push(s.to_string());
            }
            Ok(Some(out))
        }
        other => Err(Error::custom(format!(
            "nonce_addresses: expected string or array, got {other}"
        ))),
    }
}

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
    /// Pool of durable nonce accounts the builder may use as `recentBlockhash`
    /// sources. Host-injected under `__config` (anti-spoof: the runtime strips
    /// caller-supplied `__config`, so the LLM cannot redirect the pool to
    /// attacker-controlled accounts via DM).
    ///
    /// Bounty alignment: one nonce account serializes to one in-flight
    /// transaction. With a pool, the agent/SOP can issue parallel pending
    /// approvals (e.g. claim + rebalance on different positions) without
    /// double-advance. The LLM hints which pool slot to use via
    /// `args.nonce_address`; the plugin validates the hint is inside the
    /// pool, so a prompt injection ("use this nonce instead") can't
    /// substitute an attacker-controlled pubkey.
    ///
    /// Accepts either `nonce_addresses` (array — preferred) or
    /// `nonce_address` (single — legacy single-nonce setups).
    #[serde(
        default,
        alias = "nonce_address",
        deserialize_with = "deserialize_nonce_addresses"
    )]
    pub nonce_addresses: Option<Vec<String>>,
}

/// Tool arguments (mode `claim` or `rebalance`).
#[derive(Deserialize, Debug, Default)]
pub struct Args {
    pub mode: String,
    #[serde(default)]
    pub position_id: Option<String>,
    /// Durable nonce account hint — must be inside `__config.nonce_addresses`.
    /// The plugin validates the hint against the host-injected pool and
    /// falls back to the first pool entry if the hint is missing or invalid.
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
    /// Token program owning the pool's token X mint (SPL Token or Token-2022).
    /// Determines the user's X ATA address. Defaults to SPL Token.
    pub token_program_x: Pubkey,
    /// Token program owning the pool's token Y mint.
    pub token_program_y: Pubkey,
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

/// Resolve which nonce to use for `recentBlockhash` from the operator's pool.
///
/// Precedence:
///   1. `args.nonce_address` — the LLM's pick — but only if it is inside
///      the host-injected pool. This is the anti-spoof gate: a prompt
///      injection asking for a different nonce can't escape the pool.
///   2. First non-empty entry of `config_nonce_pool` — the host's default
///      when the LLM doesn't hint (or hints something invalid).
///   3. None — caller falls back to the latest blockhash (demo path; the
///      wasm entry decides which branch to take).
///
/// Bounty alignment: the LLM is never the source of the nonce pool. The
/// runtime strips caller-supplied `__config` so the pool is always
/// host-controlled. Prompt-injection scenarios (e.g. scenario 6, replayed
/// URL with attacker-controlled nonce) can't substitute the operator's
/// nonces — the hint must match a pool entry or be ignored.
pub fn resolve_nonce(
    config_nonce_pool: &[String],
    args_nonce: Option<&str>,
) -> Option<String> {
    let cleaned: Vec<&str> = config_nonce_pool
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .collect();

    // 1. LLM hint — only valid if it matches a pool entry.
    if let Some(hint) = args_nonce.filter(|s| !s.is_empty()) {
        if cleaned.iter().any(|p| *p == hint) {
            return Some(hint.to_string());
        }
    }

    // 2. First pool entry (deterministic host default).
    cleaned.first().map(|s| s.to_string())
}

/// Build + validate + encode, return the shaped output (action URL, tx base64,
/// summary). Every failure is an Err with a human-readable reason — the plugin
/// surfaces it directly to the LLM.
pub fn build_action(input: &ActionInput) -> Result<serde_json::Value, String> {
    // Per-mint token programs: an ATA must be derived against the token
    // program that owns the mint (SPL Token vs Token-2022). The wasm shim
    // resolves these from the mint accounts' owner; defaults to SPL Token.
    let token_program_x = input.token_program_x;
    let token_program_y = input.token_program_y;
    let pool_ctx = PoolCtx {
        lb_pair: input.position.lb_pair,
        reserve_x: input.lb_pair.reserve_x,
        reserve_y: input.lb_pair.reserve_y,
        token_x_mint: input.lb_pair.token_x_mint,
        token_y_mint: input.lb_pair.token_y_mint,
        token_program_x,
        token_program_y,
    };

    // 1. Mechanical validation — fail closed.
    validate_owned(input.position, input.owner).map_err(|e| format!("ownership: {e}"))?;

    // Deposit accounts: the owner's ATAs for the pool's two mints, each
    // derived against the mint's own token program.
    let user_token_x =
        associated_token_address(input.owner, &pool_ctx.token_x_mint, &token_program_x);
    let user_token_y =
        associated_token_address(input.owner, &pool_ctx.token_y_mint, &token_program_y);

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
                token_program_x: TOKEN_PROGRAM_ID.parse().unwrap(),
                token_program_y: TOKEN_PROGRAM_ID.parse().unwrap(),
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
    // `resolve_nonce` — pool-driven, anti-spoof
    // -----------------------------------------------------------------------
    //
    // The host-injected `__config.nonce_addresses` pool is the source of
    // truth. The LLM's `args.nonce_address` is accepted only if it matches a
    // pool entry; otherwise the first pool entry is used. The LLM can never
    // escape the pool (bounty threat model, `prompts/injection-tests.md`
    // scenario 6).
    #[test]
    fn resolve_nonce_uses_args_when_in_pool() {
        let pool = vec![
            "PoolNOnceAXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            "PoolNOnceBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
        ];
        let resolved = resolve_nonce(&pool, Some("PoolNOnceBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"));
        assert_eq!(
            resolved.as_deref(),
            Some("PoolNOnceBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
        );
    }

    #[test]
    fn resolve_nonce_rejects_args_outside_pool() {
        let pool = vec!["PoolNOnceAXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string()];
        // Attacker-supplied nonce — not in pool — falls back to pool head.
        let resolved = resolve_nonce(
            &pool,
            Some("AttackerXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
        );
        assert_eq!(
            resolved.as_deref(),
            Some("PoolNOnceAXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
        );
    }

    #[test]
    fn resolve_nonce_falls_back_to_pool_head_when_args_missing() {
        let pool = vec![
            "PoolNOnceAXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            "PoolNOnceBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
        ];
        assert_eq!(
            resolve_nonce(&pool, None).as_deref(),
            Some("PoolNOnceAXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
        );
    }

    #[test]
    fn resolve_nonce_treats_empty_pool_and_empty_args_as_none() {
        assert_eq!(resolve_nonce(&[], None), None);
        assert_eq!(resolve_nonce(&[], Some("ArgsNOnceXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX")), None);
        let empty_pool = vec!["".to_string()];
        assert_eq!(resolve_nonce(&empty_pool, None), None);
        assert_eq!(resolve_nonce(&empty_pool, Some("")), None);
    }

    #[test]
    fn resolve_nonce_skips_empty_pool_entries() {
        let pool = vec![
            "".to_string(),
            "PoolNOnceAXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
        ];
        assert_eq!(
            resolve_nonce(&pool, None).as_deref(),
            Some("PoolNOnceAXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
        );
    }
}
