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
pub use chia_wallet_sdk::utils::Address;

// Re-export async_api and constants modules at the top level for convenience
pub use async_api::{NetworkType, connect_random, create_tls_connector, connect_peer_rust};
pub use constants::{get_mainnet_genesis_challenge, get_testnet11_genesis_challenge};

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

/// Converts a puzzle hash to an address by encoding it using bech32m.
pub fn puzzle_hash_to_address(puzzle_hash: Bytes32, prefix: &str) -> Result<String> {
    use chia_wallet_sdk::utils::Address;
    Ok(Address::new(puzzle_hash, prefix.to_string()).encode()?)
}

/// Converts an address to a puzzle hash using bech32m.
pub fn address_to_puzzle_hash(address: &str) -> Result<Bytes32> {
    use chia_wallet_sdk::utils::Address;
    Ok(Address::decode(address)?.puzzle_hash)
}

/// Converts hex-encoded spend bundle to coin spends.
pub fn hex_spend_bundle_to_coin_spends(hex: &str) -> Result<Vec<CoinSpend>> {
    use chia::traits::Streamable;
    let bytes = hex::decode(hex)?;
    let spend_bundle = SpendBundle::from_bytes(&bytes)?;
    Ok(spend_bundle.coin_spends)
}

/// Converts a spend bundle to hex encoding.
pub fn spend_bundle_to_hex(spend_bundle: &SpendBundle) -> Result<String> {
    use chia::traits::Streamable;
    let bytes = spend_bundle.to_bytes()?;
    Ok(hex::encode(bytes))
}

/// Adds an offset to a launcher id to make it deterministically unique from the original.
pub fn morph_launcher_id_wrapper(launcher_id: Bytes32, offset: u64) -> Bytes32 {
    server_coin::morph_launcher_id(launcher_id, &offset.into())
}

/// Output for send_xch function
#[derive(Debug, Clone)]
pub struct Output {
    pub puzzle_hash: Bytes32,
    pub amount: u64,
    pub memos: Vec<Bytes>,
}

/// Sends XCH to a given set of puzzle hashes (Rust API version).
pub fn send_xch_rust(
    synthetic_key: &PublicKey,
    selected_coins: &[Coin],
    outputs: &[Output],
    fee: u64,
) -> Result<Vec<CoinSpend>> {
    let outputs: Vec<(Bytes32, u64, Vec<Bytes>)> = outputs
                .iter()
        .map(|output| (output.puzzle_hash, output.amount, output.memos.clone()))
        .collect();
    
    Ok(wallet::send_xch(*synthetic_key, selected_coins, &outputs, fee)?)
}

/// Selects coins using the knapsack algorithm (Rust API version).
pub fn select_coins_rust(all_coins: &[Coin], total_amount: u64) -> Result<Vec<Coin>> {
    Ok(wallet::select_coins(all_coins.to_vec(), total_amount)?)
}

/// Adds a fee to any transaction (Rust API version).
pub fn add_fee_rust(
    spender_synthetic_key: &PublicKey,
    selected_coins: &[Coin],
    assert_coin_ids: &[Bytes32],
    fee: u64,
) -> Result<Vec<CoinSpend>> {
    Ok(wallet::add_fee(
        *spender_synthetic_key,
        selected_coins.to_vec(),
        assert_coin_ids.to_vec(),
        fee,
    )?)
}

/// Signs coin spends using a list of keys (Rust API version).
pub fn sign_coin_spends_rust(
    coin_spends: &[CoinSpend],
    private_keys: &[SecretKey],
        for_testnet: bool,
) -> Result<Signature> {
    Ok(wallet::sign_coin_spends(
        coin_spends.to_vec(),
        private_keys.to_vec(),
            if for_testnet {
            wallet::TargetNetwork::Testnet11
            } else {
            wallet::TargetNetwork::Mainnet
        },
    )?)
}

/// Signs a message using the provided private key (Rust API version).
pub fn sign_message_rust(message: &[u8], private_key: &SecretKey) -> Result<Signature> {
    Ok(wallet::sign_message(message.into(), private_key.clone())?)
}

/// Verifies a signed message using the provided public key (Rust API version).
pub fn verify_signed_message_rust(
    signature: &Signature,
    public_key: &PublicKey,
    message: &[u8],
) -> Result<bool> {
    Ok(wallet::verify_signature(message.into(), *public_key, signature.clone())?)
}

/// Calculates the total cost of coin spends (Rust API version).
pub fn get_cost_rust(coin_spends: &[CoinSpend]) -> Result<u64> {
    Ok(wallet::get_cost(coin_spends.to_vec())?)
}

/// Mints a new datastore (Rust API version).
#[allow(clippy::too_many_arguments)]
pub fn mint_store_rust(
    minter_synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    root_hash: Bytes32,
    label: Option<String>,
    description: Option<String>,
    bytes: Option<u64>,
    owner_puzzle_hash: Bytes32,
    delegated_puzzles: Vec<DelegatedPuzzle>,
    fee: u64,
) -> Result<SuccessResponse> {
    Ok(wallet::mint_store(
        minter_synthetic_key,
        selected_coins,
        root_hash,
        label,
        description,
        bytes,
        owner_puzzle_hash,
        delegated_puzzles,
        fee,
    )?)
}

/// Spends a store in oracle mode (Rust API version).
pub fn oracle_spend_rust(
    spender_synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    store: DataStore,
    fee: u64,
) -> Result<SuccessResponse> {
    Ok(wallet::oracle_spend(
        spender_synthetic_key,
        selected_coins,
        store,
        fee,
    )?)
}

/// Updates the metadata of a store (Rust API version).
#[allow(clippy::too_many_arguments)]
pub fn update_store_metadata_rust(
    store: DataStore,
    new_root_hash: Bytes32,
    new_label: Option<String>,
    new_description: Option<String>,
    new_bytes: Option<u64>,
    inner_spend_info: wallet::DataStoreInnerSpend,
) -> Result<SuccessResponse> {
    Ok(wallet::update_store_metadata(
        store,
        new_root_hash,
        new_label,
        new_description,
        new_bytes,
        inner_spend_info,
    )?)
}

/// Updates the ownership of a store (Rust API version).
pub fn update_store_ownership_rust(
    store: DataStore,
    new_owner_puzzle_hash: Bytes32,
    new_delegated_puzzles: Vec<DelegatedPuzzle>,
    inner_spend_info: wallet::DataStoreInnerSpend,
) -> Result<SuccessResponse> {
    Ok(wallet::update_store_ownership(
        store,
        new_owner_puzzle_hash,
        new_delegated_puzzles,
        inner_spend_info,
    )?)
}

/// Melts a store (Rust API version).
pub fn melt_store_rust(store: DataStore, owner_pk: PublicKey) -> Result<Vec<CoinSpend>> {
    Ok(wallet::melt_store(store, owner_pk)?)
}

/// Creates a server coin (Rust API version).
pub fn create_server_coin_rust(
    synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    hint: Bytes32,
    uris: Vec<String>,
    amount: u64,
    fee: u64,
) -> Result<NewServerCoin> {
    Ok(wallet::create_server_coin(
        synthetic_key,
        selected_coins,
        hint,
        uris,
        amount,
        fee,
    )?)
}

/// Async functions for blockchain interaction (Rust API versions)
pub mod async_api {
    use super::*;
    use chia_wallet_sdk::client::{connect_peer, create_native_tls_connector, load_ssl_cert, PeerOptions};
    use std::net::SocketAddr;
    use tokio::net::lookup_host;
    use rand::seq::SliceRandom;
    use futures_util::stream::{FuturesUnordered, StreamExt};
    use tokio::time::{timeout, Duration};

    // DNS introducers and default ports for connecting to random peers.
    const MAINNET_DNS_INTRODUCERS: &[&str] = &[
        "dns-introducer.chia.net",
        "chia.ctrlaltdel.ch", 
        "seeder.dexie.space",
        "chia.hoffmang.com",
    ];
    const TESTNET11_DNS_INTRODUCERS: &[&str] = &["dns-introducer-testnet11.chia.net"];
    const MAINNET_DEFAULT_PORT: u16 = 8444;
    const TESTNET11_DEFAULT_PORT: u16 = 58444;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum NetworkType {
        Mainnet,
        Testnet11,
    }

    /// Connects to a random peer on the specified network (Rust API version).
    ///
    /// The function performs DNS lookups using the network's introducers, picks a random
    /// address from the returned list, and attempts to establish a connection. It will
    /// try every resolved address until a connection succeeds.
    pub async fn connect_random(
        network: NetworkType,
        cert_path: &str,
        key_path: &str,
    ) -> Result<Peer> {
        // Load TLS certificate
        let cert = load_ssl_cert(cert_path, key_path)?;
        let tls = create_native_tls_connector(&cert)?;

        // Introducers and default port per network
        let (introducers, default_port) = match network {
            NetworkType::Mainnet => (MAINNET_DNS_INTRODUCERS, MAINNET_DEFAULT_PORT),
            NetworkType::Testnet11 => (TESTNET11_DNS_INTRODUCERS, TESTNET11_DEFAULT_PORT),
        };

        // Resolve all introducers to socket addresses
        let mut addrs = Vec::new();
        for introducer in introducers {
            if let Ok(iter) = lookup_host((*introducer, default_port)).await {
                addrs.extend(iter);
            }
        }

        if addrs.is_empty() {
            return Err("Failed to resolve any peer addresses from introducers".into());
        }

        // Shuffle for randomness so every call has different order
        {
            let mut rng = rand::thread_rng();
            addrs.shuffle(&mut rng);
        }

        // Try to connect in concurrent batches with timeout logic
        const BATCH_SIZE: usize = 10;
        const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

        for chunk in addrs.chunks(BATCH_SIZE) {
            let mut futures = FuturesUnordered::new();
            for addr in chunk {
                let addr = *addr;
                let network_str = match network {
                    NetworkType::Mainnet => "mainnet",
                    NetworkType::Testnet11 => "testnet11",
                };
                let tls_clone = tls.clone();
                
                // Spawn connection attempt with timeout
                futures.push(async move {
                    timeout(
                        CONNECT_TIMEOUT,
                        connect_peer(
                            network_str.to_string(),
                            tls_clone,
                            addr,
                            PeerOptions::default(),
                        ),
                    )
                    .await
                });
            }

            while let Some(result) = futures.next().await {
                match result {
                    Ok(Ok((peer, _receiver))) => {
                        // Successfully connected, return the peer
                        return Ok(peer);
                    }
                    _ => {
                        // Either timed out or failed; continue with others
                    }
                }
            }
        }

        Err("Unable to connect to any discovered peer".into())
    }

    /// Creates a TLS connector for Chia peer connections (Rust API version).
    pub fn create_tls_connector(cert_path: &str, key_path: &str) -> Result<chia_wallet_sdk::client::Connector> {
        let cert = load_ssl_cert(cert_path, key_path)?;
        Ok(create_native_tls_connector(&cert)?)
    }

    /// Connects to a specific peer address (Rust API version).
    pub async fn connect_peer_rust(
        network: NetworkType,
        tls_connector: chia_wallet_sdk::client::Connector,
        address: SocketAddr,
    ) -> Result<Peer> {
        let network_str = match network {
            NetworkType::Mainnet => "mainnet",
            NetworkType::Testnet11 => "testnet11",
        };

        let (peer, _receiver) = connect_peer(
            network_str.to_string(),
            tls_connector,
            address,
            PeerOptions::default(),
        )
        .await?;

        Ok(peer)
    }
    
    /// Mints a new NFT using a DID string (Rust API version).
    #[allow(clippy::too_many_arguments)]
    pub async fn mint_nft_rust(
        peer: &Peer,
        synthetic_key: PublicKey,
        selected_coins: Vec<Coin>,
        did_string: &str,
        recipient_puzzle_hash: Bytes32,
        metadata: chia::puzzles::nft::NftMetadata,
        royalty_puzzle_hash: Option<Bytes32>,
        royalty_basis_points: u16,
        fee: u64,
        for_testnet: Option<bool>,
    ) -> Result<Vec<CoinSpend>> {
        let network = if for_testnet.unwrap_or(false) {
            wallet::TargetNetwork::Testnet11
        } else {
            wallet::TargetNetwork::Mainnet
        };

        Ok(wallet::mint_nft(
            peer,
            synthetic_key,
            selected_coins,
            did_string,
            recipient_puzzle_hash,
            metadata,
            royalty_puzzle_hash,
            royalty_basis_points,
            fee,
            network,
        )
        .await?)
    }

    /// Generates a DID proof automatically (Rust API version).
    pub async fn generate_did_proof_rust(
        peer: &Peer,
        did_coin: Coin,
    for_testnet: bool,
    ) -> Result<(Proof, Coin)> {
        let network = if for_testnet {
            wallet::TargetNetwork::Testnet11
        } else {
            wallet::TargetNetwork::Mainnet
        };

        Ok(wallet::generate_did_proof(peer, did_coin, network).await?)
    }

    /// Creates a simple DID (Rust API version).
    pub fn create_simple_did_rust(
        synthetic_key: PublicKey,
        selected_coins: Vec<Coin>,
        fee: u64,
    ) -> Result<(Vec<CoinSpend>, Coin)> {
        Ok(wallet::create_simple_did(synthetic_key, selected_coins, fee)?)
    }

    /// Synchronizes a datastore (Rust API version).
    pub async fn sync_store_rust(
        peer: &Peer,
        store: &DataStore,
        last_height: Option<u32>,
        last_header_hash: Bytes32,
        with_history: bool,
    ) -> Result<SyncStoreResponse> {
        Ok(wallet::sync_store(peer, store, last_height, last_header_hash, with_history).await?)
    }

    /// Synchronizes a store using its launcher ID (Rust API version).
    pub async fn sync_store_from_launcher_id_rust(
        peer: &Peer,
        launcher_id: Bytes32,
        last_height: Option<u32>,
        last_header_hash: Bytes32,
        with_history: bool,
    ) -> Result<SyncStoreResponse> {
        Ok(wallet::sync_store_using_launcher_id(
            peer,
            launcher_id,
            last_height,
            last_header_hash,
            with_history,
        )
        .await?)
    }

    /// Gets all unspent coins for a puzzle hash (Rust API version).
    pub async fn get_all_unspent_coins_rust(
        peer: &Peer,
        puzzle_hash: Bytes32,
        previous_height: Option<u32>,
        previous_header_hash: Bytes32,
    ) -> Result<UnspentCoinStates> {
        Ok(wallet::get_unspent_coin_states(
            peer,
            puzzle_hash,
            previous_height,
            previous_header_hash,
            false,
        )
        .await?)
    }

    /// Checks if a coin is spent on-chain (Rust API version).
    pub async fn is_coin_spent_rust(
        peer: &Peer,
        coin_id: Bytes32,
        last_height: Option<u32>,
        header_hash: Bytes32,
    ) -> Result<bool> {
        Ok(wallet::is_coin_spent(peer, coin_id, last_height, header_hash).await?)
    }

    /// Gets the header hash at a specific height (Rust API version).
    pub async fn get_header_hash_rust(peer: &Peer, height: u32) -> Result<Bytes32> {
        Ok(wallet::get_header_hash(peer, height).await?)
    }

    /// Gets fee estimate for target time (Rust API version).
    pub async fn get_fee_estimate_rust(peer: &Peer, target_time_seconds: u64) -> Result<u64> {
        Ok(wallet::get_fee_estimate(peer, target_time_seconds).await?)
    }

    /// Broadcasts a spend bundle (Rust API version).
    pub async fn broadcast_spend_bundle_rust(
        peer: &Peer,
        spend_bundle: SpendBundle,
    ) -> Result<chia::protocol::TransactionAck> {
        Ok(wallet::broadcast_spend_bundle(peer, spend_bundle).await?)
    }
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

/// Example usage of the Rust API
#[cfg(test)]
mod examples {
    use super::*;
    
    #[test]
    fn example_key_operations() {
        // Example: Generate keys and addresses
        let secret_key = SecretKey::from_bytes(&[1u8; 32]).unwrap();
        let public_key = secret_key_to_public_key(&secret_key);
        let _synthetic_key = master_public_key_to_wallet_synthetic_key(&public_key);
        let puzzle_hash = master_public_key_to_first_puzzle_hash(&public_key);
        
        // Convert to address
        let address = puzzle_hash_to_address(puzzle_hash, "xch").unwrap();
        println!("Address: {}", address);
        
        // Convert back
        let decoded_hash = address_to_puzzle_hash(&address).unwrap();
        assert_eq!(puzzle_hash, decoded_hash);
    }
    
    #[tokio::test]
    async fn example_nft_minting() {
        // Example of complete NFT minting workflow using Rust API
        
        /*
        // 1. Connect to a random peer
        let peer = connect_random(
            NetworkType::Mainnet,
            "~/.chia/mainnet/config/ssl/wallet/wallet_node.crt",
            "~/.chia/mainnet/config/ssl/wallet/wallet_node.key"
        ).await.unwrap();

        // 2. Set up your wallet keys (from mnemonic or existing keys)
        let master_secret_key = SecretKey::from_bytes([1u8; 32]).unwrap(); // Your actual key
        let master_public_key = secret_key_to_public_key(&master_secret_key);
        let synthetic_key = master_public_key_to_wallet_synthetic_key(&master_public_key);
        let puzzle_hash = master_public_key_to_first_puzzle_hash(&master_public_key);

        // 3. Get unspent coins for the transaction
        let unspent_coins = async_api::get_all_unspent_coins_rust(
            &peer,
            puzzle_hash,
            None,
            get_mainnet_genesis_challenge(),
        ).await.unwrap();

        // 4. Select coins for the transaction
        let fee = 1_000_000; // 1 million mojos
        let selected_coins = select_coins_rust(&unspent_coins.coin_states.iter().map(|cs| cs.coin).collect::<Vec<_>>(), fee + 1).unwrap();

        // 5. Create NFT metadata
        let metadata = chia::puzzles::nft::NftMetadata {
            data_uris: vec!["https://example.com/nft.png".to_string()],
            metadata_uris: vec!["https://example.com/metadata.json".to_string()],
            ..Default::default()
        };

        // 6. Mint the NFT
        let nft_spends = async_api::mint_nft_rust(
            &peer,
            synthetic_key,
            selected_coins,
            "did:chia:1s8j4pquxfu5mhlldzu357qfqkwa9r35mdx5a0p0ehn76dr4ut4tqs0n6kv",
            puzzle_hash, // Send NFT to yourself
            metadata,
            None, // No royalty address
            300,  // 3% royalty
            fee,
            None, // Defaults to mainnet
        ).await.unwrap();

        // 7. Sign the transaction
        let signature = sign_coin_spends_rust(
            &nft_spends,
            &[master_secret_key_to_wallet_synthetic_secret_key(&master_secret_key)],
            false, // mainnet
        ).unwrap();

        // 8. Create and broadcast spend bundle
        let spend_bundle = SpendBundle::new(nft_spends, signature);
        let result = async_api::broadcast_spend_bundle_rust(&peer, spend_bundle).await.unwrap();
        
        println!("NFT minting transaction broadcast: {:?}", result);
        */
    }
}
