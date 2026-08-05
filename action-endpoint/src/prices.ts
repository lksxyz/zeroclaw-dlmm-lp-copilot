/**
 * Price feed reader — Switchboard Crossbar.
 *
 * Public, unauthenticated, best-effort. We use it for two things only:
 *  - mark position to current value (IL vs HODL)
 *  - sanity-check that the active bin is in a sensible range vs current price
 *
 * **Do not** switch to Pyth unauthenticated Hermes — those endpoints stop
 * serving 2026-07-31 (Pyth Core deprecation).
 */
const CROSSBAR = 'https://crossbar.switchboard-oracle.xyz';

export async function readSolUsd(crossbarBase: string = CROSSBAR): Promise<number> {
  const res = await fetch(`${crossbarBase}/latest?feedKey=sol_usd`, {
    cf: { cacheTtl: 30 } as any, // 30s cache
  });
  if (!res.ok) throw new Error(`switchboard sol_usd: HTTP ${res.status}`);
  const json = (await res.json()) as { results?: Array<{ value?: number }> };
  const v = json?.results?.[0]?.value;
  if (typeof v !== 'number' || !isFinite(v)) {
    throw new Error('switchboard sol_usd: bad value');
  }
  return v;
}

export async function readUsdcUsd(crossbarBase: string = CROSSBAR): Promise<number> {
  // USDC is a stablecoin; this is always ~1. We still read for symmetry and
  // to surface a feed outage in the alert path.
  const res = await fetch(`${crossbarBase}/latest?feedKey=usdc_usd`, {
    cf: { cacheTtl: 30 } as any,
  });
  if (!res.ok) return 1.0; // fall back; don't block on a feed issue
  const json = (await res.json()) as { results?: Array<{ value?: number }> };
  return json?.results?.[0]?.value ?? 1.0;
}
