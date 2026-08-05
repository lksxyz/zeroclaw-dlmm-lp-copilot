/**
 * Self-hosted Solana Action endpoint for DLMM LP Copilot.
 *
 * Routes:
 *   GET  /actions/claim?pos=<pubkey>&pool=<pubkey>       → Action metadata
 *   POST /actions/claim?pos=<pubkey>&pool=<pubkey>       → unsigned claimFee tx
 *   GET  /actions/rebalance?pos=<pubkey>&pool=<pubkey>&new_low=<bin>&new_high=<bin>&nonce=<pubkey>
 *   POST /actions/rebalance (same params in body.account)  → atomic removeLiquidity+addLiquidityByStrategy on durable nonce
 *
 * Deploy:   npx wrangler deploy
 * Set:      wrangler secret put RPC_URL
 *           wrangler secret put NONCE_ACCOUNT    (rebalance only)
 *           wrangler secret put NONCE_AUTHORITY  (rebalance only)
 *
 * Single-operator design: NONCE_AUTHORITY must be the LP's own wallet
 * (same as `wallet_pubkey` in config.example.toml). The rebalance tx is
 * signed by the user alone — user IS the nonce authority. The endpoint
 * rejects any other signer with 403.
 */
import {
  ActionGetResponse,
  ActionPostResponse,
  ACTIONS_CORS_HEADERS,
  createPostResponse,
} from '@solana/actions';
import { Connection, PublicKey } from '@solana/web3.js';
import { buildClaimFee, buildRebalance } from './dlmm';

export interface Env {
  RPC_URL: string;
  DLMM_PROGRAM: string;
  NONCE_ACCOUNT?: string;
  NONCE_AUTHORITY?: string;
}

const cors = (extra: Record<string, string> = {}) => ({
  ...ACTIONS_CORS_HEADERS,
  'Access-Control-Allow-Origin': '*',
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
  async fetch(request: Request, env: Env): Promise<Response> {
    if (request.method === 'OPTIONS') {
      return new Response(null, { headers: cors() });
    }

    const url = new URL(request.url);
    const baseUrl = url.origin; // icons are served from wherever this worker lives
    try {
      if (url.pathname === '/actions/claim') {
        return request.method === 'GET'
          ? await handleClaimGet(url, baseUrl)
          : await handleClaimPost(request, url, env);
      }
      if (url.pathname === '/actions/rebalance') {
        return request.method === 'GET'
          ? await handleRebalanceGet(url, baseUrl)
          : await handleRebalancePost(request, url, env);
      }
      if (url.pathname === '/health') {
        return new Response(JSON.stringify({ ok: true }), {
          headers: cors({ 'content-type': 'application/json' }),
        });
      }
      const icon = ICONS[url.pathname];
      if (icon) {
        return new Response(Uint8Array.from(atob(icon), (c) => c.charCodeAt(0)), {
          headers: { 'content-type': 'image/png', 'cache-control': 'public, max-age=86400' },
        });
      }
      return new Response('Not found', { status: 404, headers: cors() });
    } catch (e: any) {
      // Log details server-side; never leak internals to the client.
      console.error('handler error:', e);
      return new Response(JSON.stringify({ error: 'internal error' }), {
        status: 500,
        headers: cors({ 'content-type': 'application/json' }),
      });
    }
  },
};

// --- CLAIM ---------------------------------------------------------------

async function handleClaimGet(url: URL, baseUrl: string): Promise<Response> {
  const pos = url.searchParams.get('pos');
  const pool = url.searchParams.get('pool');
  if (!pos || !pool) return badRequest('pos and pool are required');

  // We don't fetch price here — the GET is shown by the wallet *before* the user
  // signs. Wallets respect an upper latency budget; we return quickly.
  const metadata: ActionGetResponse = {
    type: 'action',
    title: 'Claim DLMM fees',
    icon: `${baseUrl}/icon-claim.png`,
    description:
      'Claim accrued trading fees from this Meteora DLMM position. ' +
      'Funds land in your wallet, position stays open.',
    label: 'Claim',
    links: { actions: [] },
  };
  return new Response(JSON.stringify(metadata), {
    headers: cors({ 'content-type': 'application/json' }),
  });
}

async function handleClaimPost(
  request: Request,
  url: URL,
  env: Env,
): Promise<Response> {
  const pos = url.searchParams.get('pos');
  const pool = url.searchParams.get('pool');
  if (!pos || !pool) return badRequest('pos and pool are required');

  const body = await readJsonBody(request);
  if (!body?.account) return badRequest('account is required');

  let user: PublicKey;
  let position: PublicKey;
  let lbPair: PublicKey;
  try {
    user = new PublicKey(body.account);
    position = new PublicKey(pos);
    lbPair = new PublicKey(pool);
  } catch {
    return badRequest('invalid public key');
  }

  const conn = new Connection(env.RPC_URL, 'confirmed');

  const tx = await buildClaimFee(conn, env, position, lbPair, user);

  const payload: ActionPostResponse = await createPostResponse({
    fields: { type: 'transaction', transaction: tx, message: 'Claim DLMM fees' },
  });
  return new Response(JSON.stringify(payload), {
    headers: cors({ 'content-type': 'application/json' }),
  });
}

// --- REBALANCE -----------------------------------------------------------

async function handleRebalanceGet(url: URL, baseUrl: string): Promise<Response> {
  const pos = url.searchParams.get('pos');
  const pool = url.searchParams.get('pool');
  const newLow = url.searchParams.get('new_low');
  const newHigh = url.searchParams.get('new_high');
  if (!pos || !pool || !newLow || !newHigh) {
    return badRequest('pos, pool, new_low, new_high are required');
  }

  const metadata: ActionGetResponse = {
    type: 'action',
    title: 'Rebalance DLMM position',
    icon: `${baseUrl}/icon-rebalance.png`,
    description:
      `Atomic rebalance: removeLiquidity 100% from the current bin range, ` +
      `then addLiquidityByStrategy into the new bin range. ` +
      `Uses a durable nonce so the signature is valid as long as you need.`,
    label: 'Rebalance',
    links: { actions: [] },
  };
  return new Response(JSON.stringify(metadata), {
    headers: cors({ 'content-type': 'application/json' }),
  });
}

async function handleRebalancePost(
  request: Request,
  url: URL,
  env: Env,
): Promise<Response> {
  if (!env.NONCE_ACCOUNT || !env.NONCE_AUTHORITY) {
    return new Response(
      JSON.stringify({ error: 'rebalance requires NONCE_ACCOUNT and NONCE_AUTHORITY secrets' }),
      { status: 503, headers: cors({ 'content-type': 'application/json' }) },
    );
  }

  const pos = url.searchParams.get('pos');
  const pool = url.searchParams.get('pool');
  const newLow = url.searchParams.get('new_low');
  const newHigh = url.searchParams.get('new_high');
  if (!pos || !pool || !newLow || !newHigh) {
    return badRequest('pos, pool, new_low, new_high are required');
  }

  const body = await readJsonBody(request);
  if (!body?.account) return badRequest('account is required');

  // Single-operator: the signing user must BE the nonce authority (their own
  // nonce account). The Action spec provides exactly one signer, so we refuse
  // anyone else — the tx would fail on-chain anyway, but fail early and clean.
  if (body.account !== env.NONCE_AUTHORITY) {
    return new Response(
      JSON.stringify({ error: 'account is not the authorized rebalance operator' }),
      { status: 403, headers: cors({ 'content-type': 'application/json' }) },
    );
  }

  let newLowerBinId: number;
  let newUpperBinId: number;
  try {
    newLowerBinId = parseInt(newLow, 10);
    newUpperBinId = parseInt(newHigh, 10);
    if (!Number.isInteger(newLowerBinId) || !Number.isInteger(newUpperBinId)) {
      throw new Error('not int');
    }
    if (newLowerBinId <= 0 || newUpperBinId <= 0 || newUpperBinId <= newLowerBinId) {
      throw new Error('bad range');
    }
  } catch {
    return badRequest('new_low and new_high must be positive integers with new_low < new_high');
  }

  let user: PublicKey;
  let position: PublicKey;
  let lbPair: PublicKey;
  try {
    user = new PublicKey(body.account);
    position = new PublicKey(pos);
    lbPair = new PublicKey(pool);
  } catch {
    return badRequest('invalid public key');
  }

  const conn = new Connection(env.RPC_URL, 'confirmed');

  const tx = await buildRebalance(conn, env, {
    position,
    pool: lbPair,
    newLowerBinId,
    newUpperBinId,
    user,
    nonceAccount: new PublicKey(env.NONCE_ACCOUNT),
    nonceAuthority: new PublicKey(env.NONCE_AUTHORITY),
  });

  const payload: ActionPostResponse = await createPostResponse({
    fields: { type: 'transaction', transaction: tx, message: 'Rebalance DLMM position' },
  });
  return new Response(JSON.stringify(payload), {
    headers: cors({ 'content-type': 'application/json' }),
  });
}

// --- helpers -------------------------------------------------------------

async function readJsonBody(request: Request): Promise<{ account?: string } | null> {
  try {
    return (await request.json()) as { account?: string };
  } catch {
    return null;
  }
}

function badRequest(msg: string): Response {
  return new Response(JSON.stringify({ error: msg }), {
    status: 400,
    headers: cors({ 'content-type': 'application/json' }),
  });
}
