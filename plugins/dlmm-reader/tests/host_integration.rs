//! Host-run tests for the dlmm-reader plugin's pure core.
//! No wasm toolchain required — `cargo test` from this directory is enough.

use dlmm_reader::decoder::{decode_position, shape, Status, Action};

fn fixture_data() -> Vec<u8> {
    // The borsh-encoded test fixture from the unit tests in src/decoder.rs.
    // Repeated here so the integration tests can use it without exporting
    // test-only symbols.
    use solana_pubkey::Pubkey;
    use borsh::BorshSerialize;

    const POSITION_DISCRIMINATOR: [u8; 8] = [170, 188, 143, 228, 122, 64, 173, 159];

    #[derive(BorshSerialize)]
    struct Pos {
        lb_pair: Pubkey,
        owner: Pubkey,
        lower_bin_id: i32,
        upper_bin_id: i32,
        last_updated_at: i64,
        total_x_amount: u64,
        total_y_amount: u64,
        fee_x_pending: u64,
        fee_y_pending: u64,
        reward_one_pending: u64,
        reward_two_pending: u64,
    }

    let mut v = POSITION_DISCRIMINATOR.to_vec();
    let pos = Pos {
        lb_pair: Pubkey::default(),
        owner: Pubkey::default(),
        lower_bin_id: 8450,
        upper_bin_id: 8520,
        last_updated_at: 0,
        total_x_amount: 1_500_000_000,
        total_y_amount: 200_000_000,
        fee_x_pending: 30_000_000,
        fee_y_pending: 5_000_000,
        reward_one_pending: 0,
        reward_two_pending: 0,
    };
    v.extend_from_slice(&borsh::to_vec(&pos).unwrap());
    v
}

#[test]
fn host_integration_end_to_end() {
    let data = fixture_data();
    let pos = decode_position(&data).expect("decode works on host");

    let summary = shape(
        &pos, 150.0, 8500, 1000.0, 1000.0, 0.5, 1.0,
        "SOL/USDC", "#4821",
    );

    // 1.5 SOL × $150 + 200 USDC × $1 = $225 + $200 = $425
    assert!((summary.value_usd - 425.0).abs() < 0.01);
    // 0.03 SOL × $150 + 5 USDC × $1 = $4.5 + $5 = $9.5 claimable
    assert!((summary.claimable_usd - 9.5).abs() < 0.01);
    // in range
    assert!(matches!(summary.status, Status::InRange));
    // IL = -57.5% (entry $1000 vs value $425) → Review, not Claim
    assert!(matches!(summary.action, Action::Review));
}
