# getrandom backend resolution (wasm32)

Determined 2026-05-29 via `cargo tree -i getrandom --target wasm32-unknown-unknown --no-default-features -p datalayer-driver`.

Two versions exist in the overall lockfile (**0.2.16** and **0.3.3**), but only **0.2.16** is present in the `wasm32-unknown-unknown` dependency graph. `getrandom@0.3.3` returns "nothing to print" for the wasm target — it is pulled only on the host/native target.

`getrandom 0.2.16` reaches the wasm build via `chia-sdk-driver`, `clvm_tools_rs` (→ `chia-sdk-types`), and the `rand_core 0.6 → crypto-bigint → k256` chain.

## Resolution for the `wasm` crate

`wasm/Cargo.toml` must declare:

```toml
getrandom = { version = "0.2", features = ["js"] }
```

**No `.cargo/config.toml` is required** (the `wasm_js` backend + `RUSTFLAGS` cfg is a getrandom 0.3 concern, and 0.3 is not on the wasm path).

Without the `js` feature, `getrandom 0.2` compiles on wasm32 but panics at runtime on the first random-bytes call (e.g. inside k256/BLS code paths). The `js` feature wires it to the browser/Node crypto API.

## Why this is safe for the offline path

`getrandom 0.2.16` is pulled transitively into the `wasm32-unknown-unknown` build by: `chia-sdk-driver` (default feature), `clvm_tools_rs` (via `chia-sdk-types`), and the `rand_core 0.6 → crypto-bigint → elliptic-curve → k256` chain (plus `datalayer-driver-wasm` itself declaring the `js` feature directly to satisfy the transitive requirement).

The offline DIGStore builders and BLS signing are deterministic — the Node parity test produces byte-identical signatures and spend bundles across two independent runtimes (NAPI native vs WASM), which it could not if any exercised path consumed RNG. The `js` feature is required so the transitive getrandom 0.2 compiles for `wasm32-unknown-unknown` and resolves at runtime under the shipped `--target nodejs`/`--target bundler` outputs (which provide the JS crypto global).

## Update: chia-sdk 0.34 puts getrandom 0.3 on the wasm path

Since the bump to `chia-sdk-*` 0.34, `chia-sdk-driver` depends on `getrandom` 0.3 directly, so both
majors now reach the wasm build. 0.3 refuses to compile for `wasm32-unknown-unknown` unless a
backend is named, and the feature alone is not enough — it also needs a cfg flag. Both halves are
committed:

- `wasm/Cargo.toml` takes `getrandom` 0.3 under the alias `getrandom_v03` with the `wasm_js` feature
  (alongside the existing 0.2 dependency with `js`), so the feature is enabled on the real crate in
  the graph.
- `.cargo/config.toml` sets `--cfg getrandom_backend="wasm_js"` for the `wasm32-unknown-unknown`
  target only, which is what actually selects the browser backend.

Removing either one turns the wasm build into a `compile_error!` from inside `getrandom`.
