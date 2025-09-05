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
    server_coin, wallet, rust,
    master_to_wallet_unhardened, 
    PublicKey, SecretKey, Signature,
    Bytes, Bytes32, Coin, CoinSpend, CoinState, Program, SpendBundle, Proof,
    EveProof, LineageProof,
    DataStoreInfo, BlsPair, SimulatorPuzzle, ServerCoin,
    constants,
    master_public_key_to_wallet_synthetic_key, master_public_key_to_first_puzzle_hash,
    master_secret_key_to_wallet_synthetic_secret_key, secret_key_to_public_key,
    synthetic_key_to_puzzle_hash, admin_delegated_puzzle_from_key,
    writer_delegated_puzzle_from_key, oracle_delegated_puzzle, get_coin_id,
};

// Re-export the NAPI bindings
pub use napi_lib::*;