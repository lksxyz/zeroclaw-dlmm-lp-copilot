/**
 * Unit tests for the durable-nonce helpers (src/nonce.ts).
 *
 * Pure functions, no network, no Solana SDK — run with `npm test`
 * (`node --test`), which type-strips TS on Node ≥ 22.6.
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { encodeAdvanceNonceData, decodeNonceHash } from '../src/nonce.ts';

test('encodeAdvanceNonceData emits System Program tag 4 as u32 LE', () => {
  assert.deepEqual(encodeAdvanceNonceData(), Buffer.from([0x04, 0x00, 0x00, 0x00]));
});

test('decodeNonceHash skips the 4-byte version header and base58-encodes', () => {
  const hash = Buffer.alloc(32, 0x42);
  const data = Buffer.concat([Buffer.from([0x00, 0x00, 0x00, 0x00]), hash]);
  assert.equal(decodeNonceHash(data), '5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf');
});

test('decodeNonceHash round-trips a sequential byte vector', () => {
  const data = Buffer.concat([
    Buffer.from([0x00, 0x00, 0x00, 0x00]),
    Buffer.from(Array.from({ length: 32 }, (_, i) => i)),
  ]);
  assert.equal(decodeNonceHash(data), '1thX6LZfHDZZKUs92febYZhYRcXddmzfzF2NvTkPNE');
});

test('decodeNonceHash ignores trailing bytes after the 32-byte hash', () => {
  const hash = Buffer.alloc(32, 0x42);
  const data = Buffer.concat([
    Buffer.from([0x00, 0x00, 0x00, 0x00]),
    hash,
    Buffer.alloc(64), // e.g. recent blockhashes + authority pubkey
  ]);
  assert.equal(decodeNonceHash(data), '5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf');
});

test('decodeNonceHash rejects data shorter than the 36-byte minimum', () => {
  assert.throws(() => decodeNonceHash(Buffer.alloc(35)), /too short/);
});
