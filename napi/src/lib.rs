#![allow(unexpected_cfgs)]

// Re-export the NAPI macros for this crate
#[macro_use]
extern crate napi_derive;

// Module declarations
mod conversions;
mod js;
mod napi_lib;

// Re-export only necessary types from the main crate
pub use datalayer_driver::{
    admin_delegated_puzzle_from_key, constants, get_coin_id,
    master_public_key_to_first_puzzle_hash, master_public_key_to_wallet_synthetic_key,
    master_secret_key_to_wallet_synthetic_secret_key, master_to_wallet_unhardened,
    oracle_delegated_puzzle, rust, secret_key_to_public_key, server_coin,
    synthetic_key_to_puzzle_hash, wallet, writer_delegated_puzzle_from_key, BlsPair, Bytes,
    Bytes32, Coin, CoinSpend, CoinState, DataStoreInfo, EveProof, LineageProof, Program, Proof,
    PublicKey, SecretKey, ServerCoin, Signature, SimulatorPuzzle, SpendBundle,
};

// Re-export the NAPI bindings
pub use napi_lib::*;
