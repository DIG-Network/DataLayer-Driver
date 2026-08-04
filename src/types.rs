pub use crate::xch_server_coin::XchServerCoin;
use crate::DataStore;
use chia_bls::{PublicKey, SecretKey};
pub use chia_protocol::*;
pub use chia_puzzle_types::{EveProof, LineageProof, Proof};

pub struct SimulatorPuzzle {
    pub puzzle_hash: Bytes32,
    pub puzzle_reveal: Program,
}

pub struct BlsPair {
    pub sk: SecretKey,
    pub pk: PublicKey,
    pub puzzle_hash: Bytes32,
}

#[derive(Clone, Debug)]
pub struct SuccessResponse {
    pub coin_spends: Vec<CoinSpend>,
    pub new_datastore: DataStore,
}

pub struct UnspentCoinsResponse {
    pub coins: Vec<Coin>,
    pub last_height: u32,
    pub last_header_hash: Bytes32,
}

impl From<UnspentCoinStates> for UnspentCoinsResponse {
    fn from(unspent_coin_states: UnspentCoinStates) -> Self {
        Self {
            coins: unspent_coin_states
                .coin_states
                .into_iter()
                .map(|cs| cs.coin)
                .collect(),
            last_height: unspent_coin_states.last_height,
            last_header_hash: unspent_coin_states.last_header_hash,
        }
    }
}

pub struct UnspentCoinStates {
    pub coin_states: Vec<CoinState>,
    pub last_height: u32,
    pub last_header_hash: Bytes32,
}
pub fn coin_records_to_states(
    coin_records: Vec<chia_wallet_sdk::coinset::CoinRecord>,
) -> Vec<CoinState> {
    coin_records
        .into_iter()
        .map(|coin_record| CoinState {
            coin: coin_record.coin,
            spent_height: Some(coin_record.spent_block_index),
            created_height: Some(coin_record.confirmed_block_index),
        })
        .collect()
}
