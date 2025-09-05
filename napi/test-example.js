// Example test script to verify NAPI bindings are working
// Run this after building: npm run build

const driver = require('./index.js');

async function testNAPIBindings() {
  console.log('Testing DataLayer Driver NAPI bindings...\n');

  try {
    // Test key generation
    const secretKey = driver.secretKeyGenerate();
    const publicKey = driver.secretKeyToPublicKey(secretKey);
    console.log('✓ Generated secret key and public key');

    // Test synthetic key functions
    const syntheticKey = driver.masterPublicKeyToWalletSyntheticKey(publicKey);
    const puzzleHash = driver.syntheticKeyToPuzzleHash(syntheticKey);
    console.log('✓ Generated synthetic key and puzzle hash:', puzzleHash);

    // Test delegated puzzles
    const adminPuzzle = driver.adminDelegatedPuzzleFromKey(syntheticKey);
    const writerPuzzle = driver.writerDelegatedPuzzleFromKey(syntheticKey);
    const oraclePuzzle = driver.oracleDelegatedPuzzle(puzzleHash, '1000');
    console.log('✓ Created delegated puzzles');

    // Test coin creation
    const coin = driver.newCoin(puzzleHash, puzzleHash, '100');
    const coinId = driver.getCoinId(coin);
    console.log('✓ Created coin and got coin ID:', coinId);

    // Test coin selection
    const coins = [
      driver.newCoin(puzzleHash, puzzleHash, '100'),
      driver.newCoin(puzzleHash, puzzleHash, '200'),
      driver.newCoin(puzzleHash, puzzleHash, '300'),
    ];
    const selected = driver.selectCoins(coins, '250');
    console.log('✓ Selected', selected.length, 'coins for amount 250');

    // Test morph launcher ID
    const morphed = driver.morphLauncherId(puzzleHash, puzzleHash);
    console.log('✓ Morphed launcher ID:', morphed);

    // Test address encoding
    const address = driver.encodeAddress(puzzleHash, 'xch');
    console.log('✓ Encoded address:', address);

    // Test constants
    const mainnetGenesis = driver.getMainnetGenesisChallenge();
    const testnetGenesis = driver.getTestnet11GenesisChallenge();
    console.log('✓ Got network genesis challenges');

    console.log('\n✅ All NAPI bindings are working correctly!');
  } catch (error) {
    console.error('❌ Error:', error);
  }
}

// Run the test
testNAPIBindings();
