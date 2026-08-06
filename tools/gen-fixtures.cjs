// Ground-truth fixture generator v2 — INDEPENDENT sources:
//  1. @solana/web3.js — Solana wire format (Transaction serialize, SystemProgram)
//  2. @meteora-ag/dlmm exported helpers — PDA derivation (deriveBinArray, ...)
//  3. IDL spec (extracted) — DLMM instruction data + account lists, encoded by hand
//  4. bytemuck C-repr layouts — PositionV2 / LbPair / BinArray account bytes
// Pure, no RPC. Rust core (plugins/dlmm-core) cross-checks byte-for-byte.
const fs = require("node:fs");
const crypto = require("node:crypto");
const bs58 = require("bs58");
const { web3 } = require("@coral-xyz/anchor");
const { deriveBinArray, deriveEventAuthority, deriveBinArrayBitmapExtension } = require("@meteora-ag/dlmm");

const PROGRAM = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo"; // DLMM devnet
// deterministic valid pubkeys: sha256(seed) — stable across runs
const keyFor = (seed) => new web3.PublicKey(crypto.createHash("sha256").update(seed).digest()).toBase58();
const POOL = keyFor("fixture.pool");
const POSITION = keyFor("fixture.position");
const OWNER = keyFor("fixture.owner");
const NONCE = keyFor("fixture.nonce");
const TOKEN_X_MINT = "So11111111111111111111111111111111111111112";
const TOKEN_Y_MINT = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"; // USDC devnet
const RESERVE_X = keyFor("fixture.reserve_x");
const RESERVE_Y = keyFor("fixture.reserve_y");
const TOKEN_PROGRAM = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const MEMO_PROGRAM = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const SYSVAR_RECENT_BLOCKHASHES = "SysvarRecentB1ockHashes11111111111111111111";
// Real ATA PDAs (web3.js findProgramAddress ground truth for the Rust ATA
// derivation): user_token_x/y = ATA(OWNER, mint) on the SPL ATA program.
const ATA_PROGRAM = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const ata = (owner, mint) =>
  web3.PublicKey.findProgramAddressSync(
    [new web3.PublicKey(owner).toBuffer(), new web3.PublicKey(TOKEN_PROGRAM).toBuffer(), new web3.PublicKey(mint).toBuffer()],
    new web3.PublicKey(ATA_PROGRAM),
  )[0].toBase58();
const USER_TOKEN_X = ata(OWNER, TOKEN_X_MINT);
const USER_TOKEN_Y = ata(OWNER, TOKEN_Y_MINT);

const p = (s) => new web3.PublicKey(s);
const ZERO = web3.PublicKey.default;
const eventAuthority = deriveEventAuthority(p(PROGRAM))[0].toBase58();

// ---- minimal encoders (IDL spec: i32/i16/u16/u64 LE, borsh vec, u8 enums) ----
const u8 = (n) => Buffer.from([n]);
const u32le = (n) => { const b = Buffer.alloc(4); b.writeUInt32LE(n >>> 0); return b; };
const i32le = (n) => { const b = Buffer.alloc(4); b.writeInt32LE(n); return b; };
const u16le = (n) => { const b = Buffer.alloc(2); b.writeUInt16LE(n); return b; };
const u64le = (n) => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; };
const disc = (name) => crypto.createHash("sha256").update("global:" + name).digest().subarray(0, 8);
// RemainingAccountsInfo: vec<RemainingAccountsSlice{accounts_type: u8, length: u8}>
const rai = (slices) => Buffer.concat([u32le(slices.length), ...slices.map(([tag, len]) => Buffer.from([tag, len]))]);
// SPL pools: SDK emits two zero-length transfer-hook slices
const SPL_SLICES = [[0, 0], [1, 0]]; // transferHookX=0, transferHookY=1

const { BN } = require("@coral-xyz/anchor");
const claimBinKeys = [deriveBinArray(p(POOL), new BN(120), p(PROGRAM))[0], deriveBinArray(p(POOL), new BN(121), p(PROGRAM))[0]];
const binMetas = claimBinKeys.map((k) => ({ pubkey: k, isSigner: false, isWritable: true }));
const meta = (key, w, s) => ({ pubkey: p(key), isSigner: !!s, isWritable: !!w });

const out = {};

// ---------- 1. claim_fee2 ----------
const claimData = Buffer.concat([
  disc("claim_fee2"), i32le(8450), i32le(8520), rai(SPL_SLICES),
]);
const claimKeys = [
  meta(POOL, true), meta(POSITION, true), meta(OWNER, false, true),
  meta(RESERVE_X, true), meta(RESERVE_Y, true),
  meta(USER_TOKEN_X, true), meta(USER_TOKEN_Y, true),
  meta(TOKEN_X_MINT, false), meta(TOKEN_Y_MINT, false),
  meta(TOKEN_PROGRAM, false), meta(TOKEN_PROGRAM, false), meta(MEMO_PROGRAM, false),
  meta(eventAuthority, false), meta(PROGRAM, false),
  ...binMetas,
];
out.claim_ix = { program_id: PROGRAM, keys: claimKeys.map((k) => ({ pubkey: k.pubkey.toBase58(), isSigner: k.isSigner, isWritable: k.isWritable })), data: claimData.toString("base64") };

// ---------- 2. remove_liquidity_by_range2 (bps=10000; bitmap ext = programId dummy) ----------
const removeData = Buffer.concat([
  disc("remove_liquidity_by_range2"), i32le(8450), i32le(8520), u16le(10000), rai(SPL_SLICES),
]);
const removeKeys = [
  meta(POSITION, true), meta(POOL, true), meta(PROGRAM, true), // bitmap ext: programId when pool has none
  meta(USER_TOKEN_X, true), meta(USER_TOKEN_Y, true),
  meta(RESERVE_X, true), meta(RESERVE_Y, true),
  meta(TOKEN_X_MINT, false), meta(TOKEN_Y_MINT, false),
  meta(OWNER, false, true), meta(TOKEN_PROGRAM, false), meta(TOKEN_PROGRAM, false),
  meta(MEMO_PROGRAM, false), meta(eventAuthority, false), meta(PROGRAM, false),
  ...binMetas,
];
out.remove_ix = { program_id: PROGRAM, keys: removeKeys.map((k) => ({ pubkey: k.pubkey.toBase58(), isSigner: k.isSigner, isWritable: k.isWritable })), data: removeData.toString("base64") };

// ---------- 3. add_liquidity_by_strategy2 (SpotImBalanced=6, no overflow → no bitmap ext, no memo) ----------
const addData = Buffer.concat([
  disc("add_liquidity_by_strategy2"),
  u64le(1500000000), u64le(200000000), // amount_x, amount_y
  i32le(8500), i32le(3),                // active_id, max_active_bin_slippage
  i32le(8450), i32le(8520),             // min_bin_id, max_bin_id
  u8(6),                                // StrategyType::SpotImBalanced
  Buffer.alloc(64),                     // parameteres (favorSide = None → 0)
  rai(SPL_SLICES),
]);
const addKeys = [
  meta(POSITION, true), meta(POOL, true),
  meta(USER_TOKEN_X, true), meta(USER_TOKEN_Y, true),
  meta(RESERVE_X, true), meta(RESERVE_Y, true),
  meta(TOKEN_X_MINT, false), meta(TOKEN_Y_MINT, false),
  meta(OWNER, false, true), meta(TOKEN_PROGRAM, false), meta(TOKEN_PROGRAM, false),
  meta(eventAuthority, false), meta(PROGRAM, false),
  ...binMetas,
];
out.add_ix = { program_id: PROGRAM, keys: addKeys.map((k) => ({ pubkey: k.pubkey.toBase58(), isSigner: k.isSigner, isWritable: k.isWritable })), data: addData.toString("base64") };

// ---------- 4. advance_nonce_account (web3.js SystemProgram — reference impl) ----------
// AdvanceNonceAccount: u32 LE tag 4 (no payload); keys [nonce(w), recent_blockhashes, authority(s)]
const nonceIx = {
  programId: p("11111111111111111111111111111111"),
  keys: [
    { pubkey: p(NONCE), isSigner: false, isWritable: true },
    { pubkey: p(SYSVAR_RECENT_BLOCKHASHES), isSigner: false, isWritable: false },
    { pubkey: p(OWNER), isSigner: true, isWritable: false },
  ],
  data: u32le(4),
};
out.nonce_ix = {
  program_id: nonceIx.programId.toBase58(),
  keys: nonceIx.keys.map((k) => ({ pubkey: k.pubkey.toBase58(), isSigner: k.isSigner, isWritable: k.isWritable })),
  data: Buffer.from(nonceIx.data).toString("base64"),
};

// ---------- 5. full transactions (web3.js wire format — reference impl) ----------
const nonceHash = bs58.encode(crypto.createHash("sha256").update("fixture.nonce_hash").digest());
const mkTx = () => { const t = new web3.Transaction(); t.feePayer = p(OWNER); t.recentBlockhash = nonceHash; return t; };
const txClaim = mkTx();
txClaim.add(nonceIx, { keys: claimKeys, programId: p(PROGRAM), data: claimData });
out.tx_claim_b58 = bs58.encode(txClaim.serialize({ requireAllSignatures: false, verifySignatures: false }));

const txRebalance = mkTx();
txRebalance.add(nonceIx, { keys: removeKeys, programId: p(PROGRAM), data: removeData }, { keys: addKeys, programId: p(PROGRAM), data: addData });
out.tx_rebalance_b58 = bs58.encode(txRebalance.serialize({ requireAllSignatures: false, verifySignatures: false }));

// ---------- 6. PDA vectors ----------
out.pdas = {
  bin_array_120: claimBinKeys[0].toBase58(),
  bin_array_121: claimBinKeys[1].toBase58(),
  bin_array_neg1: deriveBinArray(p(POOL), new BN(-1), p(PROGRAM))[0].toBase58(),
  event_authority: eventAuthority,
  bitmap_extension: deriveBinArrayBitmapExtension(p(POOL), p(PROGRAM))[0].toBase58(),
};

// ---------- 7. nonce account bytes (decodeNonceHash ground truth) ----------
const nonceData = Buffer.concat([
  u32le(1), u32le(1),                    // version, state (Initialized)
  p(OWNER).toBuffer(),                   // authority
  bs58.decode(nonceHash),                // stored blockhash @40
  u64le(0),                              // fee_calculator
]);
out.nonce_account = nonceData.toString("base64");

// ---------- 8. account bytes (bytemuck C-repr layouts) ----------
// PositionV2: bins 0 (share 1e9, fee_x 1e6) and 5 (share 5e8, fee_y 2e6) non-empty
const u128le = (n) => Buffer.concat([u64le(n), u64le(0)]);
const positionBase = Buffer.concat([
  Buffer.from([117, 176, 212, 199, 245, 180, 133, 182]), // PositionV2 discriminator
  p(POOL).toBuffer(), p(OWNER).toBuffer(),
  ...Array.from({ length: 70 }, (_, i) => u128le(i === 0 ? "1000000000" : i === 5 ? "500000000" : "0")), // liquidity_shares
]);
const rewardInfos = Buffer.concat(Array.from({ length: 70 }, () => Buffer.concat([u128le(0), u128le(0), u64le(0), u64le(0)]))); // UserRewardInfo: per_token_completes[2], pendings[2]
const feeInfos = Buffer.concat(Array.from({ length: 70 }, (_, i) => Buffer.concat([
  u128le(0), u128le(0),                   // fee_x/y per_token_complete
  u64le(i === 0 ? "1000000" : "0"), u64le(i === 5 ? "2000000" : "0"), // fee_x/y_pending
])));
out.position_v2 = Buffer.concat([
  positionBase, rewardInfos, feeInfos,
  i32le(8450), i32le(8520), // lower, upper
  u64le("1700000000"),      // last_updated_at
  u64le(0), u64le(0), u64le(0), u64le(0), // total_claimed x/y, total_claimed_rewards[2]
  p(OWNER).toBuffer(), u64le(0), u8(0),   // operator, lock_release_point, _padding_0
  p(OWNER).toBuffer(), u8(2), u8(0),      // fee_owner, version, permissionless_operation_bits
  Buffer.alloc(85),
]).toString("base64");

// LbPair: active_id=8500, bin_step=10
const staticParams = Buffer.concat([
  u16le(500), u16le(30), u16le(600), u16le(5000),       // base_factor, filter_period, decay, reduction
  u32le(0), u32le(100000), i32le(-443636), i32le(443636), // var fee control, max vol, min/max bin
  u16le(0), u8(0), u8(0), u8(0), Buffer.alloc(3),       // protocol_share, powers, collect mode, padding
]);
const vParams = Buffer.concat([
  u32le(0), u32le(0), i32le(0), Buffer.alloc(4), u64le(0), Buffer.alloc(8), // vol acc, ref, index_ref, last_update
]);
const rewardInfo = (mint) => Buffer.concat([mint, mint, mint, u64le(0), u64le(0), u64le(0), u64le(0), u64le(0)]); // 8×8 = 144? no: 32×3 + 8×5 = 136
out.lb_pair = Buffer.concat([
  Buffer.from([33, 11, 49, 98, 181, 101, 177, 13]), // LbPair discriminator
  staticParams, vParams,
  u8(255), Buffer.alloc(2), u8(0),       // bump_seed, bin_step_seed, pair_type
  i32le(8500), u16le(10), u8(0), u8(0), Buffer.alloc(2), u8(0), u8(0), // active_id, bin_step, status, flags, seeds, activation_type, creator_ctrl
  p(TOKEN_X_MINT).toBuffer(), p(TOKEN_Y_MINT).toBuffer(),
  p(RESERVE_X).toBuffer(), p(RESERVE_Y).toBuffer(),
  u64le(0), u64le(0), Buffer.alloc(32),   // protocol_fee, _padding_1
  Buffer.concat([rewardInfo(ZERO.toBuffer()), rewardInfo(ZERO.toBuffer())]), // reward_infos[2]
  ZERO.toBuffer(), Buffer.alloc(128), u64le(0), Buffer.alloc(32), // oracle, bitmap, last_updated_at, _padding_2
  ZERO.toBuffer(), ZERO.toBuffer(), u64le(0), u64le(0), // pre_activation, base_key, activation_point, pre_activation_duration
  Buffer.alloc(8), u64le(0), ZERO.toBuffer(), // _padding_3, _padding_4, creator
  u8(0), u8(0), u8(0), Buffer.alloc(21),   // flags, flags, version, _reserved
]).toString("base64");

// BinArray index 120: bin 0 has liquidity 2e9, amounts 2e9/3e8
const emptyBin = Buffer.alloc(144);
const filledBin = Buffer.concat([
  u64le("2000000000"), u64le("300000000"), u128le(0), u128le("2000000000"), // amount_x, amount_y, price, liquidity_supply
  u64le(0), u64le(0), u64le(0), u64le(0),                                   // fulfilled x/y, limit fees
  u128le(0), u128le(0),                                                     // fee per-token stored
  u64le(0), u64le(0), u64le(0),                                             // open order, total, processed
  u32le(0), u8(0), Buffer.alloc(3),                                         // order_age, ask_side, padding
]);
out.bin_array = Buffer.concat([
  Buffer.from([92, 142, 92, 220, 5, 148, 70, 181]), // BinArray discriminator
  u64le(120), u8(0), Buffer.alloc(7), p(POOL).toBuffer(),
  filledBin, ...Array.from({ length: 69 }, () => emptyBin),
]).toString("base64");

// Aligned BinArray for position_amounts: array 120 covers bins 8400..8469,
// so position bins 8450 (share idx 0) → inner 50, 8455 (share idx 5) → inner 55.
const binHeader = Buffer.concat([
  Buffer.from([92, 142, 92, 220, 5, 148, 70, 181]), // BinArray discriminator
  u64le(120), u8(0), Buffer.alloc(7), p(POOL).toBuffer(),
]);
const alignedBins = Array.from({ length: 70 }, () => emptyBin);
const bin50 = Buffer.concat([
  u64le("2000000000"), u64le("300000000"), u128le(0), u128le("2000000000"), // amount_x, amount_y, price, liquidity_supply
  Buffer.alloc(144 - 8 - 8 - 16 - 16),
]);
alignedBins[50] = bin50; // bin 8450: liquidity 2e9, amounts 2e9/3e8
const bin55 = Buffer.concat([
  u64le("1000000000"), u64le("100000000"), u128le(0), u128le("1000000000"), // amount_x, amount_y, price, liquidity_supply
  Buffer.alloc(144 - 8 - 8 - 16 - 16),
]);
alignedBins[55] = bin55; // bin 8455: liquidity 1e9, amounts 1e9/1e8
out.bin_array_aligned = Buffer.concat([binHeader, ...alignedBins]).toString("base64");

out.keys = { program: PROGRAM, pool: POOL, position: POSITION, owner: OWNER, nonce: NONCE,
  token_x_mint: TOKEN_X_MINT, token_y_mint: TOKEN_Y_MINT, reserve_x: RESERVE_X, reserve_y: RESERVE_Y,
  user_token_x: USER_TOKEN_X, user_token_y: USER_TOKEN_Y, token_program: TOKEN_PROGRAM,
  memo_program: MEMO_PROGRAM, sysvar_recent_blockhashes: SYSVAR_RECENT_BLOCKHASHES, nonce_hash: nonceHash };
fs.writeFileSync(require("node:path").join(__dirname, "..", "plugins", "dlmm-core", "tests", "fixtures.json"), JSON.stringify(out, null, 1));
console.log("fixtures written:", Object.keys(out).join(", "));
console.log("position_v2 len:", Buffer.from(out.position_v2, "base64").length, "lb_pair len:", Buffer.from(out.lb_pair, "base64").length, "bin_array len:", Buffer.from(out.bin_array, "base64").length);
