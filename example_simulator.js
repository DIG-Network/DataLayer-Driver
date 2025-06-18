const {
    SimulatorWrapper,
    Peer,
    PeerType,
    Tls,
    getTestnet11GenesisChallenge,
    getMainnetGenesisChallenge,
    masterPublicKeyToWalletSyntheticKey,
    masterPublicKeyToFirstPuzzleHash,
    selectCoins,
    sendXch,
    mintStore,
    getCoinId,
    syntheticKeyToPuzzleHash
} = require('./index.js');

/**
 * Example 1: Basic Simulator Usage
 * Shows how to create a simulator and get a peer connection
 */
async function basicSimulatorExample() {
    console.log('\n=== Basic Simulator Example ===');
    
    try {
        // Create a new blockchain simulator
        console.log('Creating simulator...');
        const simulator = await SimulatorWrapper.new();
        console.log('✓ Simulator created successfully');

        // Get a peer connection from the simulator
        console.log('Getting peer connection...');
        const peer = await simulator.getPeer();
        console.log('✓ Peer connection established');

        // Check the current peak (should be null initially)
        const peak = await peer.getPeak();
        console.log('Current peak:', peak);

        // Get genesis challenge for testnet
        const genesisChallenge = getTestnet11GenesisChallenge();
        console.log('Genesis challenge:', genesisChallenge.toString('hex'));

        return { simulator, peer, genesisChallenge };
    } catch (error) {
        console.error('Error in basic simulator example:', error);
        throw error;
    }
}

/**
 * Example 2: Wallet Operations with Simulator
 * Shows how to work with wallets and coins in the simulator
 */
async function walletOperationsExample(peer, genesisChallenge) {
    console.log('\n=== Wallet Operations Example ===');
    
    try {
        // Create a dummy puzzle hash for demonstration
        const dummyPuzzleHash = Buffer.alloc(32, 0x42); // 32 bytes filled with 0x42
        
        console.log('Using dummy puzzle hash:', dummyPuzzleHash.toString('hex'));

        // Get unspent coins for this wallet (should be empty in fresh simulator)
        console.log('Checking unspent coins...');
        const unspentCoins = await peer.getAllUnspentCoins(dummyPuzzleHash, null, genesisChallenge);
        console.log('Unspent coins:', unspentCoins.coins.length);
        console.log('Last height:', unspentCoins.lastHeight);
        console.log('Last header hash:', unspentCoins.lastHeaderHash.toString('hex'));

        // Look up possible datastore launchers
        console.log('Looking up possible datastore launchers...');
        const launchers = await peer.lookUpPossibleLaunchers(null, genesisChallenge);
        console.log('Found launcher IDs:', launchers.launcherIds.length);

        return { puzzleHash: dummyPuzzleHash, unspentCoins };
    } catch (error) {
        console.error('Error in wallet operations example:', error);
        throw error;
    }
}

/**
 * Example 3: Multiple Peer Connections
 * Shows how to create multiple peer connections from the same simulator
 */
async function multiplePeerExample(simulator) {
    console.log('\n=== Multiple Peer Connections Example ===');
    
    try {
        // Create multiple peer connections from the same simulator
        console.log('Creating multiple peer connections...');
        const peer1 = await simulator.getPeer();
        const peer2 = await simulator.getPeer();
        const peer3 = await simulator.getPeer();
        
        console.log('✓ Created 3 peer connections');

        // They should all report the same peak since they're connected to the same simulator
        const peak1 = await peer1.getPeak();
        const peak2 = await peer2.getPeak();
        const peak3 = await peer3.getPeak();
        
        console.log('Peak from peer 1:', peak1);
        console.log('Peak from peer 2:', peak2);
        console.log('Peak from peer 3:', peak3);
        console.log('All peaks match:', peak1 === peak2 && peak2 === peak3);

        return { peer1, peer2, peer3 };
    } catch (error) {
        console.error('Error in multiple peer example:', error);
        throw error;
    }
}

/**
 * Example 4: Comparison with Original Approach
 * Shows the difference between the new direct approach and the original approach
 */
async function comparisonExample() {
    console.log('\n=== Comparison: New vs Original Approach ===');
    
    try {
        // New approach: Direct simulator creation
        console.log('New approach:');
        const simulator = await SimulatorWrapper.new();
        const peerFromSimulator = await simulator.getPeer();
        console.log('✓ Created peer via SimulatorWrapper.new() -> getPeer()');

        // Original approach: Peer with simulator type (requires TLS even for simulator)
        console.log('\nOriginal approach:');
        try {
            // Note: The original approach requires a TLS object even for simulator
            // For now, we'll just demonstrate that the new approach is preferred
            console.log('✓ New approach is preferred over Peer.new("", PeerType.Simulator, tls)');
            console.log('  (Original approach requires TLS setup even for simulator)');
        } catch (error) {
            console.log('⚠ Original approach has limitations:', error.message);
        }

        // Test the new approach
        const peak1 = await peerFromSimulator.getPeak();
        console.log('Peak from new approach:', peak1);

        return { simulator, peerFromSimulator };
    } catch (error) {
        console.error('Error in comparison example:', error);
        throw error;
    }
}

/**
 * Example 5: Error Handling and Best Practices
 */
async function errorHandlingExample() {
    console.log('\n=== Error Handling Example ===');
    
    try {
        const simulator = await SimulatorWrapper.new();
        const peer = await simulator.getPeer();
        
        // Example of handling errors gracefully
        try {
            // This might fail if the coin doesn't exist
            const nonExistentCoinId = Buffer.alloc(32, 0);
            const isSpent = await peer.isCoinSpent(
                nonExistentCoinId, 
                null, 
                getTestnet11GenesisChallenge()
            );
            console.log('Non-existent coin spent status:', isSpent);
        } catch (error) {
            console.log('Expected error for non-existent coin:', error.message);
        }

        // Example of proper resource cleanup (if needed)
        // Note: The simulator and peer don't have explicit cleanup methods,
        // but in a real application you might want to track them for lifecycle management
        console.log('✓ Simulator operations completed');
        
    } catch (error) {
        console.error('Error in error handling example:', error);
        throw error;
    }
}

/**
 * Example 6: Direct Simulator Manipulation
 * Shows how to use the new simulator methods directly
 */
async function directSimulatorExample() {
    console.log('\n=== Direct Simulator Manipulation Example ===');
    
    try {
        const simulator = await SimulatorWrapper.new();
        
        // Get initial state
        console.log('Initial height:', await simulator.height());
        console.log('Header hash at height 0:', (await simulator.headerHash(0)).toString('hex'));
        
        // Create a new coin
        const puzzleHash = Buffer.alloc(32, 0x33); // 32 bytes filled with 0x33
        const amount = 1000000; // 1 XCH in mojos
        
        console.log('\nCreating new coin...');
        const newCoin = await simulator.newCoin(puzzleHash, BigInt(amount));
        console.log('Created coin:', {
            parentCoinInfo: newCoin.parentCoinInfo.toString('hex'),
            puzzleHash: newCoin.puzzleHash.toString('hex'),
            amount: newCoin.amount.toString()
        });
        
        // Check coin state using the new coin's ID
        const coinId = getCoinId(newCoin);
        console.log('Checking coin state for coin ID:', coinId.toString('hex'));
        const coinState = await simulator.coinState(coinId);
        console.log('Coin state lookup result:', coinState ? 'Found' : 'Not found');
        if (coinState) {
            console.log('  Coin amount:', coinState.coin.amount.toString());
            console.log('  Created height:', coinState.createdHeight?.toString() || 'N/A');
        }
        
        // Get height again to see if it changed
        console.log('\nHeight after coin creation:', await simulator.height());
        
        return { simulator, newCoin };
    } catch (error) {
        console.error('Error in direct simulator example:', error);
        throw error;
    }
}

/**
 * Main function to run all examples
 */
async function runAllExamples() {
    console.log('🚀 SimulatorWrapper Examples');
    console.log('========================');
    
    try {
        // Example 1: Basic usage
        const { simulator, peer, genesisChallenge } = await basicSimulatorExample();
        
        // Example 2: Wallet operations
        await walletOperationsExample(peer, genesisChallenge);
        
        // Example 3: Multiple peers
        await multiplePeerExample(simulator);
        
        // Example 4: Comparison
        await comparisonExample();
        
        // Example 5: Error handling
        await errorHandlingExample();
        
        // Example 6: Direct simulator manipulation
        await directSimulatorExample();
        
        console.log('\n✅ All examples completed successfully!');
        
    } catch (error) {
        console.error('\n❌ Example failed:', error);
        process.exit(1);
    }
}

/**
 * Utility function for testing specific datastore operations
 */
async function datastoreSimulatorExample() {
    console.log('\n=== Datastore Operations with Simulator ===');
    
    try {
        const simulator = await SimulatorWrapper.new();
        const peer = await simulator.getPeer();
        const genesisChallenge = getTestnet11GenesisChallenge();
        
        // Use a dummy puzzle hash for demo purposes
        const puzzleHash = Buffer.alloc(32, 0x24); // 32 bytes filled with 0x24
        
        console.log('Owner puzzle hash:', puzzleHash.toString('hex'));
        
        // In a real scenario, you would:
        // 1. Get some coins to use for minting
        // 2. Create a datastore with mintStore()
        // 3. Sync the datastore
        // 4. Update metadata, etc.
        
        // For now, just show the structure:
        console.log('📝 To mint a datastore, you would:');
        console.log('  1. Get coins: peer.getAllUnspentCoins()');
        console.log('  2. Select coins: selectCoins(allCoins, totalAmount)');
        console.log('  3. Mint store: mintStore(syntheticKey, selectedCoins, rootHash, ...)');
        console.log('  4. Broadcast: peer.broadcastSpend(coinSpends, signatures)');
        console.log('  5. Sync: peer.syncStore() or peer.syncStoreFromLauncherId()');
        
        return { simulator, peer };
    } catch (error) {
        console.error('Error in datastore simulator example:', error);
        throw error;
    }
}

// Export functions for use in other modules
module.exports = {
    basicSimulatorExample,
    walletOperationsExample,
    multiplePeerExample,
    comparisonExample,
    errorHandlingExample,
    directSimulatorExample,
    datastoreSimulatorExample,
    runAllExamples
};

// Run examples if this file is executed directly
if (require.main === module) {
    runAllExamples().catch(console.error);
}