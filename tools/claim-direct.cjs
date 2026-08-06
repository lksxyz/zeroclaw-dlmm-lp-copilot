// Build the claim tx directly with the official SDK (no bot, no action URL).
// Returns the raw unsigned transaction (base64) for the operator to sign.
// The position belongs to 5XyiGVKPpsocryiD4CJuAuPs4QXgbMDfHdNT9AwSNbXV.
const fs = require("node:fs");
const { Connection, PublicKey, Keypair } = require("@solana/web3.js");
const DLMM = require("@meteora-ag/dlmm");

const RPC = "https://api.mainnet-beta.solana.com";
const POOL = new PublicKey("GFJxt2P2qUVbX9b14uuBQZHrdmhEV6Q618qvGnwdsX2L");
const POSITION = new PublicKey("BBrfbrZY24bvP4cWwh2tJ799dSatdCm4mRkJA3JkV2Jk");
const OWNER = new PublicKey("5XyiGVKPpsocryiD4CJuAuPs4QXgbMDfHdNT9AwSNbXV");
const PROGRAM = new PublicKey("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
const OP_KEY = process.argv[2] || "/home/fahmi/.zeroclaw/operator-mainnet.json";

(async () => {
  const c = new Connection(RPC, "confirmed");
  const dlmm = await DLMM.create(c, POOL, { programId: PROGRAM });
  console.log("pool loaded:", dlmm.pubkey.toBase58());
  console.log("active bin:", dlmm.lbPair.activeId);

  // Get the position data (LbPosition) for the claim.
  const positionData = await dlmm.getPosition(POSITION);
  console.log("position loaded, lower:", positionData.lowerBinId, "upper:", positionData.upperBinId);

  // Build the claim tx(s). Returns an array of transactions (unsigned).
  const claimTxs = await dlmm.claimSwapFee({
    owner: OWNER,
    position: positionData,
  });
  console.log("claim tx count:", claimTxs.length);

  // Serialize each unsigned tx for the operator to sign.
  for (let i = 0; i < claimTxs.length; i++) {
    const tx = claimTxs[i];
    tx.feePayer = OWNER;
    tx.recentBlockhash = (await c.getLatestBlockhash("confirmed")).blockhash;
    const signed = tx.serialize({ requireAllSignatures: false });
    const base64 = signed.toString("base64");
    const file = `/tmp/claim_unsigned_${i}.base64`;
    fs.writeFileSync(file, base64);
    console.log(`\n=== RAW UNSIGNED TX #${i} (base64) ===`);
    console.log(base64);
    console.log("=== END ===");
    console.log(`recent blockhash: ${tx.recentBlockhash}`);
    console.log(`fee payer: ${tx.feePayer.toBase58()}`);
    console.log(`saved to ${file}`);
  }
})().catch((e) => {
  console.error("FATAL:", e.message);
  if (e.logs) console.error(e.logs.join("\n"));
  process.exit(1);
});
