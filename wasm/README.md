# @dignetwork/datalayer-driver-wasm

WebAssembly bindings for the Chia DataLayer driver, mirroring the **offline subset** of the NAPI `@dignetwork/datalayer-driver` interface for browser/bundler use.

## Scope

This package is **offline only** — it contains no networking. There is no `Peer`, `Tls`, `connect`, `sync`, or `broadcast`. Reading coins from the chain and broadcasting the resulting spend bundle are the consumer's responsibility (e.g. via a wallet extension such as Goby, or via a full-node RPC in JavaScript).

The package exists to build and sign **DIGStore (DataStore) spend bundles client-side**, inside a browser or any bundler-based environment, without shipping a full node.

## Key handling — read before using the signing functions

The bundle *builders* (`mintStore`, `oracleSpend`, `meltStore`, `updateStoreMetadata`,
`updateStoreOwnership`, `sendXch`, `addFee`, `createServerCoin`) are pure construction: they take
public keys and puzzle hashes only, and return an unsigned set of coin spends. Prefer handing those
spends to an external signer — a wallet extension, or the user's node — and never giving this module
a private key at all.

Four functions do accept raw private key bytes, for parity with the NAPI package:
`signCoinSpends`, `signMessage`, `secretKeyToPublicKey`, and
`masterSecretKeyToWalletSyntheticSecretKey`. They sign in-process, inside the page. A page that
calls them has the user's key in JavaScript memory, where any script on the origin — including a
compromised dependency — can reach it. There is no seed or mnemonic entry point, and nothing is
transmitted anywhere by this module, but the exposure is real and it is the caller's to manage.

Treat in-page signing as a last resort for environments that control their own key material end to
end; for user-facing wallets, build the bundle here and sign it elsewhere.

## Installation

```bash
npm install @dignetwork/datalayer-driver-wasm
```

## Startup

Call `init()` once at startup. It installs the Rust panic hook so that panics surface as readable JavaScript errors rather than cryptic traps.

```js
import init from "@dignetwork/datalayer-driver-wasm";
await init();
```

> **Note:** When built with `--target bundler` (see below), many bundlers auto-run the default export initializer. Even so, calling `init()` explicitly is safe and recommended — it is idempotent after the first call.

## Build target

This package is built with:

```
wasm-pack --target bundler
```

The bundler target works out of the box with **webpack**, **Vite**, **Next.js**, and **esbuild**. If you need plain Node.js (without a bundler), build the `nodejs` target yourself from the `wasm/` crate in the source repo:

```bash
wasm-pack build . --target nodejs --out-dir pkg-node
```

## Primary use case — build and sign a DIGStore mint bundle

```js
import init, * as dl from "@dignetwork/datalayer-driver-wasm";

// Call init() to install the panic hook (safe to call multiple times).
await dl.init();

// ---------------------------------------------------------------------------
// 1. Derive keys from the master secret key (32-byte Uint8Array).
// ---------------------------------------------------------------------------
const sk = /* 32-byte Uint8Array — your master secret key */;
const pk             = dl.secretKeyToPublicKey(sk);
const syntheticKey   = dl.masterPublicKeyToWalletSyntheticKey(pk);
const syntheticSk    = dl.masterSecretKeyToWalletSyntheticSecretKey(sk);
const ownerPuzzleHash = dl.masterPublicKeyToFirstPuzzleHash(pk);

// ---------------------------------------------------------------------------
// 2. Fetch coins from your chain source (wallet RPC, Goby, etc.) and select.
//    All `amount` fields are bigint; all byte values are Uint8Array.
// ---------------------------------------------------------------------------
const allCoins = /* Coin[] fetched by your app */;
const selected  = dl.selectCoins(allCoins, 1n /* mojos needed + fee */);

// ---------------------------------------------------------------------------
// 3. Build the mint spend bundle.
// ---------------------------------------------------------------------------
const rootHash = /* 32-byte Uint8Array — SHA-256 root of the data tree */;

const mint = dl.mintStore(
  syntheticKey,
  selected,
  rootHash,
  "my-store",          // label (string | undefined)
  "description",       // description (string | undefined)
  undefined,           // bytes (bigint | undefined)
  undefined,           // sizeProof (Uint8Array | undefined)
  ownerPuzzleHash,
  [dl.adminDelegatedPuzzleFromKey(syntheticKey)],
  0n                   // fee in mojos
);

// ---------------------------------------------------------------------------
// 4. Sign and serialize.
// ---------------------------------------------------------------------------
const sig = dl.signCoinSpends(
  mint.coinSpends,
  [syntheticSk],
  false  // false = mainnet, true = testnet11
);

const bundleHex = dl.spendBundleToHex({
  coinSpends: mint.coinSpends,
  aggregatedSignature: sig,
});

// Broadcast `bundleHex` via your full-node RPC or wallet extension.
```

## Type notes

- All byte values (`Uint8Array`) — including keys, puzzle hashes, root hashes, and coin fields — are plain `Uint8Array`.
- All amounts (`bigint`) — including `amount`, `fee`, and `bytes` — use JavaScript's native `bigint` type.

These conventions match the NAPI `@dignetwork/datalayer-driver` package exactly, so code that runs offline is portable between environments.

## Available functions

| Category | Functions |
|---|---|
| Key derivation | `secretKeyToPublicKey`, `masterPublicKeyToWalletSyntheticKey`, `masterPublicKeyToFirstPuzzleHash`, `masterSecretKeyToWalletSyntheticSecretKey`, `syntheticKeyToPuzzleHash` |
| Addresses | `puzzleHashToAddress`, `addressToPuzzleHash` |
| Delegated puzzles | `adminDelegatedPuzzleFromKey`, `writerDelegatedPuzzleFromKey` |
| Proofs / IDs | `newLineageProof`, `newEveProof`, `getCoinId`, `morphLauncherId` |
| Genesis challenges | `getMainnetGenesisChallenge`, `getTestnet11GenesisChallenge` |
| DIGStore builders | `mintStore`, `oracleSpend`, `meltStore`, `updateStoreMetadata`, `updateStoreOwnership` |
| Signing / verification | `signCoinSpends`, `signMessage`, `verifySignedMessage` |
| Serialization | `spendBundleToHex`, `hexSpendBundleToCoinSpends` |
| Coin selection / fees | `selectCoins`, `addFee`, `getCost` |
| Transfers | `sendXch` |
| Server coins | `createServerCoin` |

Full TypeScript types are in `datalayer-driver-wasm.d.ts` (published with the package).

## Parity guarantee

This package is generated from the `wasm/` crate in the [DataLayer-Driver](https://github.com/DIG-Network/DataLayer-Driver) repository. Its offline functions are validated **byte-for-byte** against the NAPI package by a parity test that runs in CI on every commit. If a result differs between the WASM and NAPI implementations the CI build fails.
