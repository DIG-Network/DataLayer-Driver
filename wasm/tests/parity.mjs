import assert from "node:assert/strict";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);

const wasm = require("../pkg-node");
const napi = require("@dignetwork/datalayer-driver");
wasm.init();

const hex = (b) => Buffer.from(b).toString("hex");
const eqBytes = (a, b, msg) => assert.equal(hex(a), hex(b), msg);

// Deterministic key material.
const sk = Buffer.alloc(32, 1);

// 1. Key derivation parity.
const pkW = wasm.secretKeyToPublicKey(sk);
const pkN = napi.secretKeyToPublicKey(sk);
eqBytes(pkW, pkN, "secretKeyToPublicKey");

const synthW = wasm.masterPublicKeyToWalletSyntheticKey(pkW);
const synthN = napi.masterPublicKeyToWalletSyntheticKey(pkN);
eqBytes(synthW, synthN, "masterPublicKeyToWalletSyntheticKey");

const phW = wasm.masterPublicKeyToFirstPuzzleHash(pkW);
const phN = napi.masterPublicKeyToFirstPuzzleHash(pkN);
eqBytes(phW, phN, "masterPublicKeyToFirstPuzzleHash");

const synthSkW = wasm.masterSecretKeyToWalletSyntheticSecretKey(sk);
const synthSkN = napi.masterSecretKeyToWalletSyntheticSecretKey(sk);
eqBytes(synthSkW, synthSkN, "masterSecretKeyToWalletSyntheticSecretKey");

// 2. Address round-trip parity.
const addrW = wasm.puzzleHashToAddress(phW, "xch");
const addrN = napi.puzzleHashToAddress(phN, "xch");
assert.equal(addrW, addrN, "puzzleHashToAddress");
eqBytes(wasm.addressToPuzzleHash(addrW), napi.addressToPuzzleHash(addrN), "addressToPuzzleHash");

// 3. Delegated puzzle parity.
const adminW = wasm.adminDelegatedPuzzleFromKey(synthW);
const adminN = napi.adminDelegatedPuzzleFromKey(synthN);
eqBytes(adminW.adminInnerPuzzleHash, adminN.adminInnerPuzzleHash, "adminDelegatedPuzzleFromKey");

// 4. PRIMARY: mint_store DIGStore spend bundle parity.
const ownerPh = Buffer.from(phW);
const coin = {
  parentCoinInfo: Buffer.alloc(32, 2),
  puzzleHash: ownerPh,
  amount: 1_000_000_000_000n, // 1 XCH
};
const rootHash = Buffer.alloc(32, 3);

const mintW = wasm.mintStore(synthW, [coin], rootHash, "label", "desc", 42n, null, ownerPh, [adminW], 0n);
const mintN = napi.mintStore(synthN, [coin], rootHash, "label", "desc", 42n, null, ownerPh, [adminN], 0n);

assert.equal(mintW.coinSpends.length, mintN.coinSpends.length, "mint coinSpends length");
for (let i = 0; i < mintW.coinSpends.length; i++) {
  eqBytes(mintW.coinSpends[i].puzzleReveal, mintN.coinSpends[i].puzzleReveal, `coinSpends[${i}].puzzleReveal`);
  eqBytes(mintW.coinSpends[i].solution, mintN.coinSpends[i].solution, `coinSpends[${i}].solution`);
  eqBytes(mintW.coinSpends[i].coin.parentCoinInfo, mintN.coinSpends[i].coin.parentCoinInfo, `coinSpends[${i}].coin.parent`);
  assert.equal(mintW.coinSpends[i].coin.amount, mintN.coinSpends[i].coin.amount, `coinSpends[${i}].coin.amount`);
}
eqBytes(mintW.newStore.launcherId, mintN.newStore.launcherId, "newStore.launcherId");
eqBytes(mintW.newStore.metadata.rootHash, mintN.newStore.metadata.rootHash, "newStore.metadata.rootHash");

// 5. Sign + serialize the mint bundle — ultimate byte-for-byte check.
const sigW = wasm.signCoinSpends(mintW.coinSpends, [synthSkW], true);
const sigN = napi.signCoinSpends(mintN.coinSpends, [synthSkN], true);
eqBytes(sigW, sigN, "signCoinSpends(mint)");

const hexW = wasm.spendBundleToHex({ coinSpends: mintW.coinSpends, aggregatedSignature: sigW });
const hexN = napi.spendBundleToHex({ coinSpends: mintN.coinSpends, aggregatedSignature: sigN });
assert.equal(hexW, hexN, "mint spend bundle hex parity");

// 6. Cost + coin id parity.
assert.equal(wasm.getCost(mintW.coinSpends), napi.getCost(mintN.coinSpends), "getCost");
eqBytes(wasm.getCoinId(coin), napi.getCoinId(coin), "getCoinId");

// 7. meltStore parity (uses the identical newStore from the mint above).
//    The minted store's owner is the synthetic key, so melt with synthW/synthN.
const meltStore = mintN.newStore; // byte-identical to mintW.newStore (asserted above)
const meltW = wasm.meltStore(meltStore, synthW);
const meltN = napi.meltStore(meltStore, synthN);
assert.equal(meltW.length, meltN.length, "meltStore coinSpends length");
for (let i = 0; i < meltW.length; i++) {
  eqBytes(meltW[i].puzzleReveal, meltN[i].puzzleReveal, `melt coinSpends[${i}].puzzleReveal`);
  eqBytes(meltW[i].solution, meltN[i].solution, `melt coinSpends[${i}].solution`);
}

// 8. sendXch parity.
const outputs = [{ puzzleHash: ownerPh, amount: 1n, memos: [] }];
const sendW = wasm.sendXch(synthW, [coin], outputs, 0n);
const sendN = napi.sendXch(synthN, [coin], outputs, 0n);
assert.equal(sendW.length, sendN.length, "sendXch coinSpends length");
for (let i = 0; i < sendW.length; i++) {
  eqBytes(sendW[i].puzzleReveal, sendN[i].puzzleReveal, `sendXch coinSpends[${i}].puzzleReveal`);
  eqBytes(sendW[i].solution, sendN[i].solution, `sendXch coinSpends[${i}].solution`);
}

// 9. signMessage / verifySignedMessage parity.
const msg = Buffer.from("hello datalayer", "utf8");
const msgSigW = wasm.signMessage(msg, synthSkW);
const msgSigN = napi.signMessage(msg, synthSkN);
eqBytes(msgSigW, msgSigN, "signMessage");
// public key corresponding to the synthetic secret key:
const synthPkW = wasm.secretKeyToPublicKey(synthSkW);
const synthPkN = napi.secretKeyToPublicKey(synthSkN);
assert.equal(wasm.verifySignedMessage(msgSigW, synthPkW, msg), true, "wasm verifySignedMessage");
assert.equal(napi.verifySignedMessage(msgSigN, synthPkN, msg), true, "napi verifySignedMessage");
assert.equal(
  wasm.verifySignedMessage(msgSigW, synthPkW, msg),
  napi.verifySignedMessage(msgSigN, synthPkN, msg),
  "verifySignedMessage parity"
);

console.log("All parity checks passed.");
