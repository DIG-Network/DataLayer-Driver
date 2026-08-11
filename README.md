# DataLayer-Driver

Native Chia DataLayer Driver for storing and retrieving data in Chia blockchain.

This project provides both Rust library APIs and Node.js bindings for interacting with Chia's DataLayer.

A WebAssembly build is also available as [`@dignetwork/datalayer-driver-wasm`](./wasm/README.md) for browser and bundler environments (webpack, Vite, Next.js, esbuild). The WASM package mirrors the offline subset of the NAPI interface — no networking, no `Peer`/`Tls` — making it ideal for building and signing DIGStore spend bundles client-side without a full node. See [`wasm/README.md`](./wasm/README.md) for installation, usage, and a worked example.

## Installation

### As a Rust Crate

Add to your `Cargo.toml`:

```toml
[dependencies]
datalayer-driver = "0.1.0"
```

### As a Node.js Package

```bash
npm install datalayer-driver
# or
yarn add datalayer-driver
```

## Rust Usage

```rust
use datalayer_driver::{
    Peer, Tls, mint_store, select_coins, sign_coin_spends,
    master_public_key_to_wallet_synthetic_key, TargetNetwork
};
use chia::bls::{SecretKey, PublicKey};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Chia node
    let cert_path = "~/.chia/mainnet/config/ssl/wallet/wallet_node.crt";
    let key_path = "~/.chia/mainnet/config/ssl/wallet/wallet_node.key";
    let tls = Tls::new(cert_path, key_path)?;
    let peer = Peer::new("127.0.0.1:8444", "mainnet", &tls).await?;
    
    // Example: Mint a new data store
    let master_sk = SecretKey::from_bytes(&your_key_bytes)?;
    let master_pk = master_sk.public_key();
    let wallet_pk = master_public_key_to_wallet_synthetic_key(&master_pk);
    
    // Select coins for the transaction
    let coins = peer.get_all_unspent_coins(
        wallet_pk.to_puzzle_hash(),
        None,
        genesis_challenge
    ).await?;
    
    let selected_coins = select_coins(coins.coins, amount_needed)?;
    
    // Mint the store
    let response = mint_store(
        wallet_pk,
        selected_coins,
        root_hash,
        Some("My Store".to_string()),
        Some("Description".to_string()),
        None,
        owner_puzzle_hash,
        vec![],
        fee
    )?;
    
    // Sign and broadcast
    let signature = sign_coin_spends(
        response.coin_spends,
        vec![master_sk],
        TargetNetwork::Mainnet
    )?;
    
    Ok(())
}
```

## Node.js Usage

A collection of functions that can be used to interact with datastores on the Chia blockchain.

This library offers the following functions:

- wallet: `selectCoins`, `addFee`, `signCoinSpends`, `sendXch`
- drivers: `mintStore`, `adminDelegatedPuzzleFromKey`, `writerDelegatedPuzzleFromKey`, `oracleDelegatedPuzzle`, `oracleSpend`, `updateStoreMetadata`, `updateStoreOwnership`, `meltStore`, `getCost`, `createServerCoin`, `lookupAndSpendServerCoins`
- utils: `getCoinId`, `masterPublicKeyToWalletSyntheticKey`, `masterPublicKeyToFirstPuzzleHash`, `masterSecretKeyToWalletSyntheticSecretKey`, `secretKeyToPublicKey`, `puzzleHashToAddress`, `addressToPuzzleHash`, `newLineageProof`, `newEveProof`, `signMessage`, `verifySignedMessage`, `syntheticKeyToPuzzleHash`, `morphLauncherId`, `getMainnetGenesisChallenge`, `getTestnet11GenesisChallenge`.

The `Peer` class also exposes the following methods: `getAllUnspentCoins`, `syncStore`, `syncStoreFromLauncherId`, `broadcastSpend`, `isCoinSpent`, `getHeaderHash`, `getFeeEstimate`, `getPeak`, `getHintedCoinStates`, `fetchServerCoin`, `getStoreCreationHeight`, `lookUpPossibleLaunchers`, `waitForCoinToBeSpent`.

Note that all functions come with detailed JSDoc comments.

## Example

This example will assume that you want to use a server-side wallet to mint & update the store. You can also send unsigned coin spends to the client and ask them to sign via a wallet such as Goby, but this scenario will not be covered here.

First, you'll need to generate new keys for the server wallet. You can do that by running `chia keys generate`, followed by `chia keys show --show-mnemonic-seed -f [FINGERPRINT]` (fingerprint is present in the output of the first command). The server signing transactions will need the `Master private key (m)` printed by the second command. The server wallet will be single-address, meaning that you can only send XCH to `First wallet address` to fund it (i.e., don't use `get_address` to obtain the wallet address!).

The synthetic public and private keys corresponding to the first address can simply be obtain as follows:

```js
export const getPublicSyntheticKey = (): Buffer => {
  const master_sk = Buffer.from(process.env.SERVER_SK as string, 'hex');
  const master_pk = secretKeyToPublicKey(master_sk);

  return masterPublicKeyToWalletSyntheticKey(master_pk);
}

export const getPrivateSyntheticKey = (): Buffer => {
  const master_sk = Buffer.from(process.env.SERVER_SK as string, 'hex');

  return masterSecretKeyToWalletSyntheticSecretKey(master_sk);
}
```

To get the wallet's address, you can simply do:

```js
export const getServerPuzzleHash = (): Buffer => {
  const master_sk = Buffer.from(process.env.SERVER_SK as string, 'hex');
  const master_pk = secretKeyToPublicKey(master_sk);

  return masterPublicKeyToFirstPuzzleHash(master_pk);
}

// other part of the code
const ph = getServerPuzzleHash();
const address = puzzleHashToAddress(ph, NETWORK_PREFIX);
```

Where `NETWORK_PREFIX` is `xch` for mainnet and `txch` for testnet.

To 'talk' with the wallet, you will need to initialize a `Peer` object like in the example below:

```js
const tls = new Tls(CHIA_CRT, CHIA_KEY);
const peer = await Peer.new("127.0.0.1:58444", "testnet11", tls);
```

The example above connects to a `tesntet11` full node. Note that `CHIA_CRT` is usually `~/.chia/mainnet/config/ssl/wallet/wallet_node.crt` and `CHIA_KEY` is usually `~/.chia/mainnet/config/ssl/wallet/wallet_node.key`. For mainnet, the port is usually `8444`, and the network id is `mainnet`.

Making any transaction will require finding available (unspent) coins in the server wallet and selecting them before calling any drivers:

```js
const ph = getServerPuzzleHash();
const coinsResp = await peer.getAllUnspentCoins(
  ph,
  MIN_HEIGHT,
  MIN_HEIGHT_HEADER_HASH
);
const coins = selectCoins(coinsResp.coins, feeBigInt + BigInt(1));
```

You can speed up coin lookup by setting `MIN_HEIGHT` and `MIN_HEIGHT_HEADER_HASH` to point to a block just before wallet creation (or before the first fund tx was confirmed). Alternatively, you can set them to `null` and the network's genesis challenge. When selecting coins, make sure to include the fee in the total amount.

The next step is to generate coin spends using drivers:

```js
const serverKey = getPublicSyntheticKey();
const successResponse = await mintStore(
  getPublicSyntheticKey(),
  coins,
  rootHash,
  label,
  description,
  sizeBigInt,
  ownerPuzzleHash,
  [
    adminDelegatedPuzzleFromKey(serverKey),
    writerDelegatedPuzzleFromKey(serverKey),
    oracleDelegatedPuzzle(ownerPuzzleHash, oracleFeeBigInt),
  ],
  feeBigInt
);
```

The code above is used to mint stores. Note that a success response not only contains unsigned coin spends, but also returns a new `DataStore` object that can be used to sync or spend the store in the future. Note that some drivers will not require coins, only the information of the store being spent:

```js
const resp = meltStore(parseDataStore(info), ownerPublicKey);
```

In that case, the 'basic' transaction only spends the store - to add fees, you'll need to call `addFee` and make sure to include the returned coin spends in the final bundle:

```js
const resp = await addFee(serverKey, selectedCoins, coin_ids, BigInt(fee));
```

Before broadcasting transactions, you'll usually need to sign the coin spends. The `signCoinSpends` function was created for that purpose:

```js
const sig = signCoinSpends(coinSpends, [getPrivateSyntheticKey()], true);
```

Note that `true` indicates that the transaction is being signed for testnet11, while `false` indicates mainnet.

Broadcasting a bundle is as easy as:

```js
const err = await peer.broadcastSpend(coinSpends, [sig]);
```

To confirm the transaction, you can just confirm that the datastore coin was spent on-chain:

```js
const confirmed = await peer.isCoinSpent(
  getCoinId(info.coin),
  MIN_HEIGHT,
  MIN_HEIGHT_HEADER_HASH
);
```

## More Examples

### Transferring the Store to a New Owner

Suppose `info` is holding the current store's information and you want to transfer it to `newOwnerPuzzleHash`. You can do this as follows:

```js
const {coinSpends, newInfo} = updateStoreOwnership(
    info,
    newOwnerPuzzleHash,
    info.delegatedPuzzles,
    currentOwnerPublicKey,
    null,
 );

/* optionally add a fee via 'addFee' - you'll also need to get 'server_sig' via 'signCoinSpends' */

const sig = /* fetch from user */;
/* /\ Goby in browser: await window.chia.request({ method: 'signCoinSpends', params: { coinSpends } }); */
/* or use 'signCoinSpends' if spending as admin (i.e., you have access to the private synthetic key) */

/* broadcast spend */
const err = await peer.broadcastSpend(
    coinSpends,
    [sig /* add 'server_sig' if adding fee */ ]
  );
// check that err === "" <-> successful mempool inclusion

/* wait for tx to be confirmed */
var confirmed = await peer.isCoinSpent(getCoinId(info.coin), MIN_HEIGHT, MIN_HEIGHT_HEADER_HASH);
while(!confirmed) {
    confirmed = await peer.isCoinSpent(getCoinId(info.coin), MIN_HEIGHT, MIN_HEIGHT_HEADER_HASH);
}
```

Note that, when changing ownership, either the owner's or an admin's synthetic key can be provided. Admins can change delegated puzzles, so you can use this method to spend the store as an admin when you want to modify the allowed delegated puzzles. When changing the owner's puzzle hash, however, you need to provide the current owner's synthetic key - only an owner may transfer store ownership.

Waiting for transactions is usually more complicated than the snippet above - mempool items are sometimes kicked out when transactions with higher fees can fill the mempool, meaning that the `while` loop would run infinitely.

### Syncing a Store & Verifying Ownership

To sync a store, you'll first need a peer. Recall that we've previously initialized a peer as:

```js
const CHIA_CRT = path.join(
  os.homedir(),
  ".chia/mainnet/config/ssl/wallet/wallet_node.crt"
);
const CHIA_KEY = path.join(
  os.homedir(),
  ".chia/mainnet/config/ssl/wallet/wallet_node.key"
);
// ...
const tls = new Tls(CHIA_CRT, CHIA_KEY);
const peer = await Peer.new("127.0.0.1:58444", "testnet11", tls);
```

To sync, you'll also need two other values, `MIN_HEIGHT` and `MIN_HEIGHT_HEADER_HASH`. These variables represent information relating to the block you want to start syncing from - higher heights lead to faster sync times. If you wish to sync from genesis, use a height of `null` and a header hash equal to the network's genesis challenge.

Syncing a store using its launcher id is as easy as:

```js
const { latestInfo, latestHeight } = await peer.syncStoreFromLauncherId(
  launcherId,
  MIN_HEIGHT,
  MIN_HEIGHT_HEADER_HASH,
  false
);
```

If you already have a `DataStore` object, you can use it to 'bootstrap' the syncing process and minimize the time it takes to fetch the latest info:

```js
const { latestInfo, latestHeight } = await peer.syncStore(
  oldStoreInfo,
  MIN_HEIGHT,
  MIN_HEIGHT_HEADER_HASH,
  false
);
```

With the latest store info in the `latestInfo` variable, checking that the current store owner is `myPuzzleHash` can be done as follows:

```js
if (latestInfo.ownerPuzzleHash === myPuzzleHash) {
  doSomething();
}
```

## License

This project is licensed under the MIT License. See the [LICENSE](https://github.com/DIG-Network/DataLayer-Driver/blob/HEAD/LICENSE) file for details.
