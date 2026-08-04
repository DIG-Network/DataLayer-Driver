# Changelog

## 4.0.0

- Migrate the chia-family dependencies from the 0.30 to the 0.34 family
  (dig_ecosystem#2133): `chia-wallet-sdk` 0.34.0, `chia-protocol` 0.36.1,
  `chia-puzzles` 0.20.3, `clvm-traits` 0.36.1, `clvmr` 0.16.2.
- Replace the `chia` umbrella crate (which has no release depending on
  chia-protocol 0.36.x) with the individual `chia-protocol`, `chia-bls`,
  `chia-consensus`, `chia-traits` and `clvm-utils` sub-crates.
- Puzzle Rust types (`Proof`, `EveProof`, `LineageProof`, `cat`, `nft`,
  `standard`, `DeriveSynthetic`) moved from `chia-puzzles` to the new
  `chia-puzzle-types` crate; `SINGLETON_LAUNCHER_HASH` and other bytecode
  constants remain in `chia-puzzles`.
- `chia_consensus::solution_generator` now returns
  `chia_consensus::error::Error` instead of `std::io::Error`; the `WalletError`
  `Io` variant is replaced by a `Consensus` variant.
- Add a peer-simulator custody KAT pinning `DataStore::from_spend`'s owner-melt
  signal (`Err(DriverError::MissingChild)`), verified unchanged under 0.34.

BREAKING CHANGE: the public chia-family dependency versions changed (major bump).
