//! DLMM transaction building: the three v3 instructions (claim_fee2,
//! remove_liquidity_by_range2, add_liquidity_by_strategy2), the durable-nonce
//! AdvanceNonceAccount, and legacy unsigned-message encoding.
//!
//! Instruction layouts come from the DLMM program IDL (Anchor). Account lists
//! and argument encoding are verified against web3.js-serialized fixtures in
//! tests/cross_check.rs. `encode_unsigned_tx` reproduces the web3.js legacy
//! message layout (account order: signers → non-signers → new program ids,
//! fee payer first).

use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::pda;

pub const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const SYSVAR_RECENT_BLOCKHASHES: &str = "SysvarRecentB1ockHashes11111111111111111111";

// Anchor discriminators (sha256("global:<name>")[..8]).
pub const CLAIM_FEE2_DISC: [u8; 8] = [0x70, 0xbf, 0x65, 0xab, 0x1c, 0x90, 0x7f, 0xbb];
pub const REMOVE_LIQUIDITY_BY_RANGE2_DISC: [u8; 8] =
    [0xcc, 0x02, 0xc3, 0x91, 0x35, 0x91, 0x91, 0xcd];
pub const ADD_LIQUIDITY_BY_STRATEGY2_DISC: [u8; 8] =
    [0x03, 0xdd, 0x95, 0xda, 0x6f, 0x8d, 0x76, 0xd5];

/// AccountsType discriminant (IDL enum order).
pub const ACCOUNTS_TYPE_TRANSFER_HOOK_X: u8 = 0;
pub const ACCOUNTS_TYPE_TRANSFER_HOOK_Y: u8 = 1;

/// StrategyType discriminant (IDL enum order).
#[derive(Debug, Clone, Copy)]
pub enum StrategyType {
    SpotOneSide = 0,
    CurveOneSide = 1,
    BidAskOneSide = 2,
    SpotBalanced = 3,
    CurveBalanced = 4,
    BidAskBalanced = 5,
    SpotImBalanced = 6,
    CurveImBalanced = 7,
    BidAskImBalanced = 8,
}

/// RemainingAccountsInfo for SPL pools: the SDK always emits the two
/// zero-length transfer-hook slices (X, Y) for the `Liquidity` action type.
pub fn spl_transfer_hook_slices() -> Vec<u8> {
    let mut v = (2u32).to_le_bytes().to_vec();
    v.push(ACCOUNTS_TYPE_TRANSFER_HOOK_X);
    v.push(0);
    v.push(ACCOUNTS_TYPE_TRANSFER_HOOK_Y);
    v.push(0);
    v
}

/// Floor division by 70 for bin ids → bin-array index (SDK `divmod` floor).
pub fn bin_id_to_bin_array_index(bin_id: i32) -> i64 {
    const BINS_PER_ARRAY: i64 = 70;
    let b = bin_id as i64;
    let q = b / BINS_PER_ARRAY;
    let r = b % BINS_PER_ARRAY;
    if r != 0 && b < 0 {
        q - 1
    } else {
        q
    }
}

/// All bin-array indexes covering `[min, max]` (inclusive), in order.
pub fn bin_array_indexes_for_range(min_bin_id: i32, max_bin_id: i32) -> Vec<i64> {
    let lo = bin_id_to_bin_array_index(min_bin_id);
    let hi = bin_id_to_bin_array_index(max_bin_id);
    (lo..=hi).collect()
}

/// ATA PDA (seeds [owner, token_program, mint]) — see pda.rs.
pub fn associated_token_address(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    pda::associated_token_address(owner, mint, token_program)
}

/// Writable bin-array metas covering `[min, max]`.
pub fn bin_array_metas(
    lb_pair: &Pubkey,
    min_bin_id: i32,
    max_bin_id: i32,
    program_id: &Pubkey,
) -> Vec<AccountMeta> {
    bin_array_indexes_for_range(min_bin_id, max_bin_id)
        .into_iter()
        .map(|idx| AccountMeta::new(pda::bin_array(lb_pair, idx, program_id).0, false))
        .collect()
}

// ---------------------------------------------------------------------------
// Instructions
// ---------------------------------------------------------------------------

/// Pool context shared by all three instructions.
#[derive(Debug, Clone)]
pub struct PoolCtx {
    pub lb_pair: Pubkey,
    pub reserve_x: Pubkey,
    pub reserve_y: Pubkey,
    pub token_x_mint: Pubkey,
    pub token_y_mint: Pubkey,
    pub token_program: Pubkey,
}

pub fn claim_fee2_ix(
    pool: &PoolCtx,
    position: Pubkey,
    sender: Pubkey,
    user_token_x: Pubkey,
    user_token_y: Pubkey,
    min_bin_id: i32,
    max_bin_id: i32,
    bin_arrays: Vec<Pubkey>,
    memo_program: Pubkey,
    event_authority: Pubkey,
    program_id: Pubkey,
) -> Instruction {
    let mut data = CLAIM_FEE2_DISC.to_vec();
    data.extend_from_slice(&min_bin_id.to_le_bytes());
    data.extend_from_slice(&max_bin_id.to_le_bytes());
    data.extend_from_slice(&spl_transfer_hook_slices());

    let mut accounts = vec![
        AccountMeta::new(pool.lb_pair, false),
        AccountMeta::new(position, false),
        AccountMeta::new_readonly(sender, true),
        AccountMeta::new(pool.reserve_x, false),
        AccountMeta::new(pool.reserve_y, false),
        AccountMeta::new(user_token_x, false),
        AccountMeta::new(user_token_y, false),
        AccountMeta::new_readonly(pool.token_x_mint, false),
        AccountMeta::new_readonly(pool.token_y_mint, false),
        AccountMeta::new_readonly(pool.token_program, false),
        AccountMeta::new_readonly(pool.token_program, false),
        AccountMeta::new_readonly(memo_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];
    accounts.extend(bin_arrays.into_iter().map(|k| AccountMeta::new(k, false)));

    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn remove_liquidity_by_range2_ix(
    pool: &PoolCtx,
    position: Pubkey,
    sender: Pubkey,
    user_token_x: Pubkey,
    user_token_y: Pubkey,
    from_bin_id: i32,
    to_bin_id: i32,
    bps_to_remove: u16,
    bin_array_bitmap_extension: Pubkey,
    bin_arrays: Vec<Pubkey>,
    memo_program: Pubkey,
    event_authority: Pubkey,
    program_id: Pubkey,
) -> Instruction {
    let mut data = REMOVE_LIQUIDITY_BY_RANGE2_DISC.to_vec();
    data.extend_from_slice(&from_bin_id.to_le_bytes());
    data.extend_from_slice(&to_bin_id.to_le_bytes());
    data.extend_from_slice(&bps_to_remove.to_le_bytes());
    data.extend_from_slice(&spl_transfer_hook_slices());

    let mut accounts = vec![
        AccountMeta::new(position, false),
        AccountMeta::new(pool.lb_pair, false),
        AccountMeta::new(bin_array_bitmap_extension, false),
        AccountMeta::new(user_token_x, false),
        AccountMeta::new(user_token_y, false),
        AccountMeta::new(pool.reserve_x, false),
        AccountMeta::new(pool.reserve_y, false),
        AccountMeta::new_readonly(pool.token_x_mint, false),
        AccountMeta::new_readonly(pool.token_y_mint, false),
        AccountMeta::new_readonly(sender, true),
        AccountMeta::new_readonly(pool.token_program, false),
        AccountMeta::new_readonly(pool.token_program, false),
        AccountMeta::new_readonly(memo_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];
    accounts.extend(bin_arrays.into_iter().map(|k| AccountMeta::new(k, false)));

    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn add_liquidity_by_strategy2_ix(
    pool: &PoolCtx,
    position: Pubkey,
    sender: Pubkey,
    user_token_x: Pubkey,
    user_token_y: Pubkey,
    amount_x: u64,
    amount_y: u64,
    active_id: i32,
    max_active_bin_slippage: i32,
    min_bin_id: i32,
    max_bin_id: i32,
    strategy_type: StrategyType,
    parameteres: [u8; 64],
    bin_arrays: Vec<Pubkey>,
    event_authority: Pubkey,
    program_id: Pubkey,
) -> Instruction {
    let mut data = ADD_LIQUIDITY_BY_STRATEGY2_DISC.to_vec();
    data.extend_from_slice(&amount_x.to_le_bytes());
    data.extend_from_slice(&amount_y.to_le_bytes());
    data.extend_from_slice(&active_id.to_le_bytes());
    data.extend_from_slice(&max_active_bin_slippage.to_le_bytes());
    data.extend_from_slice(&min_bin_id.to_le_bytes());
    data.extend_from_slice(&max_bin_id.to_le_bytes());
    data.push(strategy_type as u8);
    data.extend_from_slice(&parameteres);
    data.extend_from_slice(&spl_transfer_hook_slices());

    let mut accounts = vec![
        AccountMeta::new(position, false),
        AccountMeta::new(pool.lb_pair, false),
        AccountMeta::new(user_token_x, false),
        AccountMeta::new(user_token_y, false),
        AccountMeta::new(pool.reserve_x, false),
        AccountMeta::new(pool.reserve_y, false),
        AccountMeta::new_readonly(pool.token_x_mint, false),
        AccountMeta::new_readonly(pool.token_y_mint, false),
        AccountMeta::new_readonly(sender, true),
        AccountMeta::new_readonly(pool.token_program, false),
        AccountMeta::new_readonly(pool.token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];
    accounts.extend(bin_arrays.into_iter().map(|k| AccountMeta::new(k, false)));

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// AdvanceNonceAccount — must be the FIRST instruction of a durable-nonce tx.
pub fn advance_nonce_ix(nonce: Pubkey, authorized: Pubkey) -> Instruction {
    Instruction {
        program_id: SYSTEM_PROGRAM_ID.parse().unwrap(),
        accounts: vec![
            AccountMeta::new(nonce, false),
            AccountMeta::new_readonly(SYSVAR_RECENT_BLOCKHASHES.parse().unwrap(), false),
            AccountMeta::new_readonly(authorized, true),
        ],
        data: crate::nonce::encode_advance_nonce_data(),
    }
}

/// Full claim sequence: [AdvanceNonceAccount, claim_fee2].
pub fn build_claim_instructions(
    nonce: Pubkey,
    owner: Pubkey,
    pool: &PoolCtx,
    position: Pubkey,
    user_token_x: Pubkey,
    user_token_y: Pubkey,
    min_bin_id: i32,
    max_bin_id: i32,
    program_id: Pubkey,
) -> Vec<Instruction> {
    let event_authority = pda::event_authority(&program_id).0;
    let bin_arrays = bin_array_metas(&pool.lb_pair, min_bin_id, max_bin_id, &program_id)
        .into_iter()
        .map(|m| m.pubkey)
        .collect();

    vec![
        advance_nonce_ix(nonce, owner),
        claim_fee2_ix(
            pool,
            position,
            owner,
            user_token_x,
            user_token_y,
            min_bin_id,
            max_bin_id,
            bin_arrays,
            MEMO_PROGRAM_ID.parse().unwrap(),
            event_authority,
            program_id,
        ),
    ]
}

/// Full rebalance sequence: [AdvanceNonceAccount, remove_liquidity_by_range2,
/// add_liquidity_by_strategy2]. `bps_to_remove` is 10000 (100%) for a
/// full-range rebalance.
#[allow(clippy::too_many_arguments)]
pub fn build_rebalance_instructions(
    nonce: Pubkey,
    owner: Pubkey,
    pool: &PoolCtx,
    position: Pubkey,
    user_token_x: Pubkey,
    user_token_y: Pubkey,
    current_low: i32,
    current_high: i32,
    new_low: i32,
    new_high: i32,
    active_id: i32,
    amount_x: u64,
    amount_y: u64,
    max_active_bin_slippage: i32,
    strategy_type: StrategyType,
    parameteres: [u8; 64],
    program_id: Pubkey,
) -> Vec<Instruction> {
    let event_authority = pda::event_authority(&program_id).0;
    let memo = MEMO_PROGRAM_ID.parse().unwrap();

    // The SDK substitutes the program id as the dummy bitmap-extension account
    // when the pool has no extension. Indexes beyond |512| would need the real
    // extension — rebalance validation rejects those ranges (validate.rs).
    let remove_bitmap_ext = program_id;
    let remove_bin_arrays: Vec<Pubkey> =
        bin_array_metas(&pool.lb_pair, current_low, current_high, &program_id)
            .into_iter()
            .map(|m| m.pubkey)
            .collect();
    let add_bin_arrays: Vec<Pubkey> =
        bin_array_metas(&pool.lb_pair, new_low, new_high, &program_id)
            .into_iter()
            .map(|m| m.pubkey)
            .collect();

    vec![
        advance_nonce_ix(nonce, owner),
        remove_liquidity_by_range2_ix(
            pool,
            position,
            owner,
            user_token_x,
            user_token_y,
            current_low,
            current_high,
            10_000,
            remove_bitmap_ext,
            remove_bin_arrays,
            memo,
            event_authority,
            program_id,
        ),
        add_liquidity_by_strategy2_ix(
            pool,
            position,
            owner,
            user_token_x,
            user_token_y,
            amount_x,
            amount_y,
            active_id,
            max_active_bin_slippage,
            new_low,
            new_high,
            strategy_type,
            parameteres,
            add_bin_arrays,
            event_authority,
            program_id,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Legacy unsigned message encoding (web3.js-compatible)
// ---------------------------------------------------------------------------

fn shortvec(n: usize) -> Vec<u8> {
    // All our counts are < 128 → single byte.
    debug_assert!(n < 128);
    vec![n as u8]
}

/// web3.js `compileMessage` account-key sort: signers first, then writable,
/// then base58 string with English collation (`localeCompare('en', {sensitivity:
/// 'variant', caseFirst: 'lower', numeric: false})`) — digits before letters,
/// case-insensitive primary weight, lowercase before uppercase on a same-letter
/// tie. This is NOT a raw-byte sort: `a..Z` and `A..z` interleave differently.
fn base58_key_cmp(a: &Pubkey, b: &Pubkey) -> std::cmp::Ordering {
    let sa = bs58::encode(a.as_ref()).into_string();
    let sb = bs58::encode(b.as_ref()).into_string();
    for (ca, cb) in sa.bytes().zip(sb.bytes()) {
        let (la, lb) = (ca.to_ascii_lowercase(), cb.to_ascii_lowercase());
        match la.cmp(&lb) {
            std::cmp::Ordering::Equal => {
                // Same letter, different case → lowercase first (caseFirst).
                if ca != cb {
                    return if ca.is_ascii_lowercase() {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    };
                }
            }
            ord => return ord,
        }
    }
    sa.len().cmp(&sb.len())
}

/// Encode an unsigned legacy transaction (web3.js `Transaction.serialize`
/// with empty signatures). Reproduces web3.js v1.9x `compileMessage`
/// byte-for-byte:
///
///   * account metas collected in first-seen order, unique program ids
///     appended as readonly metas, duplicates culled (first wins, flags OR'd)
///   * sort: signers, then writable, then base58 collation (see
///     `base58_key_cmp`); fee payer moved to front and forced signer+writable
///   * one empty 64-byte signature placeholder per required signer
pub fn encode_unsigned_tx(
    instructions: &[Instruction],
    recent_blockhash: &[u8; 32],
    fee_payer: &Pubkey,
) -> Vec<u8> {
    // key, is_signer, is_writable
    let mut metas: Vec<(Pubkey, bool, bool)> = Vec::new();
    for ix in instructions {
        for meta in &ix.accounts {
            metas.push((meta.pubkey, meta.is_signer, meta.is_writable));
        }
        if !metas.iter().any(|(k, _, _)| k == &ix.program_id) {
            metas.push((ix.program_id, false, false));
        }
    }
    // Cull duplicate metas: first occurrence wins, flags OR'd.
    let mut unique: Vec<(Pubkey, bool, bool)> = Vec::new();
    for m in metas {
        if let Some(existing) = unique.iter_mut().find(|e| e.0 == m.0) {
            existing.1 |= m.1;
            existing.2 |= m.2;
        } else {
            unique.push(m);
        }
    }
    // Sort: signers first, then writable, then base58 collation.
    unique.sort_by(|x, y| {
        match (x.1, y.1) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        match (x.2, y.2) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        base58_key_cmp(&x.0, &y.0)
    });
    // Fee payer first, forced signer + writable.
    let payer_idx = unique.iter().position(|m| m.0 == *fee_payer);
    let payer = match payer_idx {
        Some(i) => unique.remove(i),
        None => (*fee_payer, false, false),
    };
    unique.insert(0, (payer.0, true, true));

    let (signed, unsigned): (Vec<_>, Vec<_>) = unique.into_iter().partition(|m| m.1);
    let num_required = signed.len();
    let num_readonly_signed = signed.iter().filter(|m| !m.2).count() as u8;
    let num_readonly_unsigned = unsigned.iter().filter(|m| !m.2).count() as u8;

    let mut account_keys: Vec<Pubkey> = signed.iter().map(|m| m.0).collect();
    account_keys.extend(unsigned.iter().map(|m| m.0));

    let mut out = Vec::new();
    // signatures: one empty placeholder per required signer
    out.extend_from_slice(&shortvec(num_required));
    for _ in 0..num_required {
        out.extend_from_slice(&[0u8; 64]);
    }
    // message header
    out.push(num_required as u8);
    out.push(num_readonly_signed);
    out.push(num_readonly_unsigned);
    // account keys
    out.extend_from_slice(&shortvec(account_keys.len()));
    for k in &account_keys {
        out.extend_from_slice(k.as_ref());
    }
    // recent blockhash
    out.extend_from_slice(recent_blockhash);
    // instructions
    out.extend_from_slice(&shortvec(instructions.len()));
    for ix in instructions {
        let pid_idx = account_keys
            .iter()
            .position(|k| k == &ix.program_id)
            .expect("program id in key list") as u8;
        out.push(pid_idx);
        out.extend_from_slice(&shortvec(ix.accounts.len()));
        for meta in &ix.accounts {
            let idx = account_keys
                .iter()
                .position(|k| k == &meta.pubkey)
                .expect("account in key list") as u8;
            out.push(idx);
        }
        out.extend_from_slice(&shortvec(ix.data.len()));
        out.extend_from_slice(&ix.data);
    }
    out
}
