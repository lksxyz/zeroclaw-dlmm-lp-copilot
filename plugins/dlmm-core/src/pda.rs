//! PDA derivation for the DLMM program — mirrors the SDK's
//! `deriveBinArray` / `deriveEventAuthority` / `deriveBinArrayBitmapExtension`
//! (seed order and encodings verified against the SDK's `findProgramAddressSync`
//! in tests/cross_check.rs).
//!
//! Implemented here instead of `solana-pubkey::find_program_address` (which
//! does not exist in the pure `solana-pubkey` crate): sha256 over
//! seeds || program_id || "ProgramDerivedAddress", reject all-zero or
//! on-curve results, bump 255→0.

use solana_pubkey::Pubkey;

use sha2::{Digest, Sha256};

const PDA_MARKER: &[u8] = b"ProgramDerivedAddress";

fn hash_seeds(seeds: &[&[u8]], program_id: &Pubkey) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for s in seeds {
        hasher.update(s);
    }
    hasher.update(program_id.as_ref());
    hasher.update(PDA_MARKER);
    hasher.finalize().into()
}

fn on_curve(bytes: &[u8; 32]) -> bool {
    // VerifyingKey::from_bytes fails when the point is not on the curve.
    ed25519_dalek::VerifyingKey::from_bytes(bytes).is_ok()
}

/// create_program_address with the same rejection rules as the runtime.
fn create_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Result<Pubkey, ()> {
    let hash = hash_seeds(seeds, program_id);
    if hash.iter().all(|&b| b == 0) || on_curve(&hash) {
        return Err(());
    }
    Ok(Pubkey::new_from_array(hash))
}

/// find_program_address: bump from 255 downwards.
fn find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    for bump in (0..=255u8).rev() {
        let mut with_bump: Vec<Vec<u8>> = seeds.iter().map(|s| s.to_vec()).collect();
        with_bump.push(vec![bump]);
        let seed_refs: Vec<&[u8]> = with_bump.iter().map(|v| v.as_slice()).collect();
        if let Ok(key) = create_program_address(&seed_refs, program_id) {
            return (key, bump);
        }
    }
    panic!("unable to find a valid program address");
}

/// `bin_array` PDA for a bin array index. Negative indexes are encoded as
/// two's-complement i64 LE (the SDK's `toTwos(64)`), which is identical to
/// `(index as i64).to_le_bytes()`.
pub fn bin_array(lb_pair: &Pubkey, index: i64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(
        &[b"bin_array", lb_pair.as_ref(), &index.to_le_bytes()],
        program_id,
    )
}

/// The program's event authority (`__event_authority`), used as the
/// `event_authority` account in every v3 instruction.
pub fn event_authority(program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[b"__event_authority"], program_id)
}

/// `bitmap` extension PDA. Passed as `bin_array_bitmap_extension` only when
/// the position's bin range overflows the default bitmap (|index| ≥ 512);
/// the SDK otherwise substitutes the program id as a dummy (optional-account
/// rules make that equivalent to omitting it).
pub fn bin_array_bitmap_extension(lb_pair: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[b"bitmap", lb_pair.as_ref()], program_id)
}

/// Associated-token-address PDA (seeds [owner, token_program, mint], on the
/// SPL ATA program).
pub fn associated_token_address(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &crate::tx::ASSOCIATED_TOKEN_PROGRAM_ID.parse().unwrap(),
    )
    .0
}
