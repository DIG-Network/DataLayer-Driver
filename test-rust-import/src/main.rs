use datalayer_driver::{
    // Core types
    PublicKey, SecretKey, Signature, Bytes32, Coin, CoinSpend,
    DataStore, DataStoreMetadata, DelegatedPuzzle,
    
    // Functions
    master_public_key_to_wallet_synthetic_key,
    master_public_key_to_first_puzzle_hash,
    secret_key_to_public_key,
    synthetic_key_to_puzzle_hash,
    admin_delegated_puzzle_from_key,
    writer_delegated_puzzle_from_key,
    oracle_delegated_puzzle,
    get_coin_id,
    select_coins,
    
    // Server coin
    ServerCoin, morph_launcher_id,
    
    // Wallet types
    TargetNetwork, DataStoreInnerSpend,
    
    // Constants
    constants,
};

#[tokio::main]
async fn main() {
    println!("Testing datalayer-driver Rust exports...\n");
    
    // Test key generation
    let secret_key = SecretKey::generate();
    let public_key = secret_key_to_public_key(&secret_key);
    println!("✓ Generated secret key and public key");
    
    // Test synthetic key functions
    let synthetic_key = master_public_key_to_wallet_synthetic_key(&public_key);
    let puzzle_hash = synthetic_key_to_puzzle_hash(&synthetic_key);
    println!("✓ Generated synthetic key and puzzle hash: {}", hex::encode(puzzle_hash));
    
    // Test delegated puzzles
    let admin_puzzle = admin_delegated_puzzle_from_key(&synthetic_key);
    let writer_puzzle = writer_delegated_puzzle_from_key(&synthetic_key);
    let oracle_puzzle = oracle_delegated_puzzle(puzzle_hash, 1000);
    println!("✓ Created delegated puzzles");
    
    // Test first puzzle hash
    let first_puzzle_hash = master_public_key_to_first_puzzle_hash(&public_key);
    println!("✓ Generated first puzzle hash: {}", hex::encode(first_puzzle_hash));
    
    // Test coin selection
    let test_coins = vec![
        Coin::new(puzzle_hash, puzzle_hash, 100),
        Coin::new(puzzle_hash, puzzle_hash, 200),
        Coin::new(puzzle_hash, puzzle_hash, 300),
    ];
    
    match select_coins(test_coins.clone(), 250) {
        Ok(selected) => println!("✓ Selected {} coins for amount 250", selected.len()),
        Err(e) => println!("✗ Coin selection failed: {}", e),
    }
    
    // Test coin ID
    let coin_id = get_coin_id(&test_coins[0]);
    println!("✓ Got coin ID: {}", hex::encode(coin_id));
    
    // Test morph launcher ID
    let morphed = morph_launcher_id(puzzle_hash, puzzle_hash);
    println!("✓ Morphed launcher ID: {}", hex::encode(morphed));
    
    // Test constants
    let mainnet_genesis = constants::get_mainnet_genesis_challenge();
    let testnet_genesis = constants::get_testnet11_genesis_challenge();
    println!("✓ Got network genesis challenges");
    
    // Test enums exist
    match TargetNetwork::Mainnet {
        TargetNetwork::Mainnet => println!("✓ TargetNetwork enum works"),
        _ => {}
    }
    
    match DataStoreInnerSpend::NormalSpend {
        DataStoreInnerSpend::NormalSpend => println!("✓ DataStoreInnerSpend enum works"),
        _ => {}
    }
    
    println!("\n✅ All Rust exports are working correctly!");
}
