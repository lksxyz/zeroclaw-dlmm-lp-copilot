import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import bs58 from 'bs58';
import { parseTx } from '../src/index.ts';

const FIXTURES = JSON.parse(
  fs.readFileSync(
    path.join(import.meta.dirname, '../../plugins/dlmm-core/tests/fixtures.json'),
    'utf8',
  ),
);

function b64url(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString('base64url');
}

test('claim tx preview parses from fixture bytes', () => {
  const bytes = bs58.decode(FIXTURES.tx_claim_b58);
  const tx = parseTx(b64url(bytes));
  assert.ok(tx, 'parses');
  assert.equal(tx.numSigners, 1);
  assert.equal(tx.feePayer, FIXTURES.keys.owner);
  assert.equal(tx.recentBlockhash, FIXTURES.keys.nonce_hash);
  assert.equal(tx.instructions.length, 2);
  // nonce first, then DLMM claim
  assert.equal(tx.instructions[0].programId, '11111111111111111111111111111111');
  assert.equal(tx.instructions[0].dataLen, 4);
  assert.equal(tx.instructions[1].programId, FIXTURES.keys.program);
});

test('rebalance tx preview parses from fixture bytes', () => {
  const bytes = bs58.decode(FIXTURES.tx_rebalance_b58);
  const tx = parseTx(b64url(bytes));
  assert.ok(tx, 'parses');
  assert.equal(tx.instructions.length, 3); // nonce + remove + add
  assert.equal(tx.instructions[0].programId, '11111111111111111111111111111111');
  assert.equal(tx.instructions[1].programId, FIXTURES.keys.program);
  assert.equal(tx.instructions[2].programId, FIXTURES.keys.program);
});

test('malformed payloads are rejected, not thrown', () => {
  assert.equal(parseTx('!!!not-base64!!!'), null);
  assert.equal(parseTx(''), null);
  // truncated: valid b64url of 3 bytes → signature count says 1 → needs 64+
  assert.equal(parseTx(b64url(new Uint8Array([1, 0, 0]))), null);
  // signature count with no signature bytes
  assert.equal(parseTx(b64url(new Uint8Array([1, 1]))), null);
});

test('v0-prefixed message parses', () => {
  // Legacy fixture with the v0 version byte (0x80) inserted at the message
  // boundary (after the signature placeholders) still parses.
  const bytes = bs58.decode(FIXTURES.tx_claim_b58);
  const msgStart = 1 + bytes[0] * 64; // shortvec sig count + 64B placeholders
  const v0 = new Uint8Array(bytes.length + 1);
  v0.set(bytes.subarray(0, msgStart), 0);
  v0[msgStart] = 0x80;
  v0.set(bytes.subarray(msgStart), msgStart + 1);
  const tx = parseTx(b64url(v0));
  assert.ok(tx, 'v0 parses');
  assert.equal(tx.instructions.length, 2);
});
