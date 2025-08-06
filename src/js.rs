use napi::bindgen_prelude::*;

#[napi(object)]
#[derive(Clone)]
/// Represents a coin on the Chia blockchain.
///
/// @property {Buffer} parentCoinInfo - Parent coin name/id.
/// @property {Buffer} puzzleHash - Puzzle hash.
/// @property {BigInt} amount - Coin amount.
pub struct Coin {
    pub parent_coin_info: Buffer,
    pub puzzle_hash: Buffer,
    pub amount: BigInt,
}

#[napi(object)]
#[derive(Clone)]
/// Represents a full coin state on the Chia blockchain.
///
/// @property {Coin} coin - The coin.
/// @property {Buffer} spentHeight - The height the coin was spent at, if it was spent.
/// @property {Buffer} createdHeight - The height the coin was created at.
pub struct CoinState {
    pub coin: Coin,
    pub spent_height: Option<BigInt>,
    pub created_height: Option<BigInt>,
}

#[napi(object)]
#[derive(Clone)]
/// Represents a coin spend on the Chia blockchain.
///
/// @property {Coin} coin - The coin being spent.
/// @property {Buffer} puzzleReveal - The puzzle of the coin being spent.
/// @property {Buffer} solution - The solution.
pub struct CoinSpend {
    pub coin: Coin,
    pub puzzle_reveal: Buffer,
    pub solution: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Represents a lineage proof that can be used to spend a singleton.
///
/// @property {Buffer} parentParentCoinInfo - Parent coin's parent coin info/name/ID.
/// @property {Buffer} parentInnerPuzzleHash - Parent coin's inner puzzle hash.
/// @property {BigInt} parentAmount - Parent coin's amount.
pub struct LineageProof {
    pub parent_parent_coin_info: Buffer,
    pub parent_inner_puzzle_hash: Buffer,
    pub parent_amount: BigInt,
}

#[napi(object)]
#[derive(Clone)]
/// Represents an eve proof that can be used to spend a singleton. Parent coin is the singleton launcher.
///
/// @property {Buffer} parentParentCoinInfo - Parent coin's name.
/// @property {BigInt} parentAmount - Parent coin's amount.
pub struct EveProof {
    pub parent_parent_coin_info: Buffer,
    pub parent_amount: BigInt,
}

#[napi(object)]
#[derive(Clone)]
/// Represents a proof (either eve or lineage) that can be used to spend a singleton. Use `new_lineage_proof` or `new_eve_proof` to create a new proof.
///
/// @property {Option<LineageProof>} lineageProof - The lineage proof, if this is a lineage proof.
/// @property {Option<EveProof>} eveProof - The eve proof, if this is an eve proof.
pub struct Proof {
    pub lineage_proof: Option<LineageProof>,
    pub eve_proof: Option<EveProof>,
}

#[napi(object)]
/// Represents a mirror coin with a potentially morphed launcher id.
///
/// @property {Coin} coin - The coin.
/// @property {Buffer} p2PuzzleHash - The puzzle hash that owns the server coin.
/// @property {Array<string>} memoUrls - The memo URLs that serve the data store being mirrored.
pub struct ServerCoin {
    pub coin: Coin,
    pub p2_puzzle_hash: Buffer,
    pub memo_urls: Vec<String>,
}

#[napi(object)]
/// Represents a spend bundle on the Chia blockchain.
///
/// @property {Array<CoinSpend>} coinSpends - The coin spends in this bundle.
/// @property {Buffer} aggregatedSignature - The aggregated signature.
pub struct SpendBundle {
    pub coin_spends: Vec<CoinSpend>,
    pub aggregated_signature: Buffer,
}

#[napi(object)]
/// Object returned by simulator_new_puzzle, containing both the puzzle hash and the puzzle reveal.
pub struct SimulatorPuzzle {
    pub puzzle_hash: Buffer,
    pub puzzle_reveal: Buffer,
}

#[napi(object)]
pub struct BlsPair {
    pub sk: Buffer,
    pub pk: Buffer,
    pub puzzle_hash: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Represents NFT metadata.
///
/// @property {Array<string>} dataUris - List of data URIs.
/// @property {Buffer} dataHash - Hash of the data (optional).
/// @property {Array<string>} metadataUris - List of metadata URIs.
/// @property {Buffer} metadataHash - Hash of the metadata (optional).
/// @property {Array<string>} licenseUris - List of license URIs.
/// @property {Buffer} licenseHash - Hash of the license (optional).
/// @property {BigInt} editionNumber - Edition number (optional).
/// @property {BigInt} editionTotal - Total number of editions (optional).
pub struct NftMetadata {
    pub data_uris: Vec<String>,
    pub data_hash: Option<Buffer>,
    pub metadata_uris: Vec<String>,
    pub metadata_hash: Option<Buffer>,
    pub license_uris: Vec<String>,
    pub license_hash: Option<Buffer>,
    pub edition_number: Option<BigInt>,
    pub edition_total: Option<BigInt>,
}

pub fn err<T>(error: T) -> napi::Error
where
    T: ToString,
{
    napi::Error::from_reason(error.to_string())
}
