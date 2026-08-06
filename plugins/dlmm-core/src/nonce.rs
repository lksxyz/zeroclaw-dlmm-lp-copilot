//! Durable-nonce helpers (System Program).
//!
//! Nonce account layout: version u32, state u32, authority 32B, stored
//! blockhash 32B **@ offset 40**, fee_calculator u64. The stored hash is what
//! becomes the transaction's `recentBlockhash`; `AdvanceNonceAccount` must be
//! the FIRST instruction of a durable-nonce transaction.

pub const ADVANCE_NONCE_TAG: u32 = 4;
pub const BLOCKHASH_OFFSET: usize = 40;
pub const BLOCKHASH_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceError {
    TooShort,
}

impl std::fmt::Display for NonceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "nonce account data too short"),
        }
    }
}

impl std::error::Error for NonceError {}

/// Serialized data for `AdvanceNonceAccount`: little-endian u32 tag, no args.
/// Must be the FIRST instruction in a durable-nonce transaction.
pub fn encode_advance_nonce_data() -> Vec<u8> {
    ADVANCE_NONCE_TAG.to_le_bytes().to_vec()
}

/// Extract the stored nonce hash from a nonce account's data (32 bytes).
pub fn decode_nonce_hash(data: &[u8]) -> Result<[u8; BLOCKHASH_LEN], NonceError> {
    if data.len() < BLOCKHASH_OFFSET + BLOCKHASH_LEN {
        return Err(NonceError::TooShort);
    }
    Ok(data[BLOCKHASH_OFFSET..BLOCKHASH_OFFSET + BLOCKHASH_LEN]
        .try_into()
        .unwrap())
}

/// Stored nonce hash as base58 — the format `Transaction.recentBlockhash`
/// expects.
pub fn nonce_hash_base58(data: &[u8]) -> Result<String, NonceError> {
    decode_nonce_hash(data).map(|h| bs58::encode(h).into_string())
}

/// Settlement evidence: the nonce account's stored hash changed since the
/// ledger recorded it → the pending transaction landed (or was replaced),
/// so a new proposal is safe.
pub fn is_settled(previous: &[u8; BLOCKHASH_LEN], current: &[u8; BLOCKHASH_LEN]) -> bool {
    previous != current
}
