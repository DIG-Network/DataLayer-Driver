// Type definitions for @dignetwork/datalayer-driver-wasm
// Mirrors the offline subset of the NAPI @dignetwork/datalayer-driver interface.

export interface Coin { parentCoinInfo: Uint8Array; puzzleHash: Uint8Array; amount: bigint; }
export interface CoinSpend { coin: Coin; puzzleReveal: Uint8Array; solution: Uint8Array; }
export interface LineageProof { parentParentCoinInfo: Uint8Array; parentInnerPuzzleHash: Uint8Array; parentAmount: bigint; }
export interface EveProof { parentParentCoinInfo: Uint8Array; parentAmount: bigint; }
export interface Proof { lineageProof?: LineageProof; eveProof?: EveProof; }
export interface DataStoreMetadata { rootHash: Uint8Array; label?: string; description?: string; bytes?: bigint; sizeProof?: Uint8Array; }
export interface DelegatedPuzzle { adminInnerPuzzleHash?: Uint8Array; writerInnerPuzzleHash?: Uint8Array; oraclePaymentPuzzleHash?: Uint8Array; oracleFee?: bigint; }
export interface DataStore { coin: Coin; launcherId: Uint8Array; proof: Proof; metadata: DataStoreMetadata; ownerPuzzleHash: Uint8Array; delegatedPuzzles: DelegatedPuzzle[]; }
export interface SuccessResponse { coinSpends: CoinSpend[]; newStore: DataStore; }
export interface ServerCoin { coin: Coin; p2PuzzleHash: Uint8Array; memoUrls: string[]; }
export interface NewServerCoin { serverCoin: ServerCoin; coinSpends: CoinSpend[]; }
export interface Output { puzzleHash: Uint8Array; amount: bigint; memos: Uint8Array[]; }
export interface SpendBundle { coinSpends: CoinSpend[]; aggregatedSignature: Uint8Array; }

/** Initialise the module (installs the panic hook). Call once at startup. */
export function init(): void;

// --- key derivation / addresses ---
export function masterPublicKeyToWalletSyntheticKey(publicKey: Uint8Array): Uint8Array;
export function masterPublicKeyToFirstPuzzleHash(publicKey: Uint8Array): Uint8Array;
export function masterSecretKeyToWalletSyntheticSecretKey(secretKey: Uint8Array): Uint8Array;
export function secretKeyToPublicKey(secretKey: Uint8Array): Uint8Array;
export function syntheticKeyToPuzzleHash(syntheticKey: Uint8Array): Uint8Array;
export function puzzleHashToAddress(puzzleHash: Uint8Array, prefix: string): string;
export function addressToPuzzleHash(address: string): Uint8Array;

// --- delegated puzzles / proofs / ids ---
export function adminDelegatedPuzzleFromKey(syntheticKey: Uint8Array): DelegatedPuzzle;
export function writerDelegatedPuzzleFromKey(syntheticKey: Uint8Array): DelegatedPuzzle;
export function newLineageProof(lineageProof: LineageProof): Proof;
export function newEveProof(eveProof: EveProof): Proof;
export function getCoinId(coin: Coin): Uint8Array;
export function morphLauncherId(launcherId: Uint8Array, offset: bigint): Uint8Array;
export function getMainnetGenesisChallenge(): Uint8Array;
export function getTestnet11GenesisChallenge(): Uint8Array;

// --- DIGStore spend builders ---
export function mintStore(minterSyntheticKey: Uint8Array, selectedCoins: Coin[], rootHash: Uint8Array, label: string | undefined, description: string | undefined, bytes: bigint | undefined, sizeProof: Uint8Array | undefined, ownerPuzzleHash: Uint8Array, delegatedPuzzles: DelegatedPuzzle[], fee: bigint): SuccessResponse;
export function oracleSpend(spenderSyntheticKey: Uint8Array, selectedCoins: Coin[], store: DataStore, fee: bigint): SuccessResponse;
export function meltStore(store: DataStore, ownerPublicKey: Uint8Array): CoinSpend[];
export function updateStoreMetadata(store: DataStore, newRootHash: Uint8Array, newLabel: string | undefined, newDescription: string | undefined, newBytes: bigint | undefined, newSizeProof: Uint8Array | undefined, ownerPublicKey: Uint8Array | undefined, adminPublicKey: Uint8Array | undefined, writerPublicKey: Uint8Array | undefined): SuccessResponse;
export function updateStoreOwnership(store: DataStore, newOwnerPuzzleHash: Uint8Array | undefined, newDelegatedPuzzles: DelegatedPuzzle[], ownerPublicKey: Uint8Array | undefined, adminPublicKey: Uint8Array | undefined): SuccessResponse;

// --- signing / serialization / selection / server coins ---
export function signCoinSpends(coinSpends: CoinSpend[], privateKeys: Uint8Array[], forTestnet: boolean): Uint8Array;
export function signMessage(message: Uint8Array, privateKey: Uint8Array): Uint8Array;
export function verifySignedMessage(signature: Uint8Array, publicKey: Uint8Array, message: Uint8Array): boolean;
export function getCost(coinSpends: CoinSpend[]): bigint;
export function selectCoins(allCoins: Coin[], totalAmount: bigint): Coin[];
export function spendBundleToHex(spendBundle: SpendBundle): string;
export function hexSpendBundleToCoinSpends(hex: string): CoinSpend[];
export function sendXch(syntheticKey: Uint8Array, selectedCoins: Coin[], outputs: Output[], fee: bigint): CoinSpend[];
export function addFee(spenderSyntheticKey: Uint8Array, selectedCoins: Coin[], assertCoinIds: Uint8Array[], fee: bigint): CoinSpend[];
export function createServerCoin(syntheticKey: Uint8Array, selectedCoins: Coin[], hint: Uint8Array, uris: string[], amount: bigint, fee: bigint): NewServerCoin;
