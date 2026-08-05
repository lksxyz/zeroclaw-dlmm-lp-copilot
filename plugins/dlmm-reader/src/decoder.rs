//! Pure core: decode Meteora DLMM Position accounts and compute
//! a shaped, ~200-token-per-position summary.
//!
//! No wasm imports. Host-testable with `cargo test`.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::Serialize;
use solana_pubkey::Pubkey;
use solana_instruction::AccountMeta;

const POSITION_DISCRIMINATOR: [u8; 8] = [170, 188, 143, 228, 122, 64, 173, 159];

/// On-chain layout of a Meteora DLMM Position account.
/// Only the fields we need to render the summary are deserialized.
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct PositionAccount {
    pub lb_pair: Pubkey,                  // 32
    pub owner: Pubkey,                    // 32
    pub lower_bin_id: i32,                // 4
    pub upper_bin_id: i32,                // 4
    pub last_updated_at: i64,             // 8
    pub total_x_amount: u64,              // 8
    pub total_y_amount: u64,              // 8
    pub fee_x_pending: u64,               // 8
    pub fee_y_pending: u64,               // 8
    pub reward_one_pending: u64,          // 8
    pub reward_two_pending: u64,          // 8
    // ... the on-chain struct has more fields we don't need
}

#[derive(Debug, Clone)]
pub enum DecodeError {
    TooShort,
    BadDiscriminator,
    Borsh(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "account data too short"),
            Self::BadDiscriminator => write!(f, "not a DLMM Position account"),
            Self::Borsh(s) => write!(f, "borsh: {s}"),
        }
    }
}

impl std::error::Error for DecodeError {}

pub fn decode_position(data: &[u8]) -> Result<PositionAccount, DecodeError> {
    if data.len() < 8 {
        return Err(DecodeError::TooShort);
    }
    if data[..8] != POSITION_DISCRIMINATOR {
        return Err(DecodeError::BadDiscriminator);
    }
    PositionAccount::try_from_slice(&data[8..])
        .map_err(|e| DecodeError::Borsh(e.to_string()))
}

/// Per-position summary, shaped to fit the LLM's context window
/// without flooding it.
#[derive(Serialize, Debug, Clone)]
pub struct PositionSummary {
    pub id: String,
    pub pair: String,
    pub status: Status,
    pub range_bins: (i32, i32),
    pub value_usd: f64,
    pub fees_24h_usd: f64,
    pub fees_7d_usd: f64,
    pub claimable_usd: f64,
    pub il_pct: f64,
    pub action: Action,
}

#[derive(Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    InRange,
    OutOfRange,
    StalePrice,
    StalePool,
}

#[derive(Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Hold,
    Rebalance,
    Claim,
    Review,
}

/// Pure compute. Caller supplies the price (Switchboard), the active bin
/// (Meteora pool state), and the entry-time token amounts (cached to
/// memory on first read).
pub fn shape(
    position: &PositionAccount,
    sol_usd: f64,
    active_bin_id: i32,
    entry_value_usd: f64,
    hodl_value_usd: f64,
    fees_24h_usd: f64,
    fees_7d_usd: f64,
    pair_label: &str,
    id_label: &str,
) -> PositionSummary {
    let in_range = active_bin_id >= position.lower_bin_id
        && active_bin_id <= position.upper_bin_id;

    let value_usd =
        (position.total_x_amount as f64 / 1e9) * sol_usd
        + (position.total_y_amount as f64 / 1e6) * 1.0; // USDC: 6 decimals

    // Claimable fees: assume fee_x is in X (e.g. SOL) and fee_y in Y (USDC).
    let claimable_usd =
        (position.fee_x_pending as f64 / 1e9) * sol_usd
        + (position.fee_y_pending as f64 / 1e6) * 1.0;

    // IL vs HODL: negative when position is worse than holding the
    // underlying at the original ratio.
    let il_pct = if entry_value_usd > 0.0 {
        ((value_usd - hodl_value_usd) / entry_value_usd) * 100.0
    } else {
        0.0
    };

    let action = if !in_range {
        Action::Rebalance
    } else if il_pct < -5.0 {
        Action::Review
    } else if claimable_usd >= 1.0 {
        Action::Claim
    } else {
        Action::Hold
    };

    PositionSummary {
        id: id_label.to_string(),
        pair: pair_label.to_string(),
        status: if in_range { Status::InRange } else { Status::OutOfRange },
        range_bins: (position.lower_bin_id, position.upper_bin_id),
        value_usd,
        fees_24h_usd,
        fees_7d_usd,
        claimable_usd,
        il_pct,
        action,
    }
}

/// Build the remaining-account list for the DLMM `claimFee` instruction.
/// Kept here so the worker (TypeScript) can mirror the same layout.
pub fn claim_fee_accounts(
    position: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
) -> Vec<AccountMeta> {
    // The Meteora SDK's claimFee places these in a specific order;
    // we mirror it so the worker's encoding matches the on-chain program.
    vec![
        AccountMeta::new(*position, false),
        AccountMeta::new(*pool, false),
        AccountMeta::new(*user, true),  // signer
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_data(lower: i32, upper: i32) -> Vec<u8> {
        let mut v = POSITION_DISCRIMINATOR.to_vec();
        let pos = PositionAccount {
            lb_pair: Pubkey::default(),
            owner: Pubkey::default(),
            lower_bin_id: lower,
            upper_bin_id: upper,
            last_updated_at: 0,
            total_x_amount: 1_500_000_000,  // 1.5 SOL
            total_y_amount: 200_000_000,    // 200 USDC
            fee_x_pending: 30_000_000,      // 0.03 SOL
            fee_y_pending: 5_000_000,       // 5 USDC
            reward_one_pending: 0,
            reward_two_pending: 0,
        };
        v.extend_from_slice(&borsh::to_vec(&pos).unwrap());
        v
    }

    #[test]
    fn decodes_well_formed_account() {
        let data = fixture_data(8450, 8520);
        let pos = decode_position(&data).expect("decode");
        assert_eq!(pos.lower_bin_id, 8450);
        assert_eq!(pos.upper_bin_id, 8520);
        assert_eq!(pos.total_x_amount, 1_500_000_000);
    }

    #[test]
    fn rejects_wrong_discriminator() {
        let mut data = vec![0u8; 8 + 64];
        data[..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(matches!(decode_position(&data), Err(DecodeError::BadDiscriminator)));
    }

    #[test]
    fn rejects_short_data() {
        assert!(matches!(decode_position(&[0; 4]), Err(DecodeError::TooShort)));
    }

    #[test]
    fn shapes_in_range_with_high_il_triggers_review() {
        // IL = -57.5% (value $425 vs entry $1000) → review
        let data = fixture_data(8450, 8520);
        let pos = decode_position(&data).unwrap();
        let s = shape(
            &pos,
            150.0,         // SOL/USD
            8500,          // active bin (in range)
            1000.0,        // entry value
            1000.0,        // HODL
            0.5, 1.0,
            "SOL/USDC", "#4821",
        );
        assert!(matches!(s.status, Status::InRange));
        assert!(matches!(s.action, Action::Review));
    }

    #[test]
    fn shapes_in_range_with_low_il_and_claimable_fees_triggers_claim() {
        // Position value ≈ entry value (negligible IL), fees ≥ $1
        let data = fixture_data(8450, 8520);
        let pos = decode_position(&data).unwrap();
        let s = shape(
            &pos,
            150.0,         // SOL/USD
            8500,          // active bin (in range)
            425.0,         // entry ≈ current value
            425.0,         // HODL ≈ current value
            0.5, 1.0,
            "SOL/USDC", "#4821",
        );
        assert!(matches!(s.status, Status::InRange));
        assert!(matches!(s.action, Action::Claim)); // claimable $9.5 ≥ $1
    }

    #[test]
    fn shapes_out_of_range_position_to_rebalance() {
        let data = fixture_data(8450, 8520);
        let pos = decode_position(&data).unwrap();
        let s = shape(
            &pos, 150.0, 8621, 1000.0, 1000.0, 0.0, 0.0,
            "JUP/USDC", "#4822",
        );
        assert!(matches!(s.status, Status::OutOfRange));
        assert!(matches!(s.action, Action::Rebalance));
    }
}
