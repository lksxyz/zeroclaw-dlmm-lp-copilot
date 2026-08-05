/**
 * DLMM transaction builders.
 *
 * Wraps the Meteora DLMM SDK helpers in two end-to-end functions that return
 * base64-encoded **unsigned** transactions. The user's wallet signs.
 */
import { Connection, PublicKey, Transaction, TransactionInstruction } from '@solana/web3.js';
import DLMM, { StrategyType } from '@meteora-ag/dlmm';
import BN from 'bn.js';
import { encodeAdvanceNonceData, decodeNonceHash } from './nonce';
import type { Env } from './index';

export interface RebalanceArgs {
  position: PublicKey;
  pool: PublicKey;
  newLowerBinId: number;
  newUpperBinId: number;
  user: PublicKey;
  nonceAccount: PublicKey;
  nonceAuthority: PublicKey;
}

/**
 * Build an unsigned `claimAllRewardsByPosition` transaction.
 *
 * Claims swap fees (+ any LM rewards) for one position. No recent blockhash
 * concerns because there is no approval queue — the user signs immediately on
 * opening the Blink/Action URL.
 *
 * Returns a `Transaction`; `createPostResponse` base64-encodes it per the
 * Solana Actions spec.
 */
export async function buildClaimFee(
  conn: Connection,
  _env: Env,
  position: PublicKey,
  pool: PublicKey,
  user: PublicKey,
): Promise<Transaction> {
  const dlmm = await DLMM.create(conn, pool);

  const { userPositions } = await dlmm.getPositionsByUserAndLbPair(user);
  const ourPos = userPositions.find(
    (p) => p.publicKey.toBase58() === position.toBase58(),
  );
  if (!ourPos) {
    throw new Error(`position ${position.toBase58()} not found for user ${user.toBase58()}`);
  }

  const claimTxs = await dlmm.claimAllRewardsByPosition({ owner: user, position: ourPos });
  if (claimTxs.length === 0) {
    throw new Error('nothing to claim for this position');
  }

  // Multiple claim txs (fee + rewards) → one transaction, all instructions.
  const tx = new Transaction();
  claimTxs.forEach((t) => tx.add(...t.instructions));
  tx.feePayer = user;
  const { blockhash } = await conn.getLatestBlockhash('confirmed');
  tx.recentBlockhash = blockhash;

  return tx;
}

/**
 * Build an atomic rebalance on a durable nonce.
 *
 *   [advanceNonceAccount, removeLiquidity..., addLiquidityByStrategy]
 *
 * The transaction is valid until the user signs and submits, no matter how
 * long approval takes. One nonce account per concurrent pending rebalance.
 *
 * The nonce authority MUST be the signing user's wallet (single-operator
 * deployment): `AdvanceNonceAccount` requires the authority's signature, and
 * the Solana Action spec provides exactly one signer — `body.account`.
 * `handleRebalancePost` validates this before calling us.
 */
export async function buildRebalance(
  conn: Connection,
  _env: Env,
  args: RebalanceArgs,
): Promise<Transaction> {
  const dlmm = await DLMM.create(conn, args.pool);

  // The user position state (bin distribution).
  const { userPositions } = await dlmm.getPositionsByUserAndLbPair(args.user);
  const ourPos = userPositions.find(
    (p) => p.publicKey.toBase58() === args.position.toBase58(),
  );
  if (!ourPos) {
    throw new Error(`position ${args.position.toBase58()} not found for user ${args.user.toBase58()}`);
  }

  // 1. removeLiquidity(100% of bins) — may chunk into multiple txs for wide ranges.
  const removeTxs = await dlmm.removeLiquidity({
    user: args.user,
    position: args.position,
    fromBinId: ourPos.positionData.lowerBinId,
    toBinId: ourPos.positionData.upperBinId,
    bps: new BN(10_000), // 100%
  });

  // 2. addLiquidityByStrategy — re-deposit into the new range with the
  // just-removed balances.
  const addTx = await dlmm.addLiquidityByStrategy({
    positionPubKey: args.position,
    user: args.user,
    totalXAmount: new BN(ourPos.positionData.totalXAmount),
    totalYAmount: new BN(ourPos.positionData.totalYAmount),
    strategy: {
      minBinId: args.newLowerBinId,
      maxBinId: args.newUpperBinId,
      strategyType: StrategyType.Spot,
    },
  });

  // 3. advanceNonceAccount — must be the **first** instruction in a durable-nonce tx.
  const SYSTEM_PROGRAM = new PublicKey('11111111111111111111111111111111');
  const nonceIx = new TransactionInstruction({
    keys: [
      { pubkey: args.nonceAccount, isSigner: false, isWritable: true },
      // The nonce authority must sign — it IS the user (validated in index.ts).
      { pubkey: args.nonceAuthority, isSigner: true, isWritable: false },
    ],
    programId: SYSTEM_PROGRAM,
    data: encodeAdvanceNonceData(),
  });

  const tx = new Transaction();
  tx.add(nonceIx);
  removeTxs.forEach((t) => tx.add(...t.instructions));
  tx.add(...addTx.instructions);
  tx.feePayer = args.user;

  // Durable nonce: no recent blockhash; the nonce account IS the blockhash.
  const nonceAccountInfo = await conn.getAccountInfo(args.nonceAccount, 'confirmed');
  if (!nonceAccountInfo) {
    throw new Error(`nonce account ${args.nonceAccount.toBase58()} not found`);
  }
  tx.recentBlockhash = decodeNonceHash(nonceAccountInfo.data);

  return tx;
}
