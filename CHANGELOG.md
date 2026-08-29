# Changelog

## 6.0.0

### Removed (BREAKING)

- `DigCollateralCoin::morph_store_launcher_id_for_mirror` and
  `DigCollateralCoin::create_mirror`. Mirror collateral is owned by the
  `dig-mirror-coin` crate; use `dig_mirror_coin::mirror_hint` and
  `dig_mirror_coin::create` instead.

  These were removed rather than renamed because they did not merely share the
  `DIG_STORE_MIRROR_COLLATERAL` tag with `dig-mirror-coin` — they occupied the
  *same* namespace. `dig-mirror-coin` derives a mirror hint from
  `morph(store + root + owner + epoch)` under that tag; this crate derived one
  from `morph(store + epoch)`. Because both hash an additive sum, the extra
  terms are absorbed rather than separating the two, so an author who chooses
  the epoch freely can solve `e' = store + epoch - store' - root' - owner'` and
  land a coin bonding their own store and root exactly on a hint derived here.

  `dig-mirror-coin` defeats that one level up: its coins **declare** their four
  terms and `MirrorCoin::advertises` checks the declaration term by term as well
  as recomputing the hint. The two-term form had no such check and could not
  gain one, because the epoch a coin was really built with is not recoverable
  from its hint.

  Coins already minted through the removed path are unaffected.
  `DigCollateralCoin::from_coin_state` reads the morphed id out of the coin's
  memos and never recomputes it, and `DigCollateralCoin::spend` does not use the
  hint, so existing mirror coins remain readable and spendable.

- `DigCollateralCoin::morph_store_launcher_id_for_collateral` and the
  store-collateral path are unchanged.

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
