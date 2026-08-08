# DataLayer-Driver WASM Bindings — Design

**Date:** 2026-05-29
**Status:** Approved (design), pending implementation plan
**Author:** Michael Taylor (with Claude Code)

## 1. Goal

Add WebAssembly bindings to `DataLayer-Driver` that mirror the existing NAPI
interface for the **offline** (non-networking) subset of the API, and publish
them as an npm package via CI.

**Primary success criterion:** build and sign **DIGStore (DataStore) spend
bundles** entirely in WASM (browser / bundler environments), producing byte-for-byte
identical output to the NAPI bindings. Everything else (key/address utilities,
coin selection, hex helpers) is supporting cast for that workflow.

Non-goals:
- Networking from WASM (no `Peer`, no `Tls`, no `connect_*`, no `sync_*`, no
  `broadcast_*`). WASM cannot open native TCP sockets or do native TLS. Chain
  reads/writes remain the JS consumer's responsibility.
- Async peer methods (`mint_nft`, `generate_did_proof*` async variants,
  `simulator_*`).

## 2. Why offline-only

The NAPI surface splits cleanly in two:

1. **Offline functions** — key derivation, address conversion, coin selection,
   DataStore/server-coin spend construction, signing, hex/cost/id helpers,
   genesis constants. Pure CLVM + BLS; no IO. Port to WASM cleanly.
2. **`Peer` + `Tls` (~24 async methods)** — native TCP socket + native TLS +
   multi-threaded tokio + filesystem cert loading. None of these exist in
   `wasm32-unknown-unknown`.

The reference project `chia-scaled-parallel-voting` (same org) solved this by
putting **zero networking in WASM** and gating native code behind a `native`
cargo feature. We follow that proven pattern.

## 3. Architecture

### 3.1 Crate layout

New workspace member `wasm/`, sibling to the existing `napi/`:

```
DataLayer-Driver/
  Cargo.toml            # workspace members = [".", "napi", "wasm"]
  src/                  # core crate — gains a `native` feature
  napi/                 # unchanged (keeps default features → native on)
  wasm/
    Cargo.toml          # crate-type = ["cdylib", "rlib"]
    build.rs            # (optional) none required for wasm-bindgen
    src/
      lib.rs            # #[wasm_bindgen] glue (analog of napi/src/napi_lib.rs)
      conversions.rs    # JS <-> Rust type bridging (analog of napi/src/conversions.rs)
    tests/              # node parity tests (run in CI)
    package.json        # publish metadata override for @dignetwork/datalayer-driver-wasm
```

### 3.2 Core crate `native` feature

`datalayer-driver/Cargo.toml`:

```toml
[features]
default = ["native"]
native = ["chia-wallet-sdk/native-tls", "chia-wallet-sdk/peer-simulator", "dep:tokio"]
```

- `chia-wallet-sdk` keeps `chip-0035` + `action-layer` unconditionally (pure
  puzzle/driver logic, wasm-safe); `native-tls` + `peer-simulator` move under
  `native`.
- `tokio` becomes optional, pulled only by `native`.
- Gate behind `#[cfg(feature = "native")]`:
  - the entire `async_api` module (`src/lib.rs`)
  - every `&Peer` async fn in `src/wallet.rs` (15+: `get_unspent_coin_states*`,
    `spend_xch_server_coins`, `fetch_xch_server_coin`, `sync_store*`,
    `get_store_creation_height`, `broadcast_spend_bundle`, `get_header_hash`,
    `get_fee_estimate`, `is_coin_spent`, `look_up_possible_launchers`,
    `subscribe_/unsubscribe_to_coin_states`, `mint_nft`,
    `generate_did_proof*`, `resolve_did_string_and_generate_proof`)
  - the `use chia_wallet_sdk::client::Peer;` imports and the `pub use ...Peer`
    re-export in `src/lib.rs`
  - `&Peer`/async items in `src/dig_coin.rs` and `src/dig_collateral_coin.rs`
- The `napi/` crate keeps `datalayer-driver` with default features → no behavior
  change, no CI regression for the native build.

The WASM crate depends on the core crate with networking off:

```toml
datalayer-driver = { path = "..", default-features = false }
```

### 3.3 WASM crate dependencies

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
datalayer-driver = { path = "..", default-features = false }
wasm-bindgen = "0.2"
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde_bytes = "0.11"
serde-wasm-bindgen = "0.6"
console_error_panic_hook = { version = "0.1", optional = true }
hex = "0.4"
# getrandom feature/version resolved against what `chia 0.26` pulls (see Risks)

[features]
default = ["console-panic-hook"]
console-panic-hook = ["dep:console_error_panic_hook"]
```

No `wasm-bindgen-futures` / `js-sys::Promise` needed — offline functions are
synchronous.

## 4. JS interface & type mapping

Goal: a JS shape as close to the NAPI package as the WASM toolchain allows, so
existing NAPI consumers can switch with minimal changes.

| NAPI | WASM |
|---|---|
| `Buffer` (fn param/return) | `&[u8]` / `Vec<u8>` ↔ `Uint8Array` (native wasm-bindgen) |
| `BigInt` (u64 amounts) | `u64` ↔ `bigint` (native wasm-bindgen) |
| struct field: byte buffer | `#[serde(with = "serde_bytes")] Vec<u8>` → `Uint8Array` |
| struct field: u64 | serde-wasm-bindgen serializer with `serialize_large_number_types_as_bigints(true)` → `bigint` |
| struct (JS object, camelCase) | `#[serde(rename_all = "camelCase")]` + `serde_wasm_bindgen::to_value` / `from_value` |
| `Option<T>` | `T | null | undefined` |
| `napi::Result<T>` (throws) | `Result<T, JsError>` (throws) |

- **Amounts (mojos) MUST be `bigint`/`BigInt`, never JS `number`** — values exceed
  2^53. This matches NAPI and is non-negotiable for correctness.
- 32-byte values are `Uint8Array` (matching NAPI `Buffer`), not hex strings, to
  keep the interface identical. (Hex helpers remain available where NAPI exposed
  them, e.g. `spend_bundle_to_hex`.)
- An `init()` export installs `console_error_panic_hook` (dev-friendly panics).
  Idempotent; safe to call once at startup.

### 4.1 Function & type inventory (offline subset, NAPI names preserved)

Priority 1 — DIGStore spend bundles (must-have):
`mint_store`, `update_store_metadata`, `update_store_ownership`, `oracle_spend`,
`melt_store`, `sign_coin_spends`, `spend_bundle_to_hex`,
`hex_spend_bundle_to_coin_spends`, `get_cost`, `get_coin_id`, `select_coins`.

Priority 2 — supporting:
`master_public_key_to_wallet_synthetic_key`,
`master_public_key_to_first_puzzle_hash`,
`master_secret_key_to_wallet_synthetic_secret_key`,
`secret_key_to_public_key`, `synthetic_key_to_puzzle_hash`,
`puzzle_hash_to_address`, `address_to_puzzle_hash`,
`admin_delegated_puzzle_from_key`, `writer_delegated_puzzle_from_key`,
`new_lineage_proof`, `new_eve_proof`, `morph_launcher_id`,
`create_server_coin`, `send_xch`, `add_fee`, `sign_message`,
`verify_signed_message`, `get_mainnet_genesis_challenge`,
`get_testnet11_genesis_challenge`.

Structs (camelCase JS objects): `Coin`, `CoinState`, `CoinSpend`,
`LineageProof`, `EveProof`, `Proof`, `ServerCoin`, `DataStoreMetadata`,
`DelegatedPuzzle`, `DataStore`, `SuccessResponse`, `NewServerCoin`, `Output`.

`DelegatedPuzzle` and `Proof` are enums/unions in the core crate — mirror the
NAPI object representation (`Proof { lineageProof?, eveProof? }`;
`DelegatedPuzzle` admin/writer/oracle variants) so JS shapes match.

## 5. Build & packaging

- Build command: `wasm-pack build wasm --target bundler --release` → `wasm/pkg/`.
- Published package: **`@dignetwork/datalayer-driver-wasm`**, version mirrors the
  root crate (currently `3.0.0`).
- `wasm-pack` derives the npm package name from the crate name; override to the
  scoped name via `wasm-pack build --scope dignetwork` and/or a post-build patch
  of `pkg/package.json` (name, repository, license, `files`). The patch step
  lives in a small script invoked by CI so the published metadata is
  deterministic.
- `pkg/` ships: `*_bg.wasm`, `*.js`, `*.d.ts`, `package.json`, `README`, `LICENSE`.

## 6. CI

Extend `.github/workflows/CI.yml` (do not disturb existing napi build/test/publish
jobs):

New `build-wasm` job (ubuntu-latest):
1. Install LLVM/clang (`blst` / `chia-bls` needs a C/Clang toolchain to compile to
   `wasm32`).
2. `rustup target add wasm32-unknown-unknown`.
3. `cargo install wasm-pack` (or use the `jetli/wasm-pack-action`).
4. `wasm-pack build wasm --target bundler --release`.
5. Build a second `--target nodejs` artifact into `wasm/pkg-node/` for tests.
6. Run node parity tests (Section 7).

New publish step (gated on the same version-tag condition the napi publish uses):
`npm publish` from `wasm/pkg/` with `NODE_AUTH_TOKEN` / `NPM_TOKEN`. Runs only on
release/tag, after tests pass.

## 7. Testing — parity against NAPI

The strongest validation of "exact same interface" is differential testing: CI
already builds the NAPI `.node`. Add a node test that loads **both** the NAPI
package and the `--target nodejs` WASM build, and asserts equality across the
offline surface:

- address round-trip (`puzzle_hash_to_address` / `address_to_puzzle_hash`)
- key derivation (all `*_key`/`*_puzzle_hash` helpers) — equal `Uint8Array`s
- coin id, cost
- **DIGStore spend bundles (primary):** construct identical inputs, call
  `mint_store`, `update_store_metadata`, `update_store_ownership`, `melt_store`,
  `oracle_spend` on both, `sign_coin_spends`, then compare the resulting
  `CoinSpend[]` / signed bundle hex via `spend_bundle_to_hex`. Bytes must match
  exactly.

Test fixtures use deterministic secret keys (e.g. `[1u8;32]`) so output is stable
and comparable. No network access required.

## 8. Risks & mitigations

1. **`chia-wallet-sdk` 0.30 wasm32 compile** without native features — likely OK
   (reference proved 0.24); verify with a throwaway
   `cargo build -p datalayer-driver --target wasm32-unknown-unknown --no-default-features`
   as the first implementation step. If it fails, identify the offending
   transitive feature and gate it.
2. **`getrandom` version mismatch** — `chia 0.26` may pull getrandom 0.2 (needs
   `features = ["js"]`) or 0.3 (needs `features = ["wasm_js"]` + `RUSTFLAGS`
   `--cfg getrandom_backend="wasm_js"`). Resolve at first build by inspecting
   `cargo tree -i getrandom`.
3. **tokio leakage into offline paths** — if any offline function transitively
   references a tokio type, gating expands. Caught by the wasm32 build in step 1.
4. **bundler output is not node-`require`-able** — hence the separate
   `--target nodejs` build for CI tests; the published artifact stays `bundler`.
5. **Package name override** — wasm-pack defaults to the crate name; CI must patch
   `pkg/package.json` to the scoped name before publish.

## 9. Out of scope / future

- A `JsChainBackend`-style callback transport that would let WASM drive `Peer`
  methods over a JS-supplied async backend (fetch/RPC). Deferred; can be a future
  spec if browser-side chain interaction is needed.
- Additional wasm-pack targets (`web`, `nodejs`) as published artifacts. Only
  `bundler` is published in v1.
