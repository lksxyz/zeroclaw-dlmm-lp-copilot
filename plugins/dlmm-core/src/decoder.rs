//! On-chain account decoders — Meteora DLMM program (devnet
//! LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo, mainnet
//! LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSK9q8Mfev5Rq).
//!
//! Layouts are bytemuck C-repr (per the current program IDL) — NOT borsh.
//! A hand-rolled cursor reads the fixed offsets; every offset is verified
//! against Anchor-coder fixtures in tests/cross_check.rs.
//!
//! Account sizes (8-byte discriminator included):
//!   PositionV2 = 8120 bytes, LbPair = 888 bytes, BinArray = 10136 bytes.

use solana_pubkey::Pubkey;

pub const POSITION_V2_DISCRIMINATOR: [u8; 8] = [117, 176, 212, 199, 245, 180, 133, 182];
pub const LB_PAIR_DISCRIMINATOR: [u8; 8] = [33, 11, 49, 98, 181, 101, 177, 13];
pub const BIN_ARRAY_DISCRIMINATOR: [u8; 8] = [92, 142, 92, 220, 5, 148, 70, 181];

pub const POSITION_V2_LEN: usize = 8120;
pub const LB_PAIR_LEN: usize = 888;
pub const BIN_ARRAY_LEN: usize = 10136;

const BIN_COUNT: usize = 70;

#[derive(Debug, Clone, Copy)]
pub enum DecodeError {
    TooShort,
    BadDiscriminator,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "account data too short"),
            Self::BadDiscriminator => write!(f, "not a DLMM account of the expected type"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Little-endian cursor over account bytes.
struct Rd<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn new(d: &'a [u8]) -> Self {
        Self { d, p: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.p + n > self.d.len() {
            return Err(DecodeError::TooShort);
        }
        let s = &self.d[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, DecodeError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn u128(&mut self) -> Result<u128, DecodeError> {
        Ok(u128::from_le_bytes(self.take(16)?.try_into().unwrap()))
    }
    fn pubkey(&mut self) -> Result<Pubkey, DecodeError> {
        Ok(Pubkey::try_from(self.take(32)?).map_err(|_| DecodeError::TooShort)?)
    }
    fn skip(&mut self, n: usize) -> Result<(), DecodeError> {
        self.take(n).map(|_| ())
    }
}

// ---------------------------------------------------------------------------
// PositionV2
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct UserRewardInfo {
    pub reward_per_token_completes: [u128; 2],
    pub reward_pendings: [u64; 2],
}

#[derive(Debug, Clone, Copy)]
pub struct FeeInfo {
    pub fee_x_per_token_complete: u128,
    pub fee_y_per_token_complete: u128,
    pub fee_x_pending: u64,
    pub fee_y_pending: u64,
}

/// PositionV2 — bytemuck C-repr. See IDL `PositionV2` for the field order.
#[derive(Debug, Clone)]
pub struct PositionV2 {
    pub lb_pair: Pubkey,
    pub owner: Pubkey,
    pub liquidity_shares: [u128; BIN_COUNT],
    pub reward_infos: [UserRewardInfo; BIN_COUNT],
    pub fee_infos: [FeeInfo; BIN_COUNT],
    pub lower_bin_id: i32,
    pub upper_bin_id: i32,
    pub last_updated_at: i64,
    pub total_claimed_fee_x_amount: u64,
    pub total_claimed_fee_y_amount: u64,
    pub total_claimed_rewards: [u64; 2],
    pub operator: Pubkey,
    pub lock_release_point: u64,
    pub _padding_0: u8,
    pub fee_owner: Pubkey,
    pub version: u8,
    pub permissionless_operation_bits: u8,
    pub _reserved: [u8; 85],
}

pub fn decode_position(data: &[u8]) -> Result<PositionV2, DecodeError> {
    if data.len() < POSITION_V2_LEN {
        return Err(DecodeError::TooShort);
    }
    if data[..8] != POSITION_V2_DISCRIMINATOR {
        return Err(DecodeError::BadDiscriminator);
    }
    let mut r = Rd::new(&data[8..]);

    let lb_pair = r.pubkey()?;
    let owner = r.pubkey()?;

    let mut liquidity_shares = [0u128; BIN_COUNT];
    for s in liquidity_shares.iter_mut() {
        *s = r.u128()?;
    }

    let mut reward_infos = [UserRewardInfo {
        reward_per_token_completes: [0; 2],
        reward_pendings: [0; 2],
    }; BIN_COUNT];
    for info in reward_infos.iter_mut() {
        info.reward_per_token_completes[0] = r.u128()?;
        info.reward_per_token_completes[1] = r.u128()?;
        info.reward_pendings[0] = r.u64()?;
        info.reward_pendings[1] = r.u64()?;
    }

    let mut fee_infos = [FeeInfo {
        fee_x_per_token_complete: 0,
        fee_y_per_token_complete: 0,
        fee_x_pending: 0,
        fee_y_pending: 0,
    }; BIN_COUNT];
    for info in fee_infos.iter_mut() {
        info.fee_x_per_token_complete = r.u128()?;
        info.fee_y_per_token_complete = r.u128()?;
        info.fee_x_pending = r.u64()?;
        info.fee_y_pending = r.u64()?;
    }

    let lower_bin_id = r.i32()?;
    let upper_bin_id = r.i32()?;
    let last_updated_at = r.i64()?;
    let total_claimed_fee_x_amount = r.u64()?;
    let total_claimed_fee_y_amount = r.u64()?;
    let total_claimed_rewards = [r.u64()?, r.u64()?];
    let operator = r.pubkey()?;
    let lock_release_point = r.u64()?;
    let _padding_0 = r.u8()?;
    let fee_owner = r.pubkey()?;
    let version = r.u8()?;
    let permissionless_operation_bits = r.u8()?;
    let mut _reserved = [0u8; 85];
    _reserved.copy_from_slice(r.take(85)?);

    Ok(PositionV2 {
        lb_pair,
        owner,
        liquidity_shares,
        reward_infos,
        fee_infos,
        lower_bin_id,
        upper_bin_id,
        last_updated_at,
        total_claimed_fee_x_amount,
        total_claimed_fee_y_amount,
        total_claimed_rewards,
        operator,
        lock_release_point,
        _padding_0,
        fee_owner,
        version,
        permissionless_operation_bits,
        _reserved,
    })
}

/// True when the position's effective fee recipient is the wallet address
/// (`fee_owner` when set, else `owner`) — mirrors the SDK's
/// `walletToReceiveFee = feeOwner.isZero ? owner : feeOwner`.
pub fn fee_recipient(position: &PositionV2) -> Pubkey {
    if position.fee_owner == Pubkey::default() {
        position.owner
    } else {
        position.fee_owner
    }
}

/// Claimable fee totals across all bins, in raw token units.
pub fn claimable_fees(position: &PositionV2) -> (u64, u64) {
    let (mut x, mut y) = (0u64, 0u64);
    for info in &position.fee_infos {
        x = x.saturating_add(info.fee_x_pending);
        y = y.saturating_add(info.fee_y_pending);
    }
    (x, y)
}

/// Sum of liquidity shares (raw). Used as a quick "position is empty" check
/// for the fail-closed claim guard.
pub fn total_liquidity_shares(position: &PositionV2) -> u128 {
    position.liquidity_shares.iter().sum()
}

/// Per-bin liquidity share, indexed by `bin_id - lower_bin_id`.
pub fn share_at(position: &PositionV2, bin_id: i32) -> u128 {
    let idx = bin_id - position.lower_bin_id;
    if idx < 0 || idx as usize >= BIN_COUNT {
        return 0;
    }
    position.liquidity_shares[idx as usize]
}

/// Position token amounts, derived from liquidity shares × bin reserves.
///
/// PositionV2 stores per-bin liquidity **shares**, not token totals — the
/// X/Y amount in a bin is `share / bin.liquidity_supply × bin.amount`.
/// Used by the rebalance builder to re-deposit "what was just removed".
pub fn position_amounts(
    position: &PositionV2,
    bin_arrays: &[BinArray],
) -> (u64, u64) {
    let find = |idx: i64| bin_arrays.iter().find(|ba| ba.index == idx);
    let (mut x, mut y) = (0u128, 0u128);
    for bin_id in position.lower_bin_id..=position.upper_bin_id {
        let arr_idx = crate::tx::bin_id_to_bin_array_index(bin_id);
        let Some(ba) = find(arr_idx) else { continue };
        let inner = (bin_id as i64 - arr_idx * BINS_PER_ARRAY) as usize;
        if inner >= BIN_COUNT {
            continue;
        }
        let bin = &ba.bins[inner];
        let share = share_at(position, bin_id);
        if share == 0 || bin.liquidity_supply == 0 {
            continue;
        }
        x += (share * bin.amount_x as u128) / bin.liquidity_supply;
        y += (share * bin.amount_y as u128) / bin.liquidity_supply;
    }
    (x.min(u64::MAX as u128) as u64, y.min(u64::MAX as u128) as u64)
}

const BINS_PER_ARRAY: i64 = 70;

// ---------------------------------------------------------------------------
// LbPair
// ---------------------------------------------------------------------------

/// The subset of LbPair we need for reporting and tx building.
#[derive(Debug, Clone)]
pub struct LbPair {
    pub active_id: i32,
    pub bin_step: u16,
    pub token_x_mint: Pubkey,
    pub token_y_mint: Pubkey,
    pub reserve_x: Pubkey,
    pub reserve_y: Pubkey,
    pub oracle: Pubkey,
}

pub fn decode_lb_pair(data: &[u8]) -> Result<LbPair, DecodeError> {
    if data.len() < LB_PAIR_LEN {
        return Err(DecodeError::TooShort);
    }
    if data[..8] != LB_PAIR_DISCRIMINATOR {
        return Err(DecodeError::BadDiscriminator);
    }
    let mut r = Rd::new(&data[8..]);

    // StaticParameters (32) + VariableParameters (32) — parsed fields only.
    let mut static_len = 0usize;
    for _ in 0..4 {
        r.u16()?; // base_factor, filter_period, decay_period, reduction_factor
        static_len += 2;
    }
    r.u32()?; // variable_fee_control
    r.u32()?; // max_volatility_accumulator
    r.i32()?; // min_bin_id
    r.i32()?; // max_bin_id
    r.u16()?; // protocol_share
    r.u8()?; // base_fee_power_factor
    r.u8()?; // function_type
    r.u8()?; // collect_fee_mode
    r.skip(3)?; // _padding
    static_len += 4 + 4 + 4 + 4 + 2 + 1 + 1 + 1 + 3;
    debug_assert_eq!(static_len, 32);

    r.u32()?; // volatility_accumulator
    r.u32()?; // volatility_reference
    r.i32()?; // index_reference
    r.skip(4)?; // _padding
    r.i64()?; // last_update_timestamp
    r.skip(8)?; // _padding_1

    r.skip(1)?; // bump_seed
    r.skip(2)?; // bin_step_seed
    r.u8()?; // pair_type
    let active_id = r.i32()?;
    let bin_step = r.u16()?;
    r.u8()?; // status
    r.u8()?; // require_base_factor_seed
    r.skip(2)?; // base_factor_seed
    r.u8()?; // activation_type
    r.u8()?; // creator_pool_on_off_control

    let token_x_mint = r.pubkey()?;
    let token_y_mint = r.pubkey()?;
    let reserve_x = r.pubkey()?;
    let reserve_y = r.pubkey()?;

    r.skip(16)?; // protocol_fee
    r.skip(32)?; // _padding_1
    r.skip(2 * 136)?; // reward_infos [RewardInfo; 2] — 136 bytes each
    let oracle = r.pubkey()?;

    Ok(LbPair {
        active_id,
        bin_step,
        token_x_mint,
        token_y_mint,
        reserve_x,
        reserve_y,
        oracle,
    })
}

// ---------------------------------------------------------------------------
// BinArray
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct Bin {
    pub amount_x: u64,
    pub amount_y: u64,
    pub price: u128,
    pub liquidity_supply: u128,
}

#[derive(Debug, Clone)]
pub struct BinArray {
    pub index: i64,
    pub lb_pair: Pubkey,
    pub bins: [Bin; BIN_COUNT],
}

pub fn decode_bin_array(data: &[u8]) -> Result<BinArray, DecodeError> {
    if data.len() < BIN_ARRAY_LEN {
        return Err(DecodeError::TooShort);
    }
    if data[..8] != BIN_ARRAY_DISCRIMINATOR {
        return Err(DecodeError::BadDiscriminator);
    }
    let mut r = Rd::new(&data[8..]);

    let index = r.i64()?;
    r.u8()?; // version
    r.skip(7)?; // _padding_1
    let lb_pair = r.pubkey()?;

    let mut bins = [Bin {
        amount_x: 0,
        amount_y: 0,
        price: 0,
        liquidity_supply: 0,
    }; BIN_COUNT];
    for bin in bins.iter_mut() {
        bin.amount_x = r.u64()?;
        bin.amount_y = r.u64()?;
        bin.price = r.u128()?;
        bin.liquidity_supply = r.u128()?;
        r.skip(144 - 8 - 8 - 16 - 16)?; // remaining Bin fields + padding
    }

    Ok(BinArray {
        index,
        lb_pair,
        bins,
    })
}
