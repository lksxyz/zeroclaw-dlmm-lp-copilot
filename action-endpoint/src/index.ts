/**
 * Stateless Solana Action relay — DLMM LP Copilot.
 *
 * The dlmm_builder plugin (in-wasm, on ZeroClaw) mechanically validates and
 * encodes an unsigned claim/rebalance transaction, then hands the LLM a
 * `solana-action:` URL with the tx bytes in the path:
 *
 *   solana-action:https://<relay>/tx/<base64url>
 *
 * Routes:
 *   GET  /tx/<b64url> → Action metadata. The preview is rendered FROM THE
 *                       BYTES — instruction count, program ids, nonce-first,
 *                       single signer — no RPC, no storage.
 *   POST /tx/<b64url> → echoes the tx back for the wallet to sign; refuses
 *                       any wallet that isn't the tx's fee payer (the only
 *                       required signer).
 *   GET  /health      → liveness.
 *
 * Security by construction: no secrets, no KV, no RPC, no SDKs, zero runtime
 * dependencies. The tx was already validated by the plugin ("LLM proposes,
 * plugin verifies"); only the operator's signature can land it.
 */

const SYSTEM_PROGRAM = '11111111111111111111111111111111';
const DLMM_DEVNET = 'LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo';
const DLMM_MAINNET = 'LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSK9q8Mfev5Rq';

const cors = (extra: Record<string, string> = {}) => ({
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'GET,POST,OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type',
  ...extra,
});

// 64×64 PNGs served inline (no asset binding / storage needed).
const ICONS: Record<string, string> = {
  '/icon-claim.png':
    'iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAIAAAAlC+aJAAAAVklEQVR42u3PQQ0AMAgEsNMyDZPDDzkzPQ08SZrUQHNerRYBAQEBAQEBAQEBAQEBAQEBAQEBAQEBgXEgfXcTEBAQEBAQEBAQEBAQEBAQEBAQEBAQEJj6pjYAxKAK5HsAAAAASUVORK5CYII=',
  '/icon-rebalance.png':
    'iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAIAAAAlC+aJAAAAV0lEQVR42u3PQQ0AMAgEsFMyEZODJuRM4DTwJGlSA82pt1oEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBMaB3N5NQEBAQEBAQEBAQEBAQEBAQEBAQEBAQGDqAzRQUOJoyc6PAAAAAElFTkSuQmCC',
};

export default {
  async fetch(request: Request): Promise<Response> {
    if (request.method === 'OPTIONS') {
      return new Response(null, { headers: cors() });
    }

    const url = new URL(request.url);
    const baseUrl = url.origin; // icons are served from wherever this worker lives

    try {
      if (url.pathname === '/health') {
        return new Response(JSON.stringify({ ok: true }), {
          headers: cors({ 'content-type': 'application/json' }),
        });
      }

      const tx = url.pathname.match(/^\/tx\/([A-Za-z0-9_-]+)$/)?.[1];
      if (tx) {
        const parsed = parseTx(tx);
        if (!parsed) return badRequest('invalid transaction payload');
        return request.method === 'GET'
          ? getAction(parsed, baseUrl)
          : postTransaction(request, parsed);
      }

      const icon = ICONS[url.pathname];
      if (icon) {
        return new Response(Uint8Array.from(atob(icon), (c) => c.charCodeAt(0)), {
          headers: { 'content-type': 'image/png', 'cache-control': 'public, max-age=86400' },
        });
      }
      return new Response('Not found', { status: 404, headers: cors() });
    } catch (e) {
      // Log details server-side; never leak internals to the client.
      console.error('handler error:', e);
      return new Response(JSON.stringify({ error: 'internal error' }), {
        status: 500,
        headers: cors({ 'content-type': 'application/json' }),
      });
    }
  },
};

// --- wire-format parsing (hand-rolled; mirrors web3.js compileMessage) ------

const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

function bs58Encode(bytes: Uint8Array): string {
  const digits: number[] = [];
  for (let i = 0; i < bytes.length; i++) {
    let carry = bytes[i];
    for (let j = 0; j < digits.length; j++) {
      carry += digits[j] << 8;
      digits[j] = carry % 58;
      carry = (carry / 58) | 0;
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = (carry / 58) | 0;
    }
  }
  let out = '';
  for (let i = 0; i < bytes.length && bytes[i] === 0; i++) out += '1';
  for (let i = digits.length - 1; i >= 0; i--) out += ALPHABET[digits[i]];
  return out;
}

interface ParsedTx {
  numSigners: number;
  feePayer: string;
  recentBlockhash: string;
  instructions: { programId: string; accountCount: number; dataLen: number }[];
}

function b64urlToBytes(s: string): Uint8Array | null {
  try {
    const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/'));
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  } catch {
    return null;
  }
}

function bytesToB64(bytes: Uint8Array): string {
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

/** Parse a serialized transaction: shortvec signature count + placeholders,
 * then the message (legacy or v0). Every length is bounds-checked — malformed
 * input returns null, never throws. */
export function parseTx(b64url: string): ParsedTx | null {
  const bytes = b64urlToBytes(b64url);
  if (!bytes) return null;

  let p = 0;
  const shortvec = (): number => {
    let n = 0;
    let shift = 0;
    for (;;) {
      if (p >= bytes.length) return -1;
      const b = bytes[p++];
      n |= (b & 0x7f) << shift;
      if ((b & 0x80) === 0) break;
      shift += 7;
    }
    return n;
  };

  // signatures: shortvec count, then 64 bytes each (unsigned → placeholders)
  const sigCount = shortvec();
  if (sigCount < 0 || sigCount > 8 || p + sigCount * 64 > bytes.length) return null;
  p += sigCount * 64;

  if (bytes[p] === 0x80) p++; // v0 message version prefix
  if (p + 3 > bytes.length) return null;
  const numRequired = bytes[p];
  p += 3; // numRequiredSignatures, readonly-signed, readonly-unsigned

  const numKeys = shortvec();
  if (numKeys < 1 || numKeys > 64) return null;
  const keys: string[] = [];
  for (let i = 0; i < numKeys; i++) {
    if (p + 32 > bytes.length) return null;
    keys.push(bs58Encode(bytes.subarray(p, p + 32)));
    p += 32;
  }
  if (p + 32 > bytes.length) return null;
  const recentBlockhash = bs58Encode(bytes.subarray(p, p + 32));
  p += 32;

  const numIxs = shortvec();
  if (numIxs < 0 || numIxs > 8) return null;
  const instructions: ParsedTx['instructions'] = [];
  for (let i = 0; i < numIxs; i++) {
    if (p >= bytes.length) return null;
    const pidIdx = bytes[p++];
    const acctCount = shortvec();
    if (acctCount < 0 || p + acctCount > bytes.length) return null;
    p += acctCount;
    const dataLen = shortvec();
    if (dataLen < 0 || p + dataLen > bytes.length) return null;
    p += dataLen;
    instructions.push({ programId: keys[pidIdx] ?? '?', accountCount: acctCount, dataLen });
  }

  return { numSigners: numRequired, feePayer: keys[0], recentBlockhash, instructions };
}

// --- handlers ----------------------------------------------------------------

function getAction(tx: ParsedTx, baseUrl: string): Response {
  const { title, icon, description } = preview(tx);
  return new Response(
    JSON.stringify({
      type: 'action',
      title,
      icon: `${baseUrl}${icon}`,
      description,
      label: 'Sign transaction',
      links: { actions: [] },
    }),
    { headers: cors({ 'content-type': 'application/json' }) },
  );
}

async function postTransaction(request: Request, tx: ParsedTx): Promise<Response> {
  let body: { account?: string } | null = null;
  try {
    body = (await request.json()) as { account?: string };
  } catch {
    /* malformed body */
  }
  if (!body?.account) return badRequest('account is required');

  // The plugin builds the tx with exactly one required signer: the operator
  // (fee payer). Refuse anyone else — the tx couldn't land anyway.
  if (body.account !== tx.feePayer) {
    return new Response(
      JSON.stringify({ error: 'this transaction is addressed to a different wallet' }),
      { status: 403, headers: cors({ 'content-type': 'application/json' }) },
    );
  }

  // Echo the decoded tx back (standard base64, as the Actions spec expects).
  const b64url = new URL(request.url).pathname.split('/').pop()!;
  const bytes = b64urlToBytes(b64url)!;
  return new Response(
    JSON.stringify({
      type: 'transaction',
      transaction: bytesToB64(bytes),
      message: 'DLMM LP Copilot — sign to broadcast',
    }),
    { headers: cors({ 'content-type': 'application/json' }) },
  );
}

/** Human-readable preview rendered purely from the message bytes. */
function preview(tx: ParsedTx): { title: string; icon: string; description: string } {
  const ixs = tx.instructions;
  const first = ixs[0];
  const nonceFirst =
    first?.programId === SYSTEM_PROGRAM && first.dataLen === 4 && first.accountCount === 3;
  const dlmm = ixs.some((i) => i.programId === DLMM_DEVNET || i.programId === DLMM_MAINNET);
  const kind = ixs.length === 2 ? 'claim' : ixs.length === 3 ? 'rebalance' : 'unknown';
  const programNames = ixs.map((i) =>
    i.programId === SYSTEM_PROGRAM ? 'System' : dlmm ? 'DLMM' : '?',
  );

  const title = kind === 'claim' ? 'Claim DLMM fees' : kind === 'rebalance' ? 'Rebalance DLMM position' : 'DLMM transaction';
  const icon = kind === 'claim' ? '/icon-claim.png' : kind === 'rebalance' ? '/icon-rebalance.png' : '/icon-claim.png';
  const description =
    `${ixs.length} instructions (${programNames.join(' → ')}) from the dlmm_builder plugin. ` +
    `${nonceFirst ? 'Durable-nonce first: valid until used. ' : ''}` +
    `Signed by your wallet (${tx.numSigners} signer). ` +
    `No server secrets — the relay only echoes validated bytes.`;

  return { title, icon, description };
}

function badRequest(msg: string): Response {
  return new Response(JSON.stringify({ error: msg }), {
    status: 400,
    headers: cors({ 'content-type': 'application/json' }),
  });
}
