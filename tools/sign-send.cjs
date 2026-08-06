// Sign + submit an unsigned transaction (base64) with a keypair file.
//
// Usage:
//   node sign-send.cjs "<tx_base64>" <keypair.json> [--url mainnet-beta]
//
// Example (tx from the bot, key you already have):
//   node sign-send.cjs "$(cat /tmp/claim_unsigned.base64)" /home/fahmi/.zeroclaw/operator-mainnet.json
//
// Prints the tx signature + Solscan link on success.
const fs = require("node:fs");
const { Connection, Keypair, Transaction } = require("@solana/web3.js");

const RPC = "https://api.mainnet-beta.solana.com";

async function main() {
  const [txB64, keyPath] = process.argv.slice(2);
  if (!txB64 || !keyPath) {
    console.error("usage: node sign-send.cjs <tx_base64> <keypair.json>");
    process.exit(1);
  }

  const kp = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(keyPath, "utf8")))
  );
  const c = new Connection(RPC, "confirmed");

  const tx = Transaction.from(Buffer.from(txB64, "base64"));
  // The unsigned tx's blockhash may be stale; refresh it per the Actions
  // spec (wallet sets latest blockhash for unsigned txs). feePayer = signer.
  const { blockhash } = await c.getLatestBlockhash("confirmed");
  tx.recentBlockhash = blockhash;
  tx.feePayer = kp.publicKey;

  console.log("signing as:", kp.publicKey.toBase58());
  tx.sign(kp);

  const sig = await c.sendTransaction(tx, { preflightCommitment: "confirmed" });
  console.log("✓ tx sent:", sig);
  await c.confirmTransaction(sig, "confirmed");
  console.log("✓ confirmed:", sig);
  console.log("view: https://solscan.io/tx/" + sig);
}

main().catch((e) => {
  console.error("FATAL:", e.message);
  if (e.logs) console.error(e.logs.join("\n"));
  process.exit(1);
});
