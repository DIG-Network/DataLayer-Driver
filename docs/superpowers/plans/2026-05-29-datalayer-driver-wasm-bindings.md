# DataLayer-Driver WASM Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add WebAssembly bindings to `DataLayer-Driver` that mirror the offline (non-networking) NAPI interface — primarily the ability to build and sign DIGStore (DataStore) spend bundles in the browser — and publish them as `@dignetwork/datalayer-driver-wasm` via CI.

**Architecture:** A `native` cargo feature (default-on) is added to the core `datalayer-driver` crate, gating all networking (tokio sockets, native TLS, the `async_api` module, every `&Peer` async fn). A new `wasm/` workspace member depends on the core crate with `default-features = false`, exposing the offline functions via `wasm-bindgen`. Scalar params stay native (`Uint8Array`/`bigint`); struct/array params cross the boundary via `serde-wasm-bindgen` with a BigInt-configured serializer. Correctness is proven by a Node parity test that asserts the WASM output byte-for-byte equals the existing NAPI output.

**Tech Stack:** Rust, `wasm-bindgen` 0.2, `serde-wasm-bindgen` 0.6, `serde_bytes`, `getrandom`, `wasm-pack` (`--target bundler`), `chia` 0.26 / `chia-wallet-sdk` 0.30, GitHub Actions, npm.

---

## Spec

`docs/superpowers/specs/2026-05-29-datalayer-driver-wasm-bindings-design.md`

## File Structure

**Modified (core crate — `native` feature gating):**
- `Cargo.toml` — workspace members + `native` feature on core crate
- `src/lib.rs` — gate `async_api` module + `pub use ...Peer`; keep offline wrappers
- `src/wallet.rs` — gate every `&Peer` async fn + tokio/Peer imports
- `src/dig_coin.rs` — gate `&Peer`/async items
- `src/dig_collateral_coin.rs` — gate `&Peer`/async items

**Created (wasm crate):**
- `wasm/Cargo.toml` — `crate-type = ["cdylib","rlib"]`, wasm deps
- `wasm/src/lib.rs` — `init()` + all `#[wasm_bindgen]` exports (analog of `napi/src/napi_lib.rs`)
- `wasm/src/types.rs` — serde boundary structs + native conversions (analog of `napi/src/js.rs` + `napi/src/conversions.rs`)
- `wasm/scripts/patch-pkg.mjs` — rewrite generated `pkg/package.json` to the scoped name + inject typed `.d.ts`
- `wasm/types/datalayer-driver-wasm.d.ts` — hand-authored TypeScript types mirroring the NAPI `index.d.ts` shapes
- `wasm/package.json` — dev scripts to drive `wasm-pack` + patch + test
- `wasm/tests/parity.mjs` — Node differential test: WASM output === NAPI output
- `wasm/README.md` — usage

**Modified (CI):**
- `.github/workflows/CI.yml` — add `build-wasm` job + `publish-wasm-npm` job

---

## Task 1: Gate networking behind a `native` feature (core crate compiles to wasm32)

This is the highest-risk task; do it first. Goal: `cargo build -p datalayer-driver --target wasm32-unknown-unknown --no-default-features` succeeds, while the default (native) build is byte-for-byte unchanged for the napi crate.

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs:21-28`, `src/lib.rs:340` (async_api module)
- Modify: `src/wallet.rs` (Peer fns + imports)
- Modify: `src/dig_coin.rs`, `src/dig_collateral_coin.rs`

- [ ] **Step 1: Add the wasm target locally**

Run:
```powershell
rustup target add wasm32-unknown-unknown
```
Expected: `info: component 'rust-std' for target 'wasm32-unknown-unknown' is up to date` (or installs it).

- [ ] **Step 2: Add the `native` feature and make tokio optional in `Cargo.toml`**

In `Cargo.toml`, change the `chia-wallet-sdk` and `tokio` lines and add a `[features]` block. Replace:
```toml
tokio = "1.39.3"
chia-wallet-sdk = { version = "0.30.0", features = ["chip-0035", "native-tls", "peer-simulator", "action-layer"] }
```
with:
```toml
tokio = { version = "1.39.3", optional = true }
chia-wallet-sdk = { version = "0.30.0", default-features = false, features = ["chip-0035", "action-layer"] }
```
and add, immediately after the `[package]`/before `[dependencies]` is fine, but place it after `[dependencies]`:
```toml
[features]
default = ["native"]
native = [
    "dep:tokio",
    "chia-wallet-sdk/native-tls",
    "chia-wallet-sdk/peer-simulator",
]
```

Note: `chia-wallet-sdk` previously did not set `default-features = false`; verify its default features are not silently required by the offline path. If the build in Step 7 fails on a missing default feature, re-add the needed one to the unconditional `features = [...]` list (NOT under `native`).

- [ ] **Step 3: Gate the `async_api` module and `Peer` re-export in `src/lib.rs`**

In `src/lib.rs`, change line 21:
```rust
pub use chia_wallet_sdk::client::Peer;
```
to:
```rust
#[cfg(feature = "native")]
pub use chia_wallet_sdk::client::Peer;
```
Change line 28:
```rust
pub use async_api::{connect_peer, connect_random, create_tls_connector, NetworkType};
```
to:
```rust
#[cfg(feature = "native")]
pub use async_api::{connect_peer, connect_random, create_tls_connector, NetworkType};
```
Add `#[cfg(feature = "native")]` directly above the `pub mod async_api {` declaration (line 340):
```rust
/// Async functions for blockchain interaction (Rust API versions)
#[cfg(feature = "native")]
pub mod async_api {
```

`NetworkType` is referenced by `wallet.rs` (`use crate::{NetworkType, ...}`) — it lives in `async_api`. Since the offline `sign_coin_spends` path uses `wallet::TargetNetwork` (not `NetworkType`), confirm no offline function imports `NetworkType`. If an offline path needs it, move the `NetworkType` enum out of `async_api` to the crate root (ungated) and re-export. Resolve when Step 7 surfaces it.

- [ ] **Step 4: Gate `&Peer` functions and native-only imports in `src/wallet.rs`**

At the top of `src/wallet.rs`, gate the networking imports. Change line 31:
```rust
use chia_wallet_sdk::client::Peer;
```
to:
```rust
#[cfg(feature = "native")]
use chia_wallet_sdk::client::Peer;
```
The `use crate::{NetworkType, UnspentCoinStates};` (line 11) and any `tokio`/`futures_util` imports used only by `&Peer` fns must also be gated with `#[cfg(feature = "native")]`. Add `#[cfg(feature = "native")]` immediately above EACH of these `pub async fn` definitions (they all take `peer: &Peer`):
`get_unspent_coin_states_by_hint` (60), `get_unspent_coin_states` (72), `spend_xch_server_coins` (250), `fetch_xch_server_coin` (341), `sync_store` (482), `sync_store_using_launcher_id` (580), `get_store_creation_height` (661), `broadcast_spend_bundle` (1028), `get_header_hash` (1037), `get_fee_estimate` (1047), `is_coin_spent` (1079), `look_up_possible_launchers` (1152), `subscribe_to_coin_states` (1183), `unsubscribe_from_coin_states` (1202), `mint_nft` (1230), `generate_did_proof` (1331), `generate_did_proof_from_chain` (1389), `resolve_did_string_and_generate_proof` (1517).

Example:
```rust
#[cfg(feature = "native")]
pub async fn sync_store(
    peer: &Peer,
    ...
```
Leave the offline functions (`mint_store`, `send_xch`, `select_coins`, `sign_coin_spends`, `sign_message`, `verify_signature`, `get_cost`, `add_fee`, `oracle_spend`, `update_store_metadata`, `update_store_ownership`, `melt_store`, `create_server_coin`, `create_simple_did`, `generate_did_proof_manual`) ungated.

- [ ] **Step 5: Gate `&Peer`/async items in `src/dig_coin.rs` and `src/dig_collateral_coin.rs`**

Run to list the exact items needing gates:
```powershell
Select-String -Path src\dig_coin.rs, src\dig_collateral_coin.rs -Pattern 'Peer|async|tokio'
```
For each `pub async fn ...(peer: &Peer, ...)` and each `use ...Peer`/`use ...tokio`, add `#[cfg(feature = "native")]` above it (same pattern as Step 4). Keep any pure (non-Peer, non-async) helpers ungated.

- [ ] **Step 6: Verify the native build still compiles (no regression)**

Run:
```powershell
cargo build -p datalayer-driver
cargo build -p datalayer-driver-napi
```
Expected: both succeed. The napi crate uses default features, so `native` is on and behavior is unchanged.

- [ ] **Step 7: Verify the offline build compiles to wasm32**

Run:
```powershell
cargo build -p datalayer-driver --target wasm32-unknown-unknown --no-default-features
```
Expected: success. If it fails, the error names either (a) a still-ungated networking item — add `#[cfg(feature = "native")]`; (b) a missing chia-wallet-sdk feature on the offline path — add it to the unconditional `features`; or (c) `getrandom` needing a wasm backend — that is resolved in Task 2, so a getrandom-related error here is expected and acceptable to defer. Re-run until only getrandom-class errors (if any) remain.

- [ ] **Step 8: Commit**

```powershell
git add Cargo.toml src/lib.rs src/wallet.rs src/dig_coin.rs src/dig_collateral_coin.rs
git commit -m "feat(core): gate networking behind native feature for wasm32 compat"
```

---

## Task 2: Resolve the `getrandom` wasm backend

`chia`/`rand` pull `getrandom`. On `wasm32-unknown-unknown` it needs an explicit JS backend or it panics at runtime. The required feature differs between getrandom 0.2 (`js`) and 0.3 (`wasm_js` + a `RUSTFLAGS` cfg).

**Files:**
- Inspect: `Cargo.lock`
- (Used in Task 3's `wasm/Cargo.toml`)

- [ ] **Step 1: Determine which getrandom version is in the tree**

Run:
```powershell
cargo tree -i getrandom --target wasm32-unknown-unknown --no-default-features -p datalayer-driver
```
Expected: prints `getrandom vX.Y.Z` and what depends on it. Record the major.minor (0.2 or 0.3). If BOTH appear, note both.

- [ ] **Step 2: Record the resolution for Task 3**

Write the decision into `wasm/GETRANDOM.md` (create it):
- If **0.2** is present: the wasm crate adds `getrandom = { version = "0.2", features = ["js"] }`. No cargo config needed.
- If **0.3** is present: the wasm crate adds `getrandom = { version = "0.3", features = ["wasm_js"] }` AND a `.cargo/config.toml` (created in Task 3) with:
  ```toml
  [target.wasm32-unknown-unknown]
  rustflags = ['--cfg', 'getrandom_backend="wasm_js"']
  ```
- If **both**: add the matching dependency line for **each** version present.

```powershell
git add wasm/GETRANDOM.md
git commit -m "docs(wasm): record getrandom backend resolution"
```

---

## Task 3: Scaffold the `wasm` crate

**Files:**
- Modify: `Cargo.toml` (workspace members)
- Create: `wasm/Cargo.toml`, `wasm/src/lib.rs`, `wasm/.cargo/config.toml` (only if getrandom 0.3)

- [ ] **Step 1: Add `wasm` to workspace members**

In `Cargo.toml`, change:
```toml
members = [".", "napi"]
```
to:
```toml
members = [".", "napi", "wasm"]
```

- [ ] **Step 2: Create `wasm/Cargo.toml`**

Use the getrandom line chosen in Task 2 (this template assumes 0.2; swap if Task 2 said 0.3):
```toml
[package]
name = "datalayer-driver-wasm"
version = "3.0.0"
edition = "2021"
license = "MIT"
publish = false
description = "WebAssembly bindings for the Chia DataLayer driver (offline DIGStore spend-bundle construction)."
repository = "https://github.com/DIG-Network/DataLayer-Driver"

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["console-panic-hook"]
console-panic-hook = ["dep:console_error_panic_hook"]

[dependencies]
datalayer-driver = { path = "..", default-features = false }
wasm-bindgen = "0.2"
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde_bytes = "0.11"
serde-wasm-bindgen = "0.6"
console_error_panic_hook = { version = "0.1", optional = true }
hex = "0.4"
# getrandom: see wasm/GETRANDOM.md — set to the version present in the tree
getrandom = { version = "0.2", features = ["js"] }

# Direct chia type access for conversions (versions match the core crate)
chia = "0.26.0"
chia-wallet-sdk = { version = "0.30.0", default-features = false, features = ["chip-0035", "action-layer"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

- [ ] **Step 3: Create `.cargo/config.toml` ONLY if Task 2 found getrandom 0.3**

If (and only if) getrandom 0.3 is in the tree, create `wasm/.cargo/config.toml`:
```toml
[target.wasm32-unknown-unknown]
rustflags = ['--cfg', 'getrandom_backend="wasm_js"']
```
Otherwise skip this step.

- [ ] **Step 4: Create a minimal `wasm/src/lib.rs` with just `init`**

```rust
//! WebAssembly bindings for the Chia DataLayer driver.
//!
//! Mirrors the offline subset of the NAPI interface. Networking
//! (Peer/Tls) is intentionally absent — WASM has no native sockets.

use wasm_bindgen::prelude::*;

mod types;

/// Initialise the module. Call once at startup. Installs a panic hook
/// (when the `console-panic-hook` feature is on) so Rust panics surface
/// in the JS console instead of an opaque `unreachable`.
#[wasm_bindgen]
pub fn init() {
    #[cfg(feature = "console-panic-hook")]
    console_error_panic_hook::set_once();
}
```

Create an empty stub `wasm/src/types.rs` for now:
```rust
//! Serde boundary structs and conversions to/from native datalayer-driver types.
```

- [ ] **Step 5: Install wasm-pack and verify the crate builds to wasm**

Run:
```powershell
cargo install wasm-pack --locked
wasm-pack build wasm --target bundler --dev
```
Expected: succeeds, producing `wasm/pkg/` containing `datalayer_driver_wasm_bg.wasm`, `datalayer_driver_wasm.js`, `datalayer_driver_wasm.d.ts`, `package.json`. If it fails on getrandom, apply the Task 2 / Step 3 fix and re-run.

- [ ] **Step 6: Commit**

```powershell
git add Cargo.toml wasm/Cargo.toml wasm/src/lib.rs wasm/src/types.rs
# include wasm/.cargo/config.toml only if created
git commit -m "feat(wasm): scaffold wasm-bindgen crate with init()"
```

---

## Task 4: Conversion layer — boundary structs and native conversions

Mirror `napi/src/js.rs` + the DataStore family in `napi/src/napi_lib.rs`. All structs derive `Serialize`/`Deserialize`, use `#[serde(rename_all = "camelCase")]` (matching the camelCase NAPI JS shape), and use `#[serde(with = "serde_bytes")]` for byte fields so they become `Uint8Array`. Amounts are `u64` and serialize as `bigint` via the configured serializer (Step 1).

**Files:**
- Modify: `wasm/src/types.rs`

- [ ] **Step 1: Add serializer helpers and byte/key conversion utilities**

Write into `wasm/src/types.rs`:
```rust
use chia::bls::{PublicKey, SecretKey, Signature};
use chia::protocol::{Bytes, Bytes32, Program};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::JsValue;

/// Serialize a Rust value into a JS value, encoding u64/i64 as BigInt
/// (DataLayer mojo amounts exceed 2^53, so the default lossy f64 path
/// is unacceptable).
pub fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let ser = serde_wasm_bindgen::Serializer::new().serialize_large_number_types_as_bigints(true);
    value
        .serialize(&ser)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Deserialize a JS value into a Rust value. Accepts both JS number and
/// BigInt for integer fields.
pub fn from_js<T: DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))
}

pub fn bytes32(buf: &[u8]) -> Result<Bytes32, JsValue> {
    Bytes32::try_from(buf.to_vec()).map_err(|_| JsValue::from_str("expected 32-byte value"))
}

pub fn public_key(buf: &[u8]) -> Result<PublicKey, JsValue> {
    let arr = <[u8; 48]>::try_from(buf).map_err(|_| JsValue::from_str("expected 48-byte public key"))?;
    PublicKey::from_bytes(&arr).map_err(|_| JsValue::from_str("invalid public key"))
}

pub fn secret_key(buf: &[u8]) -> Result<SecretKey, JsValue> {
    let arr = <[u8; 32]>::try_from(buf).map_err(|_| JsValue::from_str("expected 32-byte secret key"))?;
    SecretKey::from_bytes(&arr).map_err(|_| JsValue::from_str("invalid secret key"))
}

pub fn signature(buf: &[u8]) -> Result<Signature, JsValue> {
    let arr = <[u8; 96]>::try_from(buf).map_err(|_| JsValue::from_str("expected 96-byte signature"))?;
    Signature::from_bytes(&arr).map_err(|_| JsValue::from_str("invalid signature"))
}
```

- [ ] **Step 2: Add the coin/proof boundary structs and conversions**

Append to `wasm/src/types.rs`:
```rust
use chia::protocol::{Coin as RustCoin, CoinSpend as RustCoinSpend};
use chia::puzzles::{EveProof as RustEveProof, LineageProof as RustLineageProof, Proof as RustProof};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coin {
    #[serde(with = "serde_bytes")]
    pub parent_coin_info: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub puzzle_hash: Vec<u8>,
    pub amount: u64,
}

impl Coin {
    pub fn to_native(&self) -> Result<RustCoin, JsValue> {
        Ok(RustCoin {
            parent_coin_info: bytes32(&self.parent_coin_info)?,
            puzzle_hash: bytes32(&self.puzzle_hash)?,
            amount: self.amount,
        })
    }
    pub fn from_native(c: &RustCoin) -> Self {
        Coin {
            parent_coin_info: c.parent_coin_info.to_vec(),
            puzzle_hash: c.puzzle_hash.to_vec(),
            amount: c.amount,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinSpend {
    pub coin: Coin,
    #[serde(with = "serde_bytes")]
    pub puzzle_reveal: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub solution: Vec<u8>,
}

impl CoinSpend {
    pub fn to_native(&self) -> Result<RustCoinSpend, JsValue> {
        Ok(RustCoinSpend {
            coin: self.coin.to_native()?,
            puzzle_reveal: Program::from(self.puzzle_reveal.clone()),
            solution: Program::from(self.solution.clone()),
        })
    }
    pub fn from_native(cs: &RustCoinSpend) -> Self {
        CoinSpend {
            coin: Coin::from_native(&cs.coin),
            puzzle_reveal: cs.puzzle_reveal.to_vec(),
            solution: cs.solution.to_vec(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineageProof {
    #[serde(with = "serde_bytes")]
    pub parent_parent_coin_info: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub parent_inner_puzzle_hash: Vec<u8>,
    pub parent_amount: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EveProof {
    #[serde(with = "serde_bytes")]
    pub parent_parent_coin_info: Vec<u8>,
    pub parent_amount: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proof {
    pub lineage_proof: Option<LineageProof>,
    pub eve_proof: Option<EveProof>,
}

impl Proof {
    pub fn to_native(&self) -> Result<RustProof, JsValue> {
        if let Some(lp) = &self.lineage_proof {
            Ok(RustProof::Lineage(RustLineageProof {
                parent_parent_coin_info: bytes32(&lp.parent_parent_coin_info)?,
                parent_inner_puzzle_hash: bytes32(&lp.parent_inner_puzzle_hash)?,
                parent_amount: lp.parent_amount,
            }))
        } else if let Some(ep) = &self.eve_proof {
            Ok(RustProof::Eve(RustEveProof {
                parent_parent_coin_info: bytes32(&ep.parent_parent_coin_info)?,
                parent_amount: ep.parent_amount,
            }))
        } else {
            Err(JsValue::from_str("missing proof"))
        }
    }
    pub fn from_native(p: &RustProof) -> Self {
        match p {
            RustProof::Lineage(lp) => Proof {
                lineage_proof: Some(LineageProof {
                    parent_parent_coin_info: lp.parent_parent_coin_info.to_vec(),
                    parent_inner_puzzle_hash: lp.parent_inner_puzzle_hash.to_vec(),
                    parent_amount: lp.parent_amount,
                }),
                eve_proof: None,
            },
            RustProof::Eve(ep) => Proof {
                lineage_proof: None,
                eve_proof: Some(EveProof {
                    parent_parent_coin_info: ep.parent_parent_coin_info.to_vec(),
                    parent_amount: ep.parent_amount,
                }),
            },
        }
    }
}
```

- [ ] **Step 3: Add the DataStore family boundary structs and conversions**

Mirror `napi_lib.rs:90-300`. Append to `wasm/src/types.rs`:
```rust
use datalayer_driver::{
    DataStore as RustDataStore, DataStoreInfo as RustDataStoreInfo,
    DataStoreMetadata as RustDataStoreMetadata, DelegatedPuzzle as RustDelegatedPuzzle,
};
use datalayer_driver::types::SuccessResponse as RustSuccessResponse;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStoreMetadata {
    #[serde(with = "serde_bytes")]
    pub root_hash: Vec<u8>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub bytes: Option<u64>,
    #[serde(default, with = "serde_bytes")]
    pub size_proof: Option<Vec<u8>>,
}

impl DataStoreMetadata {
    pub fn to_native(&self) -> Result<RustDataStoreMetadata, JsValue> {
        Ok(RustDataStoreMetadata {
            root_hash: bytes32(&self.root_hash)?,
            label: self.label.clone(),
            description: self.description.clone(),
            bytes: self.bytes,
            // NAPI stores size_proof as the hex string of a Bytes32.
            size_proof: match &self.size_proof {
                Some(sp) => Some(bytes32(sp)?.to_string()),
                None => None,
            },
        })
    }
    pub fn from_native(m: &RustDataStoreMetadata) -> Result<Self, JsValue> {
        Ok(DataStoreMetadata {
            root_hash: m.root_hash.to_vec(),
            label: m.label.clone(),
            description: m.description.clone(),
            bytes: m.bytes,
            size_proof: match &m.size_proof {
                Some(s) => Some(
                    hex::decode(s.trim_start_matches("0x"))
                        .map_err(|_| JsValue::from_str("invalid size_proof hex"))?,
                ),
                None => None,
            },
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegatedPuzzle {
    #[serde(default, with = "serde_bytes")]
    pub admin_inner_puzzle_hash: Option<Vec<u8>>,
    #[serde(default, with = "serde_bytes")]
    pub writer_inner_puzzle_hash: Option<Vec<u8>>,
    #[serde(default, with = "serde_bytes")]
    pub oracle_payment_puzzle_hash: Option<Vec<u8>>,
    pub oracle_fee: Option<u64>,
}

impl DelegatedPuzzle {
    pub fn to_native(&self) -> Result<RustDelegatedPuzzle, JsValue> {
        if let Some(h) = &self.admin_inner_puzzle_hash {
            Ok(RustDelegatedPuzzle::Admin(bytes32(h)?.into()))
        } else if let Some(h) = &self.writer_inner_puzzle_hash {
            Ok(RustDelegatedPuzzle::Writer(bytes32(h)?.into()))
        } else if let (Some(h), Some(fee)) = (&self.oracle_payment_puzzle_hash, self.oracle_fee) {
            Ok(RustDelegatedPuzzle::Oracle(bytes32(h)?, fee))
        } else {
            Err(JsValue::from_str("missing delegated puzzle info"))
        }
    }
    pub fn from_native(d: &RustDelegatedPuzzle) -> Result<Self, JsValue> {
        Ok(match d {
            RustDelegatedPuzzle::Admin(h) => {
                let h: Bytes32 = (*h).into();
                DelegatedPuzzle { admin_inner_puzzle_hash: Some(h.to_vec()), writer_inner_puzzle_hash: None, oracle_payment_puzzle_hash: None, oracle_fee: None }
            }
            RustDelegatedPuzzle::Writer(h) => {
                let h: Bytes32 = (*h).into();
                DelegatedPuzzle { admin_inner_puzzle_hash: None, writer_inner_puzzle_hash: Some(h.to_vec()), oracle_payment_puzzle_hash: None, oracle_fee: None }
            }
            RustDelegatedPuzzle::Oracle(h, fee) => DelegatedPuzzle {
                admin_inner_puzzle_hash: None,
                writer_inner_puzzle_hash: None,
                oracle_payment_puzzle_hash: Some(h.to_vec()),
                oracle_fee: Some(*fee),
            },
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStore {
    pub coin: Coin,
    #[serde(with = "serde_bytes")]
    pub launcher_id: Vec<u8>,
    pub proof: Proof,
    pub metadata: DataStoreMetadata,
    #[serde(with = "serde_bytes")]
    pub owner_puzzle_hash: Vec<u8>,
    pub delegated_puzzles: Vec<DelegatedPuzzle>,
}

impl DataStore {
    pub fn to_native(&self) -> Result<RustDataStore, JsValue> {
        Ok(RustDataStore {
            coin: self.coin.to_native()?,
            proof: self.proof.to_native()?,
            info: RustDataStoreInfo {
                launcher_id: bytes32(&self.launcher_id)?,
                metadata: self.metadata.to_native()?,
                owner_puzzle_hash: bytes32(&self.owner_puzzle_hash)?,
                delegated_puzzles: self
                    .delegated_puzzles
                    .iter()
                    .map(DelegatedPuzzle::to_native)
                    .collect::<Result<Vec<_>, _>>()?,
            },
        })
    }
    pub fn from_native(s: &RustDataStore) -> Result<Self, JsValue> {
        Ok(DataStore {
            coin: Coin::from_native(&s.coin),
            launcher_id: s.info.launcher_id.to_vec(),
            proof: Proof::from_native(&s.proof),
            metadata: DataStoreMetadata::from_native(&s.info.metadata)?,
            owner_puzzle_hash: s.info.owner_puzzle_hash.to_vec(),
            delegated_puzzles: s
                .info
                .delegated_puzzles
                .iter()
                .map(DelegatedPuzzle::from_native)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub coin_spends: Vec<CoinSpend>,
    pub new_store: DataStore,
}

impl SuccessResponse {
    pub fn from_native(r: &RustSuccessResponse) -> Result<Self, JsValue> {
        Ok(SuccessResponse {
            coin_spends: r.coin_spends.iter().map(CoinSpend::from_native).collect(),
            new_store: DataStore::from_native(&r.new_datastore)?,
        })
    }
}
```

Note: confirm the field name on `RustSuccessResponse` is `new_datastore` (per `napi_lib.rs:290`). If `cargo build` reports a different name, correct it here.

- [ ] **Step 4: Add helper to decode a `Vec<Coin>` and `Vec<DelegatedPuzzle>` from JsValue**

Append:
```rust
pub fn coins_from_js(value: JsValue) -> Result<Vec<RustCoin>, JsValue> {
    let coins: Vec<Coin> = from_js(value)?;
    coins.iter().map(Coin::to_native).collect()
}

pub fn delegated_puzzles_from_js(value: JsValue) -> Result<Vec<RustDelegatedPuzzle>, JsValue> {
    let dps: Vec<DelegatedPuzzle> = from_js(value)?;
    dps.iter().map(DelegatedPuzzle::to_native).collect()
}

pub fn coin_spends_from_js(value: JsValue) -> Result<Vec<RustCoinSpend>, JsValue> {
    let css: Vec<CoinSpend> = from_js(value)?;
    css.iter().map(CoinSpend::to_native).collect()
}

pub fn coin_spends_to_js(css: &[RustCoinSpend]) -> Result<JsValue, JsValue> {
    let out: Vec<CoinSpend> = css.iter().map(CoinSpend::from_native).collect();
    to_js(&out)
}
```

- [ ] **Step 5: Verify it compiles to wasm**

Run:
```powershell
cargo build -p datalayer-driver-wasm --target wasm32-unknown-unknown
```
Expected: success (warnings about unused functions are fine; they are used in later tasks). Fix any field-name/type mismatches the compiler reports against the real `datalayer-driver` types.

- [ ] **Step 6: Commit**

```powershell
git add wasm/src/types.rs
git commit -m "feat(wasm): add serde boundary structs and native conversions"
```

---

## Task 5: Implement the utility functions (keys, addresses, proofs, ids)

These are the simplest and validate the whole toolchain end-to-end. TDD: write a Node parity test stub first (full harness lands in Task 8; here we hand-verify with a quick node check).

**Files:**
- Modify: `wasm/src/lib.rs`

- [ ] **Step 1: Implement key/address/proof/id functions**

Append to `wasm/src/lib.rs` (after `init`):
```rust
use datalayer_driver::{
    address_to_puzzle_hash as core_address_to_puzzle_hash, admin_delegated_puzzle_from_key as core_admin_dp,
    get_coin_id as core_get_coin_id, master_public_key_to_first_puzzle_hash as core_mpk_to_first_ph,
    master_public_key_to_wallet_synthetic_key as core_mpk_to_synth,
    master_secret_key_to_wallet_synthetic_secret_key as core_msk_to_synth,
    morph_launcher_id_wrapper as core_morph_launcher_id, puzzle_hash_to_address as core_ph_to_address,
    secret_key_to_public_key as core_sk_to_pk, synthetic_key_to_puzzle_hash as core_synth_to_ph,
    writer_delegated_puzzle_from_key as core_writer_dp,
    get_mainnet_genesis_challenge as core_mainnet_gc, get_testnet11_genesis_challenge as core_testnet_gc,
};

use crate::types::{
    bytes32, from_js, public_key, secret_key, to_js, Coin, DelegatedPuzzle, EveProof, LineageProof, Proof,
};

#[wasm_bindgen(js_name = "masterPublicKeyToWalletSyntheticKey")]
pub fn master_public_key_to_wallet_synthetic_key(public_key_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let pk = public_key(public_key_bytes)?;
    Ok(core_mpk_to_synth(&pk).to_bytes().to_vec())
}

#[wasm_bindgen(js_name = "masterPublicKeyToFirstPuzzleHash")]
pub fn master_public_key_to_first_puzzle_hash(public_key_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let pk = public_key(public_key_bytes)?;
    Ok(core_mpk_to_first_ph(&pk).to_vec())
}

#[wasm_bindgen(js_name = "masterSecretKeyToWalletSyntheticSecretKey")]
pub fn master_secret_key_to_wallet_synthetic_secret_key(secret_key_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let sk = secret_key(secret_key_bytes)?;
    Ok(core_msk_to_synth(&sk).to_bytes().to_vec())
}

#[wasm_bindgen(js_name = "secretKeyToPublicKey")]
pub fn secret_key_to_public_key(secret_key_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let sk = secret_key(secret_key_bytes)?;
    Ok(core_sk_to_pk(&sk).to_bytes().to_vec())
}

#[wasm_bindgen(js_name = "syntheticKeyToPuzzleHash")]
pub fn synthetic_key_to_puzzle_hash(synthetic_key_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let pk = public_key(synthetic_key_bytes)?;
    Ok(core_synth_to_ph(&pk).to_vec())
}

#[wasm_bindgen(js_name = "puzzleHashToAddress")]
pub fn puzzle_hash_to_address(puzzle_hash: &[u8], prefix: String) -> Result<String, JsValue> {
    core_ph_to_address(bytes32(puzzle_hash)?, &prefix).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "addressToPuzzleHash")]
pub fn address_to_puzzle_hash(address: String) -> Result<Vec<u8>, JsValue> {
    core_address_to_puzzle_hash(&address)
        .map(|b| b.to_vec())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "adminDelegatedPuzzleFromKey")]
pub fn admin_delegated_puzzle_from_key(synthetic_key: &[u8]) -> Result<JsValue, JsValue> {
    let dp = core_admin_dp(&public_key(synthetic_key)?);
    to_js(&DelegatedPuzzle::from_native(&dp)?)
}

#[wasm_bindgen(js_name = "writerDelegatedPuzzleFromKey")]
pub fn writer_delegated_puzzle_from_key(synthetic_key: &[u8]) -> Result<JsValue, JsValue> {
    let dp = core_writer_dp(&public_key(synthetic_key)?);
    to_js(&DelegatedPuzzle::from_native(&dp)?)
}

#[wasm_bindgen(js_name = "newLineageProof")]
pub fn new_lineage_proof(lineage_proof: JsValue) -> Result<JsValue, JsValue> {
    let lp: LineageProof = from_js(lineage_proof)?;
    to_js(&Proof { lineage_proof: Some(lp), eve_proof: None })
}

#[wasm_bindgen(js_name = "newEveProof")]
pub fn new_eve_proof(eve_proof: JsValue) -> Result<JsValue, JsValue> {
    let ep: EveProof = from_js(eve_proof)?;
    to_js(&Proof { lineage_proof: None, eve_proof: Some(ep) })
}

#[wasm_bindgen(js_name = "getCoinId")]
pub fn get_coin_id(coin: JsValue) -> Result<Vec<u8>, JsValue> {
    let c: Coin = from_js(coin)?;
    Ok(core_get_coin_id(&c.to_native()?).to_vec())
}

#[wasm_bindgen(js_name = "morphLauncherId")]
pub fn morph_launcher_id(launcher_id: &[u8], offset: u64) -> Result<Vec<u8>, JsValue> {
    Ok(core_morph_launcher_id(bytes32(launcher_id)?, offset).to_vec())
}

#[wasm_bindgen(js_name = "getMainnetGenesisChallenge")]
pub fn get_mainnet_genesis_challenge() -> Vec<u8> {
    core_mainnet_gc().to_vec()
}

#[wasm_bindgen(js_name = "getTestnet11GenesisChallenge")]
pub fn get_testnet11_genesis_challenge() -> Vec<u8> {
    core_testnet_gc().to_vec()
}
```

Note: `morph_launcher_id_wrapper` is the crate-root export (`src/lib.rs:143`); confirm the symbol name when compiling and adjust the `use` alias if needed.

- [ ] **Step 2: Build to wasm and to a node target for a quick check**

Run:
```powershell
cargo build -p datalayer-driver-wasm --target wasm32-unknown-unknown
wasm-pack build wasm --target nodejs --dev --out-dir pkg-node
```
Expected: both succeed; `wasm/pkg-node/` is produced.

- [ ] **Step 3: Quick manual round-trip check**

Run:
```powershell
node -e "const w=require('./wasm/pkg-node'); w.init(); const ph=w.addressToPuzzleHash('xch1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsk9rsxe'); console.log(Buffer.from(ph).toString('hex')); console.log(w.puzzleHashToAddress(ph,'xch'));"
```
Expected: prints a 64-char hex and re-encodes to the same address (round-trip). If the sample address is invalid, substitute any valid `xch1...` address; the point is the round-trip equality.

- [ ] **Step 4: Commit**

```powershell
git add wasm/src/lib.rs
git commit -m "feat(wasm): key, address, proof, and id utility bindings"
```

---

## Task 6: Implement DIGStore spend builders (PRIMARY GOAL)

`mint_store`, `oracle_spend`, `update_store_metadata`, `update_store_ownership`, `melt_store`. Signatures mirror the NAPI JS API positionally.

**Files:**
- Modify: `wasm/src/lib.rs`
- Reference: `napi/src/napi_lib.rs:1241` (mint_store), `:1292` (oracle_spend), `:1533` (update_store_metadata), `:1590` (update_store_ownership), `:1636` (melt_store); core wrappers `src/lib.rs:237,264,280,301,316`.

- [ ] **Step 1: Inspect the exact NAPI signatures to mirror**

Run:
```powershell
Select-String -Path napi\src\napi_lib.rs -Pattern 'pub fn (mint_store|oracle_spend|update_store_metadata|update_store_ownership|melt_store)' -Context 0,14
```
Expected: prints each function's parameter list. Confirm parameter order and the `DataStoreInnerSpend` shape used by `update_store_*` (it carries the synthetic key / delegated-puzzle selection). Mirror those exactly in Step 2–3. If `update_store_*` takes a non-trivial `inner_spend_info`, model it as a `JsValue` deserialized into a local `InnerSpendInfo` boundary struct added to `types.rs` matching the NAPI `DataStoreInnerSpend` JS fields.

- [ ] **Step 2: Implement `mint_store`, `oracle_spend`, `melt_store`**

Append to `wasm/src/lib.rs`:
```rust
use datalayer_driver::{
    melt_store as core_melt_store, mint_store as core_mint_store, oracle_spend as core_oracle_spend,
};
use crate::types::{
    coin_spends_to_js, coins_from_js, delegated_puzzles_from_js, DataStore, SuccessResponse,
};

#[wasm_bindgen(js_name = "mintStore")]
#[allow(clippy::too_many_arguments)]
pub fn mint_store(
    minter_synthetic_key: &[u8],
    selected_coins: JsValue,        // Coin[]
    root_hash: &[u8],
    label: Option<String>,
    description: Option<String>,
    bytes: Option<u64>,
    owner_puzzle_hash: &[u8],
    delegated_puzzles: JsValue,     // DelegatedPuzzle[]
    fee: u64,
) -> Result<JsValue, JsValue> {     // SuccessResponse
    let resp = core_mint_store(
        public_key(minter_synthetic_key)?,
        coins_from_js(selected_coins)?,
        bytes32(root_hash)?,
        label,
        description,
        bytes,
        None, // size_proof: NAPI exposes Option<Buffer>; add a param if a test needs it
        bytes32(owner_puzzle_hash)?,
        delegated_puzzles_from_js(delegated_puzzles)?,
        fee,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&SuccessResponse::from_native(&resp)?)
}

#[wasm_bindgen(js_name = "oracleSpend")]
pub fn oracle_spend(
    spender_synthetic_key: &[u8],
    selected_coins: JsValue, // Coin[]
    store: JsValue,          // DataStore
    fee: u64,
) -> Result<JsValue, JsValue> {
    let store: DataStore = from_js(store)?;
    let resp = core_oracle_spend(
        public_key(spender_synthetic_key)?,
        coins_from_js(selected_coins)?,
        store.to_native()?,
        fee,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&SuccessResponse::from_native(&resp)?)
}

#[wasm_bindgen(js_name = "meltStore")]
pub fn melt_store(store: JsValue, owner_public_key: &[u8]) -> Result<JsValue, JsValue> {
    let store: DataStore = from_js(store)?;
    let css = core_melt_store(store.to_native()?, public_key(owner_public_key)?)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    coin_spends_to_js(&css)
}
```

Note: the NAPI `mint_store` includes a `size_proof: Option<Buffer>` parameter between `bytes` and `owner_puzzle_hash`. If the parity test in Task 8 exercises `size_proof`, add `size_proof: Option<Vec<u8>>` as a param and pass `size_proof.map(|b| bytes32(&b)).transpose()?.map(|h| h.to_string())`. Keep parameter order identical to NAPI.

- [ ] **Step 3: Implement `update_store_metadata` and `update_store_ownership`**

Add the `InnerSpendInfo` boundary struct to `wasm/src/types.rs` first, matching the NAPI `DataStoreInnerSpend` JS shape discovered in Step 1 (typical shape: an owner/admin/writer synthetic key plus optional delegated-puzzle list). Example skeleton — adjust fields to the real shape:
```rust
// wasm/src/types.rs
use datalayer_driver::DataStoreInnerSpend as RustDataStoreInnerSpend;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerSpendInfo {
    #[serde(default, with = "serde_bytes")]
    pub public_key: Option<Vec<u8>>,
    // ...mirror the real DataStoreInnerSpend variants discovered in Task 6 Step 1
}

impl InnerSpendInfo {
    pub fn to_native(&self) -> Result<RustDataStoreInnerSpend, JsValue> {
        // construct the matching native variant
        todo!("fill in per the real DataStoreInnerSpend definition")
    }
}
```
Then in `wasm/src/lib.rs`:
```rust
use datalayer_driver::{
    update_store_metadata as core_update_meta, update_store_ownership as core_update_owner,
};
use crate::types::InnerSpendInfo;

#[wasm_bindgen(js_name = "updateStoreMetadata")]
#[allow(clippy::too_many_arguments)]
pub fn update_store_metadata(
    store: JsValue,            // DataStore
    new_root_hash: &[u8],
    new_label: Option<String>,
    new_description: Option<String>,
    new_bytes: Option<u64>,
    inner_spend_info: JsValue, // InnerSpendInfo
) -> Result<JsValue, JsValue> {
    let store: DataStore = from_js(store)?;
    let info: InnerSpendInfo = from_js(inner_spend_info)?;
    let resp = core_update_meta(
        store.to_native()?,
        bytes32(new_root_hash)?,
        new_label,
        new_description,
        new_bytes,
        None, // new_size_proof — add a param if exercised
        info.to_native()?,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&SuccessResponse::from_native(&resp)?)
}

#[wasm_bindgen(js_name = "updateStoreOwnership")]
pub fn update_store_ownership(
    store: JsValue,                 // DataStore
    new_owner_puzzle_hash: &[u8],
    new_delegated_puzzles: JsValue, // DelegatedPuzzle[]
    inner_spend_info: JsValue,      // InnerSpendInfo
) -> Result<JsValue, JsValue> {
    let store: DataStore = from_js(store)?;
    let info: InnerSpendInfo = from_js(inner_spend_info)?;
    let resp = core_update_owner(
        store.to_native()?,
        bytes32(new_owner_puzzle_hash)?,
        delegated_puzzles_from_js(new_delegated_puzzles)?,
        info.to_native()?,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&SuccessResponse::from_native(&resp)?)
}
```
Replace the `todo!()` in `InnerSpendInfo::to_native` with the real construction once Step 1 reveals the exact `DataStoreInnerSpend` definition. The build will fail until this is done — that is intentional (it forces matching the real type).

- [ ] **Step 4: Build to wasm**

Run:
```powershell
cargo build -p datalayer-driver-wasm --target wasm32-unknown-unknown
```
Expected: success once `InnerSpendInfo` matches the real type. Fix compiler-reported mismatches against the real signatures.

- [ ] **Step 5: Commit**

```powershell
git add wasm/src/lib.rs wasm/src/types.rs
git commit -m "feat(wasm): DIGStore spend builders (mint/oracle/melt/update)"
```

---

## Task 7: Implement signing, serialization, coin selection, and server-coin helpers

`sign_coin_spends`, `sign_message`, `verify_signed_message`, `spend_bundle_to_hex`, `hex_spend_bundle_to_coin_spends`, `get_cost`, `select_coins`, `send_xch`, `add_fee`, `create_server_coin`.

**Files:**
- Modify: `wasm/src/lib.rs`, `wasm/src/types.rs`

- [ ] **Step 1: Add `ServerCoin`, `NewServerCoin`, `Output` boundary structs to `types.rs`**

```rust
use datalayer_driver::XchServerCoin as RustXchServerCoin;
use datalayer_driver::xch_server_coin::NewXchServerCoin as RustNewXchServerCoin;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCoin {
    pub coin: Coin,
    #[serde(with = "serde_bytes")]
    pub p2_puzzle_hash: Vec<u8>,
    pub memo_urls: Vec<String>,
}

impl ServerCoin {
    pub fn from_native(s: &RustXchServerCoin) -> Self {
        ServerCoin {
            coin: Coin::from_native(&s.coin),
            p2_puzzle_hash: s.p2_puzzle_hash.to_vec(),
            memo_urls: s.memo_urls.clone(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewServerCoin {
    pub server_coin: ServerCoin,
    pub coin_spends: Vec<CoinSpend>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    #[serde(with = "serde_bytes")]
    pub puzzle_hash: Vec<u8>,
    pub amount: u64,
    pub memos: Vec<serde_bytes::ByteBuf>, // Vec<Buffer>
}
```
Confirm `NewXchServerCoin`'s fields against `src/xch_server_coin.rs`; adjust `NewServerCoin::from_native` accordingly (added in Step 3).

- [ ] **Step 2: Implement signing + serialization + cost + selection**

Append to `wasm/src/lib.rs`:
```rust
use datalayer_driver::{
    get_cost as core_get_cost, hex_spend_bundle_to_coin_spends as core_hex_to_css,
    select_coins as core_select_coins, sign_coin_spends as core_sign_css, sign_message as core_sign_msg,
    spend_bundle_to_hex as core_sb_to_hex, verify_signed_message as core_verify_msg, SpendBundle as RustSpendBundle,
};
use crate::types::{coin_spends_from_js, signature, Coin};

#[wasm_bindgen(js_name = "signCoinSpends")]
pub fn sign_coin_spends(
    coin_spends: JsValue,      // CoinSpend[]
    private_keys: JsValue,     // Uint8Array[]  (each a 32-byte secret key)
    for_testnet: bool,
) -> Result<Vec<u8>, JsValue> { // aggregated signature, 96 bytes
    let css = coin_spends_from_js(coin_spends)?;
    let keys_raw: Vec<serde_bytes::ByteBuf> = from_js(private_keys)?;
    let keys = keys_raw
        .iter()
        .map(|k| secret_key(k))
        .collect::<Result<Vec<_>, _>>()?;
    let sig = core_sign_css(&css, &keys, for_testnet).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(sig.to_bytes().to_vec())
}

#[wasm_bindgen(js_name = "signMessage")]
pub fn sign_message(message: &[u8], private_key: &[u8]) -> Result<Vec<u8>, JsValue> {
    let sig = core_sign_msg(message, &secret_key(private_key)?).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(sig.to_bytes().to_vec())
}

#[wasm_bindgen(js_name = "verifySignedMessage")]
pub fn verify_signed_message(sig: &[u8], public_key_bytes: &[u8], message: &[u8]) -> Result<bool, JsValue> {
    core_verify_msg(&signature(sig)?, &public_key(public_key_bytes)?, message)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "getCost")]
pub fn get_cost(coin_spends: JsValue) -> Result<u64, JsValue> {
    core_get_cost(&coin_spends_from_js(coin_spends)?).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "selectCoins")]
pub fn select_coins(all_coins: JsValue, total_amount: u64) -> Result<JsValue, JsValue> {
    let selected = core_select_coins(&coins_from_js(all_coins)?, total_amount)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out: Vec<Coin> = selected.iter().map(Coin::from_native).collect();
    to_js(&out)
}

#[wasm_bindgen(js_name = "spendBundleToHex")]
pub fn spend_bundle_to_hex(spend_bundle: JsValue) -> Result<String, JsValue> {
    // SpendBundle = { coinSpends: CoinSpend[], aggregatedSignature: Uint8Array }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SbIn { coin_spends: Vec<crate::types::CoinSpend>, #[serde(with = "serde_bytes")] aggregated_signature: Vec<u8> }
    let sb: SbIn = from_js(spend_bundle)?;
    let css = sb.coin_spends.iter().map(crate::types::CoinSpend::to_native).collect::<Result<Vec<_>, _>>()?;
    let bundle = RustSpendBundle::new(css, signature(&sb.aggregated_signature)?);
    core_sb_to_hex(&bundle).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "hexSpendBundleToCoinSpends")]
pub fn hex_spend_bundle_to_coin_spends(hex: String) -> Result<JsValue, JsValue> {
    let css = core_hex_to_css(&hex).map_err(|e| JsValue::from_str(&e.to_string()))?;
    coin_spends_to_js(&css)
}
```
Confirm `RustSpendBundle::new(coin_spends, signature)` constructor signature; the core re-export is `chia::protocol::SpendBundle`. Adjust if its constructor differs.

- [ ] **Step 3: Implement `send_xch`, `add_fee`, `create_server_coin`**

```rust
use datalayer_driver::{
    add_fee as core_add_fee, create_server_coin as core_create_server_coin, send_xch as core_send_xch, Output as RustOutput,
};
use crate::types::{NewServerCoin, Output, ServerCoin};

#[wasm_bindgen(js_name = "sendXch")]
pub fn send_xch(
    synthetic_key: &[u8],
    selected_coins: JsValue, // Coin[]
    outputs: JsValue,        // Output[]
    fee: u64,
) -> Result<JsValue, JsValue> {
    let outs: Vec<Output> = from_js(outputs)?;
    let native_outs: Vec<RustOutput> = outs
        .iter()
        .map(|o| {
            Ok(RustOutput {
                puzzle_hash: bytes32(&o.puzzle_hash)?,
                amount: o.amount,
                memos: o.memos.iter().map(|m| chia::protocol::Bytes::new(m.to_vec())).collect(),
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    let css = core_send_xch(&public_key(synthetic_key)?, &coins_from_js(selected_coins)?, &native_outs, fee)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    coin_spends_to_js(&css)
}

#[wasm_bindgen(js_name = "addFee")]
pub fn add_fee(
    spender_synthetic_key: &[u8],
    selected_coins: JsValue,    // Coin[]
    assert_coin_ids: JsValue,   // Uint8Array[]
    fee: u64,
) -> Result<JsValue, JsValue> {
    let ids_raw: Vec<serde_bytes::ByteBuf> = from_js(assert_coin_ids)?;
    let ids = ids_raw.iter().map(|b| bytes32(b)).collect::<Result<Vec<_>, _>>()?;
    let css = core_add_fee(&public_key(spender_synthetic_key)?, &coins_from_js(selected_coins)?, &ids, fee)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    coin_spends_to_js(&css)
}

#[wasm_bindgen(js_name = "createServerCoin")]
pub fn create_server_coin(
    synthetic_key: &[u8],
    selected_coins: JsValue, // Coin[]
    hint: &[u8],
    uris: JsValue,           // string[]
    amount: u64,
    fee: u64,
) -> Result<JsValue, JsValue> {
    let uris: Vec<String> = from_js(uris)?;
    let new_sc = core_create_server_coin(
        public_key(synthetic_key)?,
        coins_from_js(selected_coins)?,
        bytes32(hint)?,
        uris,
        amount,
        fee,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    // Map RustNewXchServerCoin -> NewServerCoin (confirm field names)
    let out = NewServerCoin {
        server_coin: ServerCoin::from_native(&new_sc.server_coin),
        coin_spends: new_sc.coin_spends.iter().map(crate::types::CoinSpend::from_native).collect(),
    };
    to_js(&out)
}
```
Confirm `RustNewXchServerCoin` field names (`server_coin`, `coin_spends`) against `src/xch_server_coin.rs`; adjust if different.

- [ ] **Step 4: Build to wasm**

Run:
```powershell
cargo build -p datalayer-driver-wasm --target wasm32-unknown-unknown
```
Expected: success. Fix any constructor/field mismatches the compiler reports.

- [ ] **Step 5: Commit**

```powershell
git add wasm/src/lib.rs wasm/src/types.rs
git commit -m "feat(wasm): signing, serialization, selection, server-coin bindings"
```

---

## Task 8: Node parity test — WASM output equals NAPI output

**Files:**
- Create: `wasm/package.json`, `wasm/tests/parity.mjs`

- [ ] **Step 1: Create `wasm/package.json` with build + test scripts**

```json
{
  "name": "@dignetwork/datalayer-driver-wasm-dev",
  "version": "3.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "wasm-pack build . --target bundler --release && node scripts/patch-pkg.mjs",
    "build:node": "wasm-pack build . --target nodejs --release --out-dir pkg-node",
    "test": "npm run build:node && node tests/parity.mjs"
  },
  "devDependencies": {
    "@dignetwork/datalayer-driver": "file:../napi"
  }
}
```
Note: the test depends on the NAPI package built in `../napi`. In CI the NAPI `.node` artifact is downloaded into `napi/` before this runs (Task 10).

- [ ] **Step 2: Write the parity test**

`wasm/tests/parity.mjs`:
```js
import assert from "node:assert/strict";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);

const wasm = require("../pkg-node");
const napi = require("@dignetwork/datalayer-driver");
wasm.init();

const eqBytes = (a, b, msg) =>
  assert.equal(Buffer.from(a).toString("hex"), Buffer.from(b).toString("hex"), msg);

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

// 2. Address round-trip parity.
const addrW = wasm.puzzleHashToAddress(phW, "xch");
const addrN = napi.puzzleHashToAddress(phN, "xch");
assert.equal(addrW, addrN, "puzzleHashToAddress");
eqBytes(wasm.addressToPuzzleHash(addrW), napi.addressToPuzzleHash(addrN), "addressToPuzzleHash");

// 3. PRIMARY: mint_store spend bundle parity.
const ownerPh = phW;
const adminDpW = wasm.adminDelegatedPuzzleFromKey(synthW);
const adminDpN = napi.adminDelegatedPuzzleFromKey(synthN);

const coin = {
  parentCoinInfo: Buffer.alloc(32, 2),
  puzzleHash: Buffer.from(ownerPh),
  amount: 1_000_000_000_000n, // 1 XCH — exceeds 2^53 territory in aggregate
};
const rootHash = Buffer.alloc(32, 3);

const mintW = wasm.mintStore(synthW, [coin], rootHash, "label", "desc", 42n, ownerPh, [adminDpW], 0n);
const mintN = napi.mintStore(synthN, [coin], rootHash, "label", "desc", 42n, ownerPh, [adminDpN], 0n);

// Compare the produced coin spends byte-for-byte via the hex of a signed bundle.
const sigW = wasm.signCoinSpends(mintW.coinSpends, [sk], true);
const sigN = napi.signCoinSpends(mintN.coinSpends, [sk], true);
eqBytes(sigW, sigN, "signCoinSpends(mint)");

const hexW = wasm.spendBundleToHex({ coinSpends: mintW.coinSpends, aggregatedSignature: sigW });
const hexN = napi.spendBundleToHex({ coinSpends: mintN.coinSpends, aggregatedSignature: sigN });
assert.equal(hexW, hexN, "mint spend bundle hex parity");

// 4. Cost + coin id parity.
assert.equal(wasm.getCost(mintW.coinSpends), napi.getCost(mintN.coinSpends), "getCost");
eqBytes(wasm.getCoinId(coin), napi.getCoinId(coin), "getCoinId");

console.log("All parity checks passed.");
```
Note: align the `mintStore` argument list with the final WASM signature from Task 6 (especially the `size_proof` parameter — include it in BOTH calls or omit from both). If NAPI's `mintStore` requires `size_proof`, pass `null`/`undefined` in both.

- [ ] **Step 3: Run the parity test locally (requires a native NAPI build)**

Run:
```powershell
cd napi; npm install; npm run build; cd ..
cd wasm; npm install; npm test; cd ..
```
Expected: `All parity checks passed.` If a check fails, the WASM conversion for that function diverges from NAPI — fix the conversion in `wasm/src/types.rs` or the binding in `wasm/src/lib.rs`, rebuild, re-run.

- [ ] **Step 4: Commit**

```powershell
git add wasm/package.json wasm/tests/parity.mjs
git commit -m "test(wasm): node parity test vs napi (incl. mint_store bundle)"
```

---

## Task 9: Package metadata patch + typed `.d.ts`

wasm-pack names the package after the crate (`datalayer-driver-wasm`) and emits `any` for `JsValue` params. We override both for the published artifact.

**Files:**
- Create: `wasm/scripts/patch-pkg.mjs`, `wasm/types/datalayer-driver-wasm.d.ts`

- [ ] **Step 1: Write the typed declaration file**

`wasm/types/datalayer-driver-wasm.d.ts` — mirror the NAPI `index.d.ts` object shapes. Representative content (extend to cover every exported function):
```ts
export interface Coin { parentCoinInfo: Uint8Array; puzzleHash: Uint8Array; amount: bigint; }
export interface CoinSpend { coin: Coin; puzzleReveal: Uint8Array; solution: Uint8Array; }
export interface LineageProof { parentParentCoinInfo: Uint8Array; parentInnerPuzzleHash: Uint8Array; parentAmount: bigint; }
export interface EveProof { parentParentCoinInfo: Uint8Array; parentAmount: bigint; }
export interface Proof { lineageProof?: LineageProof; eveProof?: EveProof; }
export interface DataStoreMetadata { rootHash: Uint8Array; label?: string; description?: string; bytes?: bigint; sizeProof?: Uint8Array; }
export interface DelegatedPuzzle { adminInnerPuzzleHash?: Uint8Array; writerInnerPuzzleHash?: Uint8Array; oraclePaymentPuzzleHash?: Uint8Array; oracleFee?: bigint; }
export interface DataStore { coin: Coin; launcherId: Uint8Array; proof: Proof; metadata: DataStoreMetadata; ownerPuzzleHash: Uint8Array; delegatedPuzzles: DelegatedPuzzle[]; }
export interface SuccessResponse { coinSpends: CoinSpend[]; newStore: DataStore; }

export function init(): void;
export function mintStore(minterSyntheticKey: Uint8Array, selectedCoins: Coin[], rootHash: Uint8Array, label: string | undefined, description: string | undefined, bytes: bigint | undefined, ownerPuzzleHash: Uint8Array, delegatedPuzzles: DelegatedPuzzle[], fee: bigint): SuccessResponse;
export function oracleSpend(spenderSyntheticKey: Uint8Array, selectedCoins: Coin[], store: DataStore, fee: bigint): SuccessResponse;
export function meltStore(store: DataStore, ownerPublicKey: Uint8Array): CoinSpend[];
export function signCoinSpends(coinSpends: CoinSpend[], privateKeys: Uint8Array[], forTestnet: boolean): Uint8Array;
export function spendBundleToHex(spendBundle: { coinSpends: CoinSpend[]; aggregatedSignature: Uint8Array }): string;
export function getCost(coinSpends: CoinSpend[]): bigint;
export function getCoinId(coin: Coin): Uint8Array;
export function selectCoins(allCoins: Coin[], totalAmount: bigint): Coin[];
export function puzzleHashToAddress(puzzleHash: Uint8Array, prefix: string): string;
export function addressToPuzzleHash(address: string): Uint8Array;
// ...add remaining functions to match the full exported surface
```
Keep this file in sync with the final binding signatures from Tasks 5–7.

- [ ] **Step 2: Write the patch script**

`wasm/scripts/patch-pkg.mjs`:
```js
import { readFile, writeFile, copyFile } from "node:fs/promises";
import path from "node:path";

const pkgDir = path.resolve("pkg");
const pkgJsonPath = path.join(pkgDir, "package.json");

const pkg = JSON.parse(await readFile(pkgJsonPath, "utf8"));

pkg.name = "@dignetwork/datalayer-driver-wasm";
pkg.description = "WebAssembly bindings for the Chia DataLayer driver (offline DIGStore spend-bundle construction).";
pkg.repository = { type: "git", url: "https://github.com/DIG-Network/DataLayer-Driver.git" };
pkg.license = "MIT";
pkg.types = "datalayer-driver-wasm.d.ts";
pkg.files = Array.from(new Set([...(pkg.files ?? []), "datalayer-driver-wasm.d.ts"]));

await writeFile(pkgJsonPath, JSON.stringify(pkg, null, 2) + "\n");
await copyFile(path.resolve("types/datalayer-driver-wasm.d.ts"), path.join(pkgDir, "datalayer-driver-wasm.d.ts"));

console.log(`Patched ${pkg.name}@${pkg.version}`);
```

- [ ] **Step 3: Run the bundler build + patch and inspect the result**

Run:
```powershell
cd wasm; npm run build; cd ..
Get-Content wasm\pkg\package.json
```
Expected: `package.json` shows `"name": "@dignetwork/datalayer-driver-wasm"`, version `3.0.0`, `types` pointing at the typed `.d.ts`, and the `.d.ts` present in `pkg/`.

- [ ] **Step 4: Commit**

```powershell
git add wasm/scripts/patch-pkg.mjs wasm/types/datalayer-driver-wasm.d.ts
git commit -m "build(wasm): scoped package-name patch and typed d.ts"
```

---

## Task 10: CI — build, test, and publish the WASM package

**Files:**
- Modify: `.github/workflows/CI.yml`

- [ ] **Step 1: Add the `build-wasm` job**

Insert after the existing `build` job (a sibling job, gated on `rust-checks`):
```yaml
  build-wasm:
    name: Build & test WASM bindings
    needs: rust-checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup node
        uses: actions/setup-node@v4
        with:
          node-version: 20
      - name: Install Rust + wasm target
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: stable
          targets: wasm32-unknown-unknown
      - name: Install clang/llvm (blst needs it for wasm32)
        run: sudo apt-get update && sudo apt-get install -y clang lld
      - name: Install wasm-pack
        uses: jetli/wasm-pack-action@v0.4.0
        with:
          version: latest
      - name: Build NAPI (for parity test)
        run: cd napi && npm install && npm run build
      - name: Build WASM (bundler, published artifact)
        run: cd wasm && npm install && npm run build
      - name: Parity test (nodejs target vs NAPI)
        run: cd wasm && npm test
      - name: Upload wasm pkg
        uses: actions/upload-artifact@v4
        with:
          name: wasm-pkg
          path: wasm/pkg
          if-no-files-found: error
```

- [ ] **Step 2: Add the `publish-wasm-npm` job**

Mirror the existing `publish-npm` version-tag gating (commit message is a bare semver):
```yaml
  publish-wasm-npm:
    name: Publish WASM to NPM
    runs-on: ubuntu-latest
    needs:
      - build-wasm
    steps:
      - uses: actions/checkout@v4
      - name: Setup node
        uses: actions/setup-node@v4
        with:
          node-version: 20
      - name: Download wasm pkg
        uses: actions/download-artifact@v4
        with:
          name: wasm-pkg
          path: wasm/pkg
      - name: Publish to NPM
        run: |
          cd wasm/pkg
          npm config set provenance true
          if git log -1 --pretty=%B | grep "^[0-9]\+\.[0-9]\+\.[0-9]\+$";
          then
            echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" >> ~/.npmrc
            npm publish --access public
          elif git log -1 --pretty=%B | grep "^[0-9]\+\.[0-9]\+\.[0-9]\+";
          then
            echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" >> ~/.npmrc
            npm publish --tag next --access public
          else
            echo "Not a release, skipping wasm npm publish"
          fi
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          NPM_TOKEN: ${{ secrets.NPM_TOKEN }}
```
Note: `git log` inside `wasm/pkg` works because the artifact is downloaded into a checked-out repo. If the `.git` context is unavailable there, run the `git log` check from the repo root and `cd wasm/pkg` only for `npm publish`.

- [ ] **Step 3: Validate the workflow YAML**

Run:
```powershell
python -c "import yaml,sys; yaml.safe_load(open('.github/workflows/CI.yml')); print('YAML OK')"
```
Expected: `YAML OK`. (If Python is unavailable, use any YAML linter or push to a branch and confirm Actions parses it.)

- [ ] **Step 4: Commit**

```powershell
git add .github/workflows/CI.yml
git commit -m "ci: build, parity-test, and publish wasm npm package"
```

---

## Task 11: Docs and final parity self-check

**Files:**
- Create: `wasm/README.md`
- Modify: `README.md` (root) — add a WASM section

- [ ] **Step 1: Write `wasm/README.md`**

Document: install (`npm i @dignetwork/datalayer-driver-wasm`), `init()` requirement, that it is offline-only (no Peer/networking), a `mintStore` → `signCoinSpends` → `spendBundleToHex` example, and the bundler-target note (works in webpack/vite/next; for plain Node use a separate build).

- [ ] **Step 2: Add a short WASM pointer to the root `README.md`**

One paragraph linking to `wasm/README.md`, stating the WASM package mirrors the offline NAPI surface for browser/bundler use.

- [ ] **Step 3: Final full build + test from a clean target dir**

Run:
```powershell
cargo build -p datalayer-driver --target wasm32-unknown-unknown --no-default-features
cargo build -p datalayer-driver-napi
cd wasm; npm run build; npm test; cd ..
```
Expected: all succeed; parity test prints `All parity checks passed.`

- [ ] **Step 4: Commit**

```powershell
git add wasm/README.md README.md
git commit -m "docs(wasm): usage and parity notes"
```

---

## Self-Review

**Spec coverage:**
- §3.1 crate layout → Task 3 (scaffold), Task 4/5/6/7 (src files), Task 8/9 (package, scripts, tests). ✓
- §3.2 `native` feature gating → Task 1. ✓
- §3.3 wasm deps → Task 3 Step 2. ✓
- §4 type mapping (Uint8Array, bigint, serde_bytes, camelCase, bigint serializer) → Task 4 Step 1–3. ✓ (BigInt-serializer correctness explicitly handled — supersedes the reference's lossy plain `to_value`.)
- §4.1 function/type inventory → Priority-1 in Task 6, supporting in Tasks 5 & 7; structs in Task 4 & 7. ✓
- §5 build/packaging (`--target bundler`, scoped name, package.json patch) → Task 9. ✓
- §6 CI (clang, wasm target, wasm-pack, version-tag publish) → Task 10. ✓
- §7 parity testing vs NAPI incl. DIGStore bundles → Task 8. ✓
- §8 risks (chia-wallet-sdk wasm32, getrandom, tokio leakage, bundler-not-node) → Task 1 Step 7, Task 2, Task 8 (separate node build). ✓

**Placeholder scan:** Two intentional `todo!()`/`None`-with-note points exist — `InnerSpendInfo::to_native` (Task 6 Step 3) and the `size_proof` parameter handling — because the exact `DataStoreInnerSpend` shape and whether tests exercise `size_proof` must be read from the real source during execution (Task 6 Step 1 instructs this). These are guarded by explicit "fill in per the real definition" instructions and a build that fails until resolved, not silent gaps.

**Type consistency:** Boundary struct names (`Coin`, `CoinSpend`, `DataStore`, `DataStoreMetadata`, `DelegatedPuzzle`, `Proof`, `SuccessResponse`, `ServerCoin`, `NewServerCoin`, `Output`, `InnerSpendInfo`) and helpers (`to_js`, `from_js`, `bytes32`, `public_key`, `secret_key`, `signature`, `coins_from_js`, `delegated_puzzles_from_js`, `coin_spends_from_js`, `coin_spends_to_js`) are used consistently across Tasks 4–7. JS export names are camelCase (`mintStore`, `signCoinSpends`, …) matching NAPI throughout.

**Open items the executor MUST verify against live source (flagged in-task, not assumed):**
- `RustSuccessResponse` field name `new_datastore` (Task 4 Step 3).
- Exact `DataStoreInnerSpend` definition (Task 6 Step 1/3).
- `mint_store`/`update_store_metadata` `size_proof` parameter presence (Task 6).
- `SpendBundle::new` constructor (Task 7 Step 2).
- `NewXchServerCoin` field names (Task 7 Step 3).
- `morph_launcher_id_wrapper` export symbol (Task 5 Step 1).
