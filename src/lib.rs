//! # DataLayer Driver
//!
//! Native Chia DataLayer Driver for storing and retrieving data in Chia blockchain.
//!
//! This crate provides Rust APIs for interacting with Chia's DataLayer,
//! including minting, updating, and syncing data stores.
//!
//! ## Features
//!
//! - Mint new data stores
//! - Update store metadata and ownership
//! - Sync stores from the blockchain
//! - Oracle spend functionality
//! - Server coin management
//! - Fee management utilities

// Re-export core types from dependencies
pub use chia::bls::{master_to_wallet_unhardened, PublicKey, SecretKey, Signature};
pub use chia::protocol::{Bytes, Bytes32, Coin, CoinSpend, CoinState, Program, SpendBundle};
pub use chia::puzzles::{EveProof, LineageProof, Proof};
pub use chia_wallet_sdk::client::Peer;
pub use chia_wallet_sdk::driver::{DataStore, DataStoreInfo, DataStoreMetadata, DelegatedPuzzle};

// Internal modules
pub mod rust;
pub mod server_coin;
pub mod wallet;

// Re-export types from internal modules
pub use rust::{BlsPair, SimulatorPuzzle, UnspentCoinsResponse};
pub use server_coin::{morph_launcher_id, ServerCoin};
pub use wallet::{
    add_fee, create_server_coin, create_simple_did, generate_did_proof,
    generate_did_proof_from_chain, generate_did_proof_manual, get_cost, get_fee_estimate,
    get_header_hash, get_store_creation_height, get_unspent_coin_states, is_coin_spent,
    look_up_possible_launchers, melt_store, mint_nft, mint_store, oracle_spend, select_coins,
    send_xch, sign_coin_spends, sign_message, spend_server_coins, subscribe_to_coin_states,
    sync_store, sync_store_using_launcher_id, unsubscribe_from_coin_states, update_store_metadata,
    update_store_ownership, verify_signature, DataStoreInnerSpend, NewServerCoin,
    PossibleLaunchersResponse, SuccessResponse, SyncStoreResponse, TargetNetwork,
    UnspentCoinStates,
};

// Type aliases for convenience
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Helper functions for common conversions
use chia::puzzles::{standard::StandardArgs, DeriveSynthetic};

/// Converts a master public key to a wallet synthetic key.
pub fn master_public_key_to_wallet_synthetic_key(public_key: &PublicKey) -> PublicKey {
    master_to_wallet_unhardened(public_key, 0).derive_synthetic()
}

/// Converts a master public key to the first puzzle hash.
pub fn master_public_key_to_first_puzzle_hash(public_key: &PublicKey) -> Bytes32 {
    let wallet_pk = master_to_wallet_unhardened(public_key, 0).derive_synthetic();
    StandardArgs::curry_tree_hash(wallet_pk).into()
}

/// Converts a master secret key to a wallet synthetic secret key.
pub fn master_secret_key_to_wallet_synthetic_secret_key(secret_key: &SecretKey) -> SecretKey {
    master_to_wallet_unhardened(secret_key, 0).derive_synthetic()
}

/// Converts a secret key to its corresponding public key.
pub fn secret_key_to_public_key(secret_key: &SecretKey) -> PublicKey {
    secret_key.public_key()
}

/// Converts a synthetic key to its corresponding standard puzzle hash.
pub fn synthetic_key_to_puzzle_hash(synthetic_key: &PublicKey) -> Bytes32 {
    StandardArgs::curry_tree_hash(*synthetic_key).into()
}

/// Creates an admin delegated puzzle for a given key.
pub fn admin_delegated_puzzle_from_key(synthetic_key: &PublicKey) -> DelegatedPuzzle {
    DelegatedPuzzle::Admin(StandardArgs::curry_tree_hash(*synthetic_key))
}

/// Creates a writer delegated puzzle from a given key.
pub fn writer_delegated_puzzle_from_key(synthetic_key: &PublicKey) -> DelegatedPuzzle {
    DelegatedPuzzle::Writer(StandardArgs::curry_tree_hash(*synthetic_key))
}

/// Creates an oracle delegated puzzle.
pub fn oracle_delegated_puzzle(oracle_puzzle_hash: Bytes32, oracle_fee: u64) -> DelegatedPuzzle {
    DelegatedPuzzle::Oracle(oracle_puzzle_hash, oracle_fee)
}

/// Gets the coin ID for a given coin.
pub fn get_coin_id(coin: &Coin) -> Bytes32 {
    coin.coin_id()
}

/// Constants for different networks
pub mod constants {
    use chia_wallet_sdk::types::{MAINNET_CONSTANTS, TESTNET11_CONSTANTS};

    /// Returns the mainnet genesis challenge.
    pub fn get_mainnet_genesis_challenge() -> chia::protocol::Bytes32 {
        MAINNET_CONSTANTS.genesis_challenge
    }

    /// Returns the testnet11 genesis challenge.
    pub fn get_testnet11_genesis_challenge() -> chia::protocol::Bytes32 {
        TESTNET11_CONSTANTS.genesis_challenge
    }
}
