// This script demonstrates how to call Peer.connectRandom and prints basic info.
// Usage: node test_connect_random.js [mainnet|testnet11]
// Make sure you have wallet TLS certificate/key files on your machine.

import path from 'path';
import { fileURLToPath } from 'url';
import { Tls, Peer, PeerType, addressToPuzzleHash } from './index.js';

const MIN_HEIGHT = 5777842;
const MIN_HEIGHT_HEADER_HASH =
  "b29a4daac2434fd17a36e15ba1aac5d65012d4a66f99bed0bf2b5342e92e562c";

async function main() {
  const networkArg = (process.argv[2] || 'mainnet').toLowerCase();
  const peerType = networkArg === 'testnet11' ? PeerType.Testnet11 : PeerType.Mainnet;

  // Resolve default Chia cert/key locations (override with env vars if needed)
  const homedir = process.env.HOME || process.env.USERPROFILE;
  const sslDir = path.join(homedir, '.chia', networkArg, 'config', 'ssl', 'wallet');
  const certPath = process.env.CHIA_WALLET_CERT || path.join(sslDir, 'wallet_node.crt');
  const keyPath = process.env.CHIA_WALLET_KEY || path.join(sslDir, 'wallet_node.key');

  console.log(`Using cert: ${certPath}`);
  console.log(`Using key : ${keyPath}`);

  const tls = new Tls(certPath, keyPath);

  console.time('connectRandom');
  const peer = await Peer.connectRandom(peerType, tls);
  console.timeEnd('connectRandom');

  console.log(`Connected to ${peerType === PeerType.Mainnet ? 'mainnet' : 'testnet11'} peer at`, await peer.getHeaderHash(0));
  const coinsResp = await peer.getAllUnspentCoins(
    addressToPuzzleHash('xch1qm9g5qxq4xrqqkpvmk6j64ckpjp7xeq78mdmfz48lp8g5zgq4sxs2370ec'),
    MIN_HEIGHT,
    Buffer.from(MIN_HEIGHT_HEADER_HASH, "hex")
  );

  console.log('coinsResp', coinsResp);

  process.exit(0);
}

main().catch((err) => {
  console.error('Error:', err);
  process.exit(1);
}); 