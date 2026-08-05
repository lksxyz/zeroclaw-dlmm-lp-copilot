/**
 * Durable-nonce encoding helpers (System Program `AdvanceNonceAccount`).
 *
 * Pure functions — no Solana/web3 imports, so they can be unit-tested
 * without a live RPC or the Meteora SDK (see test/nonce.test.ts).
 */
import bs58 from 'bs58';

/** Instruction tag for the System Program's `AdvanceNonceAccount`. */
const ADVANCE_NONCE_TAG = 4;

/** Nonce account layout: 4-byte u32 version, then the 32-byte stored hash. */
const BLOCKHASH_OFFSET = 4;
const BLOCKHASH_LEN = 32;

/**
 * Serialized data for `AdvanceNonceAccount`: little-endian u32 tag, no args.
 * Must be the FIRST instruction in a durable-nonce transaction.
 */
export function encodeAdvanceNonceData(): Buffer {
  const b = Buffer.alloc(4);
  b.writeUInt32LE(ADVANCE_NONCE_TAG, 0);
  return b;
}

/**
 * Extract the stored nonce hash from a nonce account's data and encode it
 * as base58 — the format `Transaction.recentBlockhash` expects.
 */
export function decodeNonceHash(data: Buffer): string {
  const need = BLOCKHASH_OFFSET + BLOCKHASH_LEN;
  if (data.length < need) {
    throw new Error(`nonce account data too short: ${data.length} < ${need}`);
  }
  return bs58.encode(data.subarray(BLOCKHASH_OFFSET, need));
}
