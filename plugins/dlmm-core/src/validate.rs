//! Mechanical validation — "the LLM proposes, the plugin verifies."
//! Every rule fails closed: any violation → the builder refuses to emit
//! a transaction.

use solana_pubkey::Pubkey;

use crate::decoder::{
    claimable_fees_with_bins, fee_recipient, BinArray, PositionV2,
};

pub const MAX_ACTIVE_BIN_SLIPPAGE: i32 = 3;
pub const BPS_FULL: u16 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Position owner / fee recipient must be the operator's own wallet.
    NotOwned,
    /// Range bounds must be strictly increasing.
    InvalidRange,
    /// The new range must contain the pool's active bin (else the rebalance
    /// would immediately be out of range).
    RangeDoesNotContainActiveBin,
    /// Active-bin slippage guard: |active_id_observed - active_id| must be
    /// within MAX_ACTIVE_BIN_SLIPPAGE.
    ActiveBinTooFar(i32),
    /// Fail-closed claim: nothing claimable (fees + rewards all zero).
    NothingClaimable,
    /// Position has no liquidity at all — claim would be a no-op at best.
    EmptyPosition,
    /// Bin-array range must stay inside the default bitmap (|index| < 512) —
    /// outside it requires the bitmap-extension account we don't build.
    BinArrayOverflow(i64),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotOwned => write!(f, "position owner/fee-recipient mismatch"),
            Self::InvalidRange => write!(f, "range must satisfy low < high"),
            Self::RangeDoesNotContainActiveBin => {
                write!(f, "range must contain the pool's active bin")
            }
            Self::ActiveBinTooFar(d) => {
                write!(
                    f,
                    "active bin moved too far (delta {d} > {MAX_ACTIVE_BIN_SLIPPAGE})"
                )
            }
            Self::NothingClaimable => write!(f, "nothing claimable (fees and rewards are zero)"),
            Self::EmptyPosition => write!(f, "position holds no liquidity"),
            Self::BinArrayOverflow(idx) => {
                write!(
                    f,
                    "bin-array index {idx} outside default bitmap (|index| >= 512)"
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Ownership gate for any position the agent is asked to touch.
pub fn validate_owned(position: &PositionV2, owner: &Pubkey) -> Result<(), ValidationError> {
    if position.owner != *owner || fee_recipient(position) != *owner {
        return Err(ValidationError::NotOwned);
    }
    Ok(())
}

/// Range sanity + active-bin containment for a rebalance proposal.
pub fn validate_range(new_low: i32, new_high: i32, active_bin: i32) -> Result<(), ValidationError> {
    if new_low >= new_high {
        return Err(ValidationError::InvalidRange);
    }
    if active_bin < new_low || active_bin > new_high {
        return Err(ValidationError::RangeDoesNotContainActiveBin);
    }
    Ok(())
}

/// The observed active id must be close to the one embedded in the liquidity
/// parameters (defense against stale reads racing a swap).
pub fn validate_active_bin_slippage(observed: i32, embedded: i32) -> Result<(), ValidationError> {
    let d = (observed - embedded).abs();
    if d > MAX_ACTIVE_BIN_SLIPPAGE {
        return Err(ValidationError::ActiveBinTooFar(d));
    }
    Ok(())
}

/// Fail-closed claim guard: refuse to build a claim tx when nothing is
/// claimable. Uses the per-bin per-token math (`claimable_fees_with_bins`),
/// which is the only accurate measure — a raw `fee_x_pending` sum undercounts
/// fees accrued since the last claim. Rewards are the pending amounts in the
/// two reward slots.
pub fn validate_claimable(
    position: &PositionV2,
    bin_arrays: &[BinArray],
) -> Result<(), ValidationError> {
    if crate::decoder::total_liquidity_shares(position) == 0 {
        return Err(ValidationError::EmptyPosition);
    }
    let (fee_x, fee_y) = claimable_fees_with_bins(position, bin_arrays);
    let reward_pending = position.reward_infos.iter().fold(0u64, |acc, r| {
        acc.saturating_add(r.reward_pendings[0] + r.reward_pendings[1])
    });
    if fee_x == 0 && fee_y == 0 && reward_pending == 0 {
        return Err(ValidationError::NothingClaimable);
    }
    Ok(())
}

/// Keep bin-array indexes inside the default bitmap (|index| < 512): the
/// builder only emits the dummy bitmap-extension account, so ranges outside
/// it would reference arrays the program can't reach without the extension.
pub fn validate_bin_array_range(min_bin_id: i32, max_bin_id: i32) -> Result<(), ValidationError> {
    for idx in crate::tx::bin_array_indexes_for_range(min_bin_id, max_bin_id) {
        if idx.abs() >= 512 {
            return Err(ValidationError::BinArrayOverflow(idx));
        }
    }
    Ok(())
}
