#![allow(clippy::result_large_err)]
use indexmap::indexmap;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use chia::bls::{sign, verify, PublicKey, SecretKey, Signature};
use chia::clvm_traits::{clvm_tuple, FromClvm, ToClvm};
use chia::clvm_utils::{tree_hash, ToTreeHash};
use chia::consensus::consensus_constants::ConsensusConstants;
use chia::consensus::flags::{DONT_VALIDATE_SIGNATURE, MEMPOOL_MODE};
use chia::consensus::owned_conditions::OwnedSpendBundleConditions;
use chia::consensus::run_block_generator::run_block_generator;
use chia::consensus::solution_generator::solution_generator;
use chia::consensus::validation_error::ValidationErr;
use chia::protocol::{
    Bytes, Bytes32, Coin, CoinSpend, CoinState, CoinStateFilters, RejectHeaderRequest,
    RequestBlockHeader, RequestFeeEstimates, RespondBlockHeader, RespondFeeEstimates, SpendBundle,
    TransactionAck,
};
use chia::puzzles::{
    nft::NftMetadata,
    standard::{StandardArgs, StandardSolution},
    DeriveSynthetic,
};

use chia_puzzles::SINGLETON_LAUNCHER_HASH;
use chia_wallet_sdk::client::{ClientError, Peer};
use chia_wallet_sdk::driver::{
    get_merkle_tree, Action, Asset, Cat, DataStore, DataStoreMetadata, DelegatedPuzzle, Did,
    DidInfo, DriverError, HashedPtr, Id, IntermediateLauncher, Launcher, Layer, NftMint,
    OracleLayer, P2ParentCoin, P2ParentLayer, Puzzle, Relation, SpendContext, SpendWithConditions,
    Spends, StandardLayer, WriterLayer,
};
// Import proof types from our own crate's rust module
use crate::rust::ServerCoin;
use crate::rust::{EveProof, LineageProof, Proof};
use crate::server_coin::{urls_from_conditions, MirrorArgs, MirrorSolution};
use chia_wallet_sdk::signer::{AggSigConstants, RequiredSignature, SignerError};
use chia_wallet_sdk::types::{
    announcement_id,
    conditions::{CreateCoin, MeltSingleton, Memos, UpdateDataStoreMerkleRoot},
    Condition, Conditions, MAINNET_CONSTANTS, TESTNET11_CONSTANTS,
};
use chia_wallet_sdk::utils::{self, CoinSelectionError};
use clvmr::{Allocator, NodePtr};
use hex_literal::hex;
use thiserror::Error;
use crate::morph_store_launcher_id;
/* echo -n 'datastore' | sha256sum */
pub const DATASTORE_LAUNCHER_HINT: Bytes32 = Bytes32::new(hex!(
    "
    aa7e5b234e1d55967bf0a316395a2eab6cb3370332c0f251f0e44a5afb84fc68
    "
));

pub const DIG_ASSET_ID: Bytes32 = Bytes32::new(hex!(
    "a406d3a9de984d03c9591c10d917593b434d5263cabe2b42f6b367df16832f81"
));

#[derive(Clone, Debug)]
pub struct SuccessResponse {
    pub coin_spends: Vec<CoinSpend>,
    pub new_datastore: DataStore,
}

/// The new server coin and coin spends to create it.
#[derive(Clone, Debug)]
pub struct NewServerCoin {
    pub server_coin: ServerCoin,
    pub coin_spends: Vec<CoinSpend>,
}

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("{0:?}")]
    Client(#[from] ClientError),

    #[error("RejectPuzzleState")]
    RejectPuzzleState,

    #[error("RejectCoinState")]
    RejectCoinState,

    #[error("RejectPuzzleSolution")]
    RejectPuzzleSolution,

    #[error("RejectHeaderRequest")]
    RejectHeaderRequest,

    #[error("{0:?}")]
    Driver(#[from] DriverError),

    #[error("ParseError")]
    Parse,

    #[error("UnknownCoin")]
    UnknownCoin,

    #[error("Clvm error")]
    Clvm,
    #[error("ToClvm error: {0}")]
    ToClvm(#[from] chia::clvm_traits::ToClvmError),

    #[error("Permission error: puzzle can't perform this action")]
    Permission,

    #[error("Io error: {0}")]
    Io(std::io::Error),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationErr),

    #[error("Fee estimation rejection: {0}")]
    FeeEstimateRejection(String),
}

pub struct UnspentCoinStates {
    pub coin_states: Vec<CoinState>,
    pub last_height: u32,
    pub last_header_hash: Bytes32,
}

pub async fn get_unspent_coin_states(
    peer: &Peer,
    puzzle_hash: Bytes32,
    previous_height: Option<u32>,
    previous_header_hash: Bytes32,
    allow_hints: bool,
) -> Result<UnspentCoinStates, WalletError> {
    let mut coin_states = Vec::new();
    let mut last_height = previous_height.unwrap_or_default();

    let mut last_header_hash = previous_header_hash;

    loop {
        let response = peer
            .request_puzzle_state(
                vec![puzzle_hash],
                if last_height == 0 {
                    None
                } else {
                    Some(last_height)
                },
                last_header_hash,
                CoinStateFilters {
                    include_spent: false,
                    include_unspent: true,
                    include_hinted: allow_hints,
                    min_amount: 1,
                },
                false,
            )
            .await
            .map_err(WalletError::Client)?
            .map_err(|_| WalletError::RejectPuzzleState)?;

        last_height = response.height;
        last_header_hash = response.header_hash;
        coin_states.extend(
            response
                .coin_states
                .into_iter()
                .filter(|cs| cs.spent_height.is_none()),
        );

        if response.is_finished {
            break;
        }
    }

    Ok(UnspentCoinStates {
        coin_states,
        last_height,
        last_header_hash,
    })
}

pub fn select_coins(coins: Vec<Coin>, total_amount: u64) -> Result<Vec<Coin>, CoinSelectionError> {
    utils::select_coins(coins.into_iter().collect(), total_amount)
}

fn spend_coins_together(
    ctx: &mut SpendContext,
    synthetic_key: PublicKey,
    coins: &[Coin],
    extra_conditions: Conditions,
    output: i64,
    change_puzzle_hash: Bytes32,
) -> Result<(), WalletError> {
    let p2 = StandardLayer::new(synthetic_key);

    let change = i64::try_from(coins.iter().map(|coin| coin.amount).sum::<u64>()).unwrap() - output;
    assert!(change >= 0);
    let change = change as u64;

    let first_coin_id = coins[0].coin_id();

    for (i, &coin) in coins.iter().enumerate() {
        if i == 0 {
            let mut conditions = extra_conditions.clone();

            if change > 0 {
                conditions = conditions.create_coin(change_puzzle_hash, change, Memos::None);
            }

            p2.spend(ctx, coin, conditions)?;
        } else {
            p2.spend(
                ctx,
                coin,
                Conditions::new().assert_concurrent_spend(first_coin_id),
            )?;
        }
    }
    Ok(())
}

pub fn send_xch(
    synthetic_key: PublicKey,
    coins: &[Coin],
    outputs: &[(Bytes32, u64, Vec<Bytes>)],
    fee: u64,
) -> Result<Vec<CoinSpend>, WalletError> {
    let mut ctx = SpendContext::new();

    let mut conditions = Conditions::new().reserve_fee(fee);
    let mut total_amount = fee;

    for output in outputs {
        let memos = ctx.alloc(&output.2)?;
        conditions = conditions.create_coin(output.0, output.1, Memos::Some(memos));
        total_amount += output.1;
    }

    spend_coins_together(
        &mut ctx,
        synthetic_key,
        coins,
        conditions,
        total_amount.try_into().unwrap(),
        StandardArgs::curry_tree_hash(synthetic_key).into(),
    )?;

    Ok(ctx.take())
}

pub fn create_dig_collateral_coin(
    dig_cats: Vec<Cat>,
    collateral_amount: u64,
    store_id: Bytes32,
    synthetic_key: PublicKey,
    fee_coins: Vec<Coin>,
    fee: u64,
) -> Result<Vec<CoinSpend>, WalletError> {
    let p2_parent_inner_hash = P2ParentCoin::inner_puzzle_hash(Some(DIG_ASSET_ID));

    let mut ctx = SpendContext::new();

    let morphed_store_id = morph_store_launcher_id(store_id);
    let hint = ctx.hint(morphed_store_id)?;

    let actions = &[
        Action::fee(fee),
        Action::send(
            Id::Existing(DIG_ASSET_ID),
            p2_parent_inner_hash.into(),
            collateral_amount,
            hint,
        ),
    ];

    let p2_layer = StandardLayer::new(synthetic_key);
    let p2_puzzle_hash: Bytes32 = p2_layer.tree_hash().into();
    let mut spends = Spends::new(p2_puzzle_hash);

    // add collateral coins to spends
    for cat in dig_cats {
        spends.add(cat);
    }

    // add fee coins to spends
    for fee_xch_coin in fee_coins {
        spends.add(fee_xch_coin);
    }

    let deltas = spends.apply(&mut ctx, actions)?;
    let index_map = indexmap! {p2_puzzle_hash => synthetic_key};

    let _outputs =
        spends.finish_with_keys(&mut ctx, &deltas, Relation::AssertConcurrent, &index_map)?;

    Ok(ctx.take())
}

pub fn create_server_coin(
    synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    hint: Bytes32,
    uris: Vec<String>,
    amount: u64,
    fee: u64,
) -> Result<NewServerCoin, WalletError> {
    let puzzle_hash = StandardArgs::curry_tree_hash(synthetic_key).into();

    let mut memos = Vec::with_capacity(uris.len() + 1);
    memos.push(hint.to_vec());

    for url in &uris {
        memos.push(url.as_bytes().to_vec());
    }

    let mut ctx = SpendContext::new();

    let memos = ctx.alloc(&memos)?;

    let conditions = Conditions::new()
        .create_coin(
            MirrorArgs::curry_tree_hash().into(),
            amount,
            Memos::Some(memos),
        )
        .reserve_fee(fee);

    spend_coins_together(
        &mut ctx,
        synthetic_key,
        &selected_coins,
        conditions,
        (amount + fee).try_into().unwrap(),
        puzzle_hash,
    )?;

    let server_coin = ServerCoin {
        coin: Coin::new(
            selected_coins[0].coin_id(),
            MirrorArgs::curry_tree_hash().into(),
            amount,
        ),
        p2_puzzle_hash: puzzle_hash,
        memo_urls: uris,
    };

    Ok(NewServerCoin {
        coin_spends: ctx.take(),
        server_coin,
    })
}

pub async fn spend_server_coins(
    peer: &Peer,
    synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    total_fee: u64,
    network: TargetNetwork,
) -> Result<Vec<CoinSpend>, WalletError> {
    let puzzle_hash = StandardArgs::curry_tree_hash(synthetic_key).into();

    let mut fee_coins = Vec::new();
    let mut server_coins = Vec::new();

    for coin in selected_coins {
        if coin.puzzle_hash == puzzle_hash {
            fee_coins.push(coin);
        } else {
            server_coins.push(coin);
        }
    }

    if server_coins.is_empty() {
        return Ok(Vec::new());
    }

    assert!(!fee_coins.is_empty());

    let parent_coins = peer
        .request_coin_state(
            server_coins.iter().map(|sc| sc.parent_coin_info).collect(),
            None,
            match network {
                TargetNetwork::Mainnet => MAINNET_CONSTANTS.genesis_challenge,
                TargetNetwork::Testnet11 => TESTNET11_CONSTANTS.genesis_challenge,
            },
            false,
        )
        .await?
        .map_err(|_| WalletError::RejectCoinState)?
        .coin_states;

    let mut ctx = SpendContext::new();

    let puzzle_reveal = ctx.curry(MirrorArgs::default())?;

    let mut conditions = Conditions::new().reserve_fee(total_fee);
    let mut total_fee: i64 = total_fee.try_into().unwrap();

    for server_coin in server_coins {
        let parent_coin = parent_coins
            .iter()
            .find(|cs| cs.coin.coin_id() == server_coin.parent_coin_info)
            .copied()
            .ok_or(WalletError::UnknownCoin)?;

        if parent_coin.coin.puzzle_hash != puzzle_hash {
            return Err(WalletError::Permission);
        }

        let parent_inner_puzzle = ctx.curry(StandardArgs::new(synthetic_key))?;

        let puzzle_reveal = ctx.serialize(&puzzle_reveal)?;

        let solution = ctx.serialize(&MirrorSolution {
            parent_parent_id: parent_coin.coin.parent_coin_info,
            parent_inner_puzzle,
            parent_amount: parent_coin.coin.amount,
            parent_solution: StandardSolution {
                original_public_key: None,
                delegated_puzzle: (),
                solution: (),
            },
        })?;

        total_fee -= i64::try_from(server_coin.amount).unwrap();
        ctx.insert(CoinSpend::new(server_coin, puzzle_reveal, solution));

        conditions = conditions.assert_concurrent_spend(server_coin.coin_id());
    }

    spend_coins_together(
        &mut ctx,
        synthetic_key,
        &fee_coins,
        conditions,
        total_fee,
        puzzle_hash,
    )?;

    Ok(ctx.take())
}

pub async fn fetch_server_coin(
    peer: &Peer,
    coin_state: CoinState,
    max_cost: u64,
) -> Result<ServerCoin, WalletError> {
    let Some(created_height) = coin_state.created_height else {
        return Err(WalletError::UnknownCoin);
    };

    let spend = peer
        .request_puzzle_and_solution(coin_state.coin.parent_coin_info, created_height)
        .await?
        .map_err(|_| WalletError::RejectPuzzleSolution)?;

    let mut allocator = Allocator::new();

    let Ok(output) = spend
        .puzzle
        .run(&mut allocator, 0, max_cost, &spend.solution)
    else {
        return Err(WalletError::Clvm);
    };

    let Ok(conditions) = Vec::<Condition>::from_clvm(&allocator, output.1) else {
        return Err(WalletError::Parse);
    };

    let Some(urls) = urls_from_conditions(&allocator, &coin_state.coin, &conditions) else {
        return Err(WalletError::Parse);
    };

    let puzzle = spend
        .puzzle
        .to_clvm(&mut allocator)
        .map_err(DriverError::ToClvm)?;

    Ok(ServerCoin {
        coin: coin_state.coin,
        p2_puzzle_hash: tree_hash(&allocator, puzzle).into(),
        memo_urls: urls,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn mint_store(
    minter_synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    root_hash: Bytes32,
    label: Option<String>,
    description: Option<String>,
    bytes: Option<u64>,
    size_proof: Option<String>,
    owner_puzzle_hash: Bytes32,
    delegated_puzzles: Vec<DelegatedPuzzle>,
    fee: u64,
) -> Result<SuccessResponse, WalletError> {
    let minter_puzzle_hash: Bytes32 = StandardArgs::curry_tree_hash(minter_synthetic_key).into();
    let total_amount_from_coins = selected_coins.iter().map(|c| c.amount).sum::<u64>();

    let total_amount = fee + 1;

    let mut ctx = SpendContext::new();

    let p2 = StandardLayer::new(minter_synthetic_key);

    let lead_coin = selected_coins[0];
    let lead_coin_name = lead_coin.coin_id();

    for coin in selected_coins.into_iter().skip(1) {
        p2.spend(
            &mut ctx,
            coin,
            Conditions::new().assert_concurrent_spend(lead_coin_name),
        )?;
    }

    let (launch_singleton, datastore) = Launcher::new(lead_coin_name, 1).mint_datastore(
        &mut ctx,
        DataStoreMetadata {
            root_hash,
            label,
            description,
            bytes,
            size_proof,
        },
        owner_puzzle_hash.into(),
        delegated_puzzles,
    )?;

    let launch_singleton = Conditions::new().extend(
        launch_singleton
            .into_iter()
            .map(|cond| {
                if let Condition::CreateCoin(cc) = cond {
                    if cc.puzzle_hash == SINGLETON_LAUNCHER_HASH.into() {
                        let hint = ctx.hint(DATASTORE_LAUNCHER_HINT)?;

                        return Ok(Condition::CreateCoin(CreateCoin {
                            puzzle_hash: cc.puzzle_hash,
                            amount: cc.amount,
                            memos: hint,
                        }));
                    }

                    return Ok(Condition::CreateCoin(cc));
                }

                Ok(cond)
            })
            .collect::<Result<Vec<_>, WalletError>>()?,
    );

    let lead_coin_conditions = if total_amount_from_coins > total_amount {
        let hint = ctx.hint(minter_puzzle_hash)?;

        launch_singleton.create_coin(
            minter_puzzle_hash,
            total_amount_from_coins - total_amount,
            hint,
        )
    } else {
        launch_singleton
    };
    p2.spend(&mut ctx, lead_coin, lead_coin_conditions)?;

    Ok(SuccessResponse {
        coin_spends: ctx.take(),
        new_datastore: datastore,
    })
}

pub struct SyncStoreResponse {
    pub latest_store: DataStore,
    pub latest_height: u32,
    pub root_hash_history: Option<Vec<(Bytes32, u64)>>,
}

pub async fn sync_store(
    peer: &Peer,
    store: &DataStore,
    last_height: Option<u32>,
    last_header_hash: Bytes32,
    with_history: bool,
) -> Result<SyncStoreResponse, WalletError> {
    let mut latest_store = store.clone();
    let mut history = vec![];

    let response = peer
        .request_coin_state(
            vec![store.coin.coin_id()],
            last_height,
            last_header_hash,
            false,
        )
        .await
        .map_err(WalletError::Client)?
        .map_err(|_| WalletError::RejectCoinState)?;
    let mut last_coin_record = response
        .coin_states
        .into_iter()
        .next()
        .ok_or(WalletError::UnknownCoin)?;

    let mut ctx = SpendContext::new(); // just to run puzzles more easily

    while last_coin_record.spent_height.is_some() {
        let puzzle_and_solution_req = peer
            .request_puzzle_and_solution(
                last_coin_record.coin.coin_id(),
                last_coin_record.spent_height.unwrap(),
            )
            .await
            .map_err(WalletError::Client)?
            .map_err(|_| WalletError::RejectPuzzleSolution)?;

        let cs = CoinSpend {
            coin: last_coin_record.coin,
            puzzle_reveal: puzzle_and_solution_req.puzzle,
            solution: puzzle_and_solution_req.solution,
        };

        let new_store = DataStore::<DataStoreMetadata>::from_spend(
            &mut ctx,
            &cs,
            &latest_store.info.delegated_puzzles,
        )
        .map_err(|_| WalletError::Parse)?
        .ok_or(WalletError::Parse)?;

        if with_history {
            let resp: Result<RespondBlockHeader, RejectHeaderRequest> = peer
                .request_fallible(RequestBlockHeader {
                    height: last_coin_record.spent_height.unwrap(),
                })
                .await
                .map_err(WalletError::Client)?;
            let block_header = resp.map_err(|_| WalletError::RejectHeaderRequest)?;

            history.push((
                new_store.info.metadata.root_hash,
                block_header
                    .header_block
                    .foliage_transaction_block
                    .unwrap()
                    .timestamp,
            ));
        }

        let response = peer
            .request_coin_state(
                vec![new_store.coin.coin_id()],
                last_height,
                last_header_hash,
                false,
            )
            .await
            .map_err(WalletError::Client)?
            .map_err(|_| WalletError::RejectCoinState)?;

        last_coin_record = response
            .coin_states
            .into_iter()
            .next()
            .ok_or(WalletError::UnknownCoin)?;
        latest_store = new_store;
    }

    Ok(SyncStoreResponse {
        latest_store,
        latest_height: last_coin_record
            .created_height
            .ok_or(WalletError::UnknownCoin)?,
        root_hash_history: if with_history { Some(history) } else { None },
    })
}

pub async fn sync_store_using_launcher_id(
    peer: &Peer,
    launcher_id: Bytes32,
    last_height: Option<u32>,
    last_header_hash: Bytes32,
    with_history: bool,
) -> Result<SyncStoreResponse, WalletError> {
    let response = peer
        .request_coin_state(vec![launcher_id], last_height, last_header_hash, false)
        .await
        .map_err(WalletError::Client)?
        .map_err(|_| WalletError::RejectCoinState)?;
    let last_coin_record = response
        .coin_states
        .into_iter()
        .next()
        .ok_or(WalletError::UnknownCoin)?;

    let mut ctx = SpendContext::new(); // just to run puzzles more easily

    let puzzle_and_solution_req = peer
        .request_puzzle_and_solution(
            last_coin_record.coin.coin_id(),
            last_coin_record
                .spent_height
                .ok_or(WalletError::UnknownCoin)?,
        )
        .await
        .map_err(WalletError::Client)?
        .map_err(|_| WalletError::RejectPuzzleSolution)?;

    let cs = CoinSpend {
        coin: last_coin_record.coin,
        puzzle_reveal: puzzle_and_solution_req.puzzle,
        solution: puzzle_and_solution_req.solution,
    };

    let first_store = DataStore::<DataStoreMetadata>::from_spend(&mut ctx, &cs, &[])
        .map_err(|_| WalletError::Parse)?
        .ok_or(WalletError::Parse)?;

    let res = sync_store(
        peer,
        &first_store,
        last_height,
        last_header_hash,
        with_history,
    )
    .await?;

    // prepend root hash from launch
    let root_hash_history = if let Some(mut res_root_hash_history) = res.root_hash_history {
        let spent_timestamp = if let Some(spent_height) = last_coin_record.spent_height {
            let resp: Result<RespondBlockHeader, RejectHeaderRequest> = peer
                .request_fallible(RequestBlockHeader {
                    height: spent_height,
                })
                .await
                .map_err(WalletError::Client)?;
            let resp = resp.map_err(|_| WalletError::RejectHeaderRequest)?;

            resp.header_block
                .foliage_transaction_block
                .unwrap()
                .timestamp
        } else {
            0
        };

        res_root_hash_history.insert(0, (first_store.info.metadata.root_hash, spent_timestamp));
        Some(res_root_hash_history)
    } else {
        None
    };

    Ok(SyncStoreResponse {
        latest_store: res.latest_store,
        latest_height: res.latest_height,
        root_hash_history,
    })
}

pub async fn get_store_creation_height(
    peer: &Peer,
    launcher_id: Bytes32,
    last_height: Option<u32>,
    last_header_hash: Bytes32,
) -> Result<u32, WalletError> {
    let response = peer
        .request_coin_state(vec![launcher_id], last_height, last_header_hash, false)
        .await
        .map_err(WalletError::Client)?
        .map_err(|_| WalletError::RejectCoinState)?;
    let last_coin_record = response
        .coin_states
        .into_iter()
        .next()
        .ok_or(WalletError::UnknownCoin)?;

    last_coin_record
        .created_height
        .ok_or(WalletError::UnknownCoin)
}

#[derive(Clone, Debug)]
pub enum DataStoreInnerSpend {
    Owner(PublicKey),
    Admin(PublicKey),
    Writer(PublicKey),
    // does not include oracle since it can't change metadata/owners :(
}

fn update_store_with_conditions(
    ctx: &mut SpendContext,
    conditions: Conditions,
    datastore: DataStore,
    inner_spend_info: DataStoreInnerSpend,
    allow_admin: bool,
    allow_writer: bool,
) -> Result<SuccessResponse, WalletError> {
    let inner_datastore_spend = match inner_spend_info {
        DataStoreInnerSpend::Owner(pk) => {
            StandardLayer::new(pk).spend_with_conditions(ctx, conditions)?
        }
        DataStoreInnerSpend::Admin(pk) => {
            if !allow_admin {
                return Err(WalletError::Permission);
            }

            StandardLayer::new(pk).spend_with_conditions(ctx, conditions)?
        }
        DataStoreInnerSpend::Writer(pk) => {
            if !allow_writer {
                return Err(WalletError::Permission);
            }

            WriterLayer::new(StandardLayer::new(pk)).spend(ctx, conditions)?
        }
    };

    let parent_delegated_puzzles = datastore.info.delegated_puzzles.clone();
    let new_spend = datastore.spend(ctx, inner_datastore_spend)?;

    let new_datastore =
        DataStore::<DataStoreMetadata>::from_spend(ctx, &new_spend, &parent_delegated_puzzles)?
            .ok_or(WalletError::Parse)?;

    Ok(SuccessResponse {
        coin_spends: vec![new_spend],
        new_datastore,
    })
}

pub fn update_store_ownership(
    datastore: DataStore,
    new_owner_puzzle_hash: Bytes32,
    new_delegated_puzzles: Vec<DelegatedPuzzle>,
    inner_spend_info: DataStoreInnerSpend,
) -> Result<SuccessResponse, WalletError> {
    let ctx = &mut SpendContext::new();

    let update_condition: Condition = match inner_spend_info {
        DataStoreInnerSpend::Owner(_) => {
            DataStore::<DataStoreMetadata>::owner_create_coin_condition(
                ctx,
                datastore.info.launcher_id,
                new_owner_puzzle_hash,
                new_delegated_puzzles,
                true,
            )?
        }
        DataStoreInnerSpend::Admin(_) => {
            let merkle_tree = get_merkle_tree(ctx, new_delegated_puzzles.clone())?;

            let new_merkle_root_condition = UpdateDataStoreMerkleRoot {
                new_merkle_root: merkle_tree.root(),
                memos: DataStore::<DataStoreMetadata>::get_recreation_memos(
                    datastore.info.launcher_id,
                    new_owner_puzzle_hash.into(),
                    new_delegated_puzzles,
                ),
            }
            .to_clvm(&mut **ctx)
            .map_err(DriverError::ToClvm)?;

            Condition::Other(new_merkle_root_condition)
        }
        _ => return Err(WalletError::Permission),
    };

    let update_conditions = Conditions::new().with(update_condition);

    update_store_with_conditions(
        ctx,
        update_conditions,
        datastore,
        inner_spend_info,
        true,
        false,
    )
}

pub fn update_store_metadata(
    datastore: DataStore,
    new_root_hash: Bytes32,
    new_label: Option<String>,
    new_description: Option<String>,
    new_bytes: Option<u64>,
    new_size_proof: Option<String>,
    inner_spend_info: DataStoreInnerSpend,
) -> Result<SuccessResponse, WalletError> {
    let ctx = &mut SpendContext::new();

    let new_metadata = DataStoreMetadata {
        root_hash: new_root_hash,
        label: new_label,
        description: new_description,
        bytes: new_bytes,
        size_proof: new_size_proof,
    };
    let mut new_metadata_condition = Conditions::new().with(
        DataStore::<DataStoreMetadata>::new_metadata_condition(ctx, new_metadata)?,
    );

    if let DataStoreInnerSpend::Owner(_) = inner_spend_info {
        new_metadata_condition = new_metadata_condition.with(
            DataStore::<DataStoreMetadata>::owner_create_coin_condition(
                ctx,
                datastore.info.launcher_id,
                datastore.info.owner_puzzle_hash,
                datastore.info.delegated_puzzles.clone(),
                false,
            )?,
        );
    }

    update_store_with_conditions(
        ctx,
        new_metadata_condition,
        datastore,
        inner_spend_info,
        true,
        true,
    )
}

pub fn melt_store(
    datastore: DataStore,
    owner_pk: PublicKey,
) -> Result<Vec<CoinSpend>, WalletError> {
    let ctx = &mut SpendContext::new();

    let melt_conditions = Conditions::new()
        .with(Condition::reserve_fee(1))
        .with(Condition::Other(
            MeltSingleton {}
                .to_clvm(&mut **ctx)
                .map_err(DriverError::ToClvm)?,
        ));

    let inner_datastore_spend =
        StandardLayer::new(owner_pk).spend_with_conditions(ctx, melt_conditions)?;

    let new_spend = datastore.spend(ctx, inner_datastore_spend)?;

    Ok(vec![new_spend])
}

pub fn oracle_spend(
    spender_synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    datastore: DataStore,
    fee: u64,
) -> Result<SuccessResponse, WalletError> {
    let Some(DelegatedPuzzle::Oracle(oracle_ph, oracle_fee)) = datastore
        .info
        .delegated_puzzles
        .iter()
        .find(|dp| matches!(dp, DelegatedPuzzle::Oracle(_, _)))
    else {
        return Err(WalletError::Permission);
    };

    let spender_puzzle_hash: Bytes32 = StandardArgs::curry_tree_hash(spender_synthetic_key).into();

    let total_amount = oracle_fee + fee;

    let ctx = &mut SpendContext::new();

    let p2 = StandardLayer::new(spender_synthetic_key);

    let lead_coin = selected_coins[0];
    let lead_coin_name = lead_coin.coin_id();

    let total_amount_from_coins = selected_coins.iter().map(|c| c.amount).sum::<u64>();
    for coin in selected_coins.into_iter().skip(1) {
        p2.spend(
            ctx,
            coin,
            Conditions::new().assert_concurrent_spend(lead_coin_name),
        )?;
    }

    let assert_oracle_conds = Conditions::new().assert_puzzle_announcement(announcement_id(
        datastore.coin.puzzle_hash,
        Bytes::new("$".into()),
    ));

    let mut lead_coin_conditions = assert_oracle_conds;
    if total_amount_from_coins > total_amount {
        let hint = ctx.hint(spender_puzzle_hash)?;

        lead_coin_conditions = lead_coin_conditions.create_coin(
            spender_puzzle_hash,
            total_amount_from_coins - total_amount,
            hint,
        );
    }
    if fee > 0 {
        lead_coin_conditions = lead_coin_conditions.reserve_fee(fee);
    }
    p2.spend(ctx, lead_coin, lead_coin_conditions)?;

    let inner_datastore_spend = OracleLayer::new(*oracle_ph, *oracle_fee)
        .ok_or(DriverError::OddOracleFee)?
        .construct_spend(ctx, ())?;

    let parent_delegated_puzzles = datastore.info.delegated_puzzles.clone();
    let new_spend = datastore.spend(ctx, inner_datastore_spend)?;

    let new_datastore = DataStore::from_spend(ctx, &new_spend, &parent_delegated_puzzles)?
        .ok_or(WalletError::Parse)?;
    ctx.insert(new_spend.clone());

    Ok(SuccessResponse {
        coin_spends: ctx.take(),
        new_datastore,
    })
}

pub fn add_fee(
    spender_synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    coin_ids: Vec<Bytes32>,
    fee: u64,
) -> Result<Vec<CoinSpend>, WalletError> {
    let spender_puzzle_hash: Bytes32 = StandardArgs::curry_tree_hash(spender_synthetic_key).into();
    let total_amount_from_coins = selected_coins.iter().map(|c| c.amount).sum::<u64>();

    let mut ctx = SpendContext::new();

    let p2 = StandardLayer::new(spender_synthetic_key);

    let lead_coin = selected_coins[0];
    let lead_coin_name = lead_coin.coin_id();

    for coin in selected_coins.into_iter().skip(1) {
        p2.spend(
            &mut ctx,
            coin,
            Conditions::new().assert_concurrent_spend(lead_coin_name),
        )?;
    }

    let mut lead_coin_conditions = Conditions::new().reserve_fee(fee);
    if total_amount_from_coins > fee {
        let hint = ctx.hint(spender_puzzle_hash)?;

        lead_coin_conditions = lead_coin_conditions.create_coin(
            spender_puzzle_hash,
            total_amount_from_coins - fee,
            hint,
        );
    }
    for coin_id in coin_ids {
        lead_coin_conditions = lead_coin_conditions.assert_concurrent_spend(coin_id);
    }

    p2.spend(&mut ctx, lead_coin, lead_coin_conditions)?;

    Ok(ctx.take())
}

pub fn public_key_to_synthetic_key(pk: PublicKey) -> PublicKey {
    pk.derive_synthetic()
}

pub fn secret_key_to_synthetic_key(sk: SecretKey) -> SecretKey {
    sk.derive_synthetic()
}

#[derive(Debug, Clone, Copy)]
pub enum TargetNetwork {
    Mainnet,
    Testnet11,
}

impl TargetNetwork {
    fn get_constants(&self) -> &ConsensusConstants {
        match self {
            TargetNetwork::Mainnet => &MAINNET_CONSTANTS,
            TargetNetwork::Testnet11 => &TESTNET11_CONSTANTS,
        }
    }
}

pub fn sign_coin_spends(
    coin_spends: Vec<CoinSpend>,
    private_keys: Vec<SecretKey>,
    network: TargetNetwork,
) -> Result<Signature, SignerError> {
    let mut allocator = Allocator::new();

    let required_signatures = RequiredSignature::from_coin_spends(
        &mut allocator,
        &coin_spends,
        &AggSigConstants::new(network.get_constants().agg_sig_me_additional_data),
    )?;

    let key_pairs = private_keys
        .iter()
        .map(|sk| {
            (
                sk.public_key(),
                sk.clone(),
                sk.public_key().derive_synthetic(),
                sk.derive_synthetic(),
            )
        })
        .flat_map(|(pk1, sk1, pk2, sk2)| vec![(pk1, sk1), (pk2, sk2)])
        .collect::<HashMap<PublicKey, SecretKey>>();

    let mut sig = Signature::default();

    for required in required_signatures {
        let RequiredSignature::Bls(required) = required else {
            continue;
        };

        let sk = key_pairs.get(&required.public_key);

        if let Some(sk) = sk {
            sig += &sign(sk, required.message());
        }
    }

    Ok(sig)
}

pub async fn broadcast_spend_bundle(
    peer: &Peer,
    spend_bundle: SpendBundle,
) -> Result<TransactionAck, WalletError> {
    peer.send_transaction(spend_bundle)
        .await
        .map_err(WalletError::Client)
}

pub async fn get_header_hash(peer: &Peer, height: u32) -> Result<Bytes32, WalletError> {
    let resp: Result<RespondBlockHeader, RejectHeaderRequest> = peer
        .request_fallible(RequestBlockHeader { height })
        .await
        .map_err(WalletError::Client)?;

    resp.map_err(|_| WalletError::RejectHeaderRequest)
        .map(|resp| resp.header_block.header_hash())
}

pub async fn get_fee_estimate(peer: &Peer, target_time_seconds: u64) -> Result<u64, WalletError> {
    let target_time_seconds = target_time_seconds
        + SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

    let resp: RespondFeeEstimates = peer
        .request_infallible(RequestFeeEstimates {
            time_targets: vec![target_time_seconds],
        })
        .await
        .map_err(WalletError::Client)?;
    let fee_estimate_group = resp.estimates;

    if let Some(error_message) = fee_estimate_group.error {
        return Err(WalletError::FeeEstimateRejection(error_message));
    }

    if let Some(first_estimate) = fee_estimate_group.estimates.first() {
        if let Some(error_message) = &first_estimate.error {
            return Err(WalletError::FeeEstimateRejection(error_message.clone()));
        }

        return Ok(first_estimate.estimated_fee_rate.mojos_per_clvm_cost);
    }

    Err(WalletError::FeeEstimateRejection(
        "No fee estimates available".to_string(),
    ))
}

pub async fn is_coin_spent(
    peer: &Peer,
    coin_id: Bytes32,
    last_height: Option<u32>,
    last_header_hash: Bytes32,
) -> Result<bool, WalletError> {
    let response = peer
        .request_coin_state(vec![coin_id], last_height, last_header_hash, false)
        .await
        .map_err(WalletError::Client)?
        .map_err(|_| WalletError::RejectCoinState)?;

    if let Some(coin_state) = response.coin_states.first() {
        return Ok(coin_state.spent_height.is_some());
    }

    Ok(false)
}

// https://github.com/Chia-Network/chips/blob/main/CHIPs/chip-0002.md#signmessage
pub fn make_message(msg: Bytes) -> Result<Bytes32, WalletError> {
    let mut alloc = Allocator::new();
    let thing_ptr = clvm_tuple!("Chia Signed Message", msg)
        .to_clvm(&mut alloc)
        .map_err(DriverError::ToClvm)?;

    Ok(tree_hash(&alloc, thing_ptr).into())
}

pub fn sign_message(message: Bytes, sk: SecretKey) -> Result<Signature, WalletError> {
    Ok(sign(&sk, make_message(message)?))
}

pub fn verify_signature(
    message: Bytes,
    pk: PublicKey,
    sig: Signature,
) -> Result<bool, WalletError> {
    Ok(verify(&sig, &pk, make_message(message)?))
}

pub fn get_cost(coin_spends: Vec<CoinSpend>) -> Result<u64, WalletError> {
    let mut alloc = Allocator::new();

    let generator = solution_generator(
        coin_spends
            .into_iter()
            .map(|cs| (cs.coin, cs.puzzle_reveal, cs.solution)),
    )
    .map_err(WalletError::Io)?;

    let conds = run_block_generator::<&[u8], _>(
        &mut alloc,
        &generator,
        [],
        u64::MAX,
        MEMPOOL_MODE | DONT_VALIDATE_SIGNATURE,
        &Signature::default(),
        None,
        TargetNetwork::Mainnet.get_constants(),
    )?;

    let conds = OwnedSpendBundleConditions::from(&alloc, conds);

    Ok(conds.cost)
}

pub struct PossibleLaunchersResponse {
    pub launcher_ids: Vec<Bytes32>,
    pub last_height: u32,
    pub last_header_hash: Bytes32,
}

pub async fn look_up_possible_launchers(
    peer: &Peer,
    previous_height: Option<u32>,
    previous_header_hash: Bytes32,
) -> Result<PossibleLaunchersResponse, WalletError> {
    let resp = get_unspent_coin_states(
        peer,
        DATASTORE_LAUNCHER_HINT,
        previous_height,
        previous_header_hash,
        true,
    )
    .await?;

    Ok(PossibleLaunchersResponse {
        last_header_hash: resp.last_header_hash,
        last_height: resp.last_height,
        launcher_ids: resp
            .coin_states
            .into_iter()
            .filter_map(|coin_state| {
                if coin_state.coin.puzzle_hash == SINGLETON_LAUNCHER_HASH.into() {
                    Some(coin_state.coin.coin_id())
                } else {
                    None
                }
            })
            .collect(),
    })
}

pub async fn prove_dig_cat_coin(
    peer: &Peer,
    coin: &Coin,
    coin_created_height: u32,
) -> Result<Cat, WalletError> {
    let mut ctx = SpendContext::new();

    // 1) Request parent coin state
    let parent_state_response = peer
        .request_coin_state(
            vec![coin.parent_coin_info],
            None,
            MAINNET_CONSTANTS.genesis_challenge,
            false,
        )
        .await?;

    let parent_state = parent_state_response.map_err(|_| WalletError::RejectCoinState)?;

    // 2) Request parent puzzle and solution
    let parent_puzzle_and_solution_response = peer
        .request_puzzle_and_solution(parent_state.coin_ids[0], coin_created_height)
        .await?;

    let parent_puzzle_and_solution =
        parent_puzzle_and_solution_response.map_err(|_| WalletError::RejectPuzzleSolution)?;

    // 3) Convert puzzle to CLVM
    let parent_puzzle_ptr = ctx.alloc(&parent_puzzle_and_solution.puzzle)?;
    let parent_puzzle = Puzzle::parse(&mut ctx, parent_puzzle_ptr);

    // 4) Convert solution to CLVM
    let parent_solution = ctx.alloc(&parent_puzzle_and_solution.solution)?;

    // 5) Parse CAT
    let parsed_children = Cat::parse_children(
        &mut ctx,
        parent_state.coin_states[0].coin,
        parent_puzzle,
        parent_solution,
    )?
    .ok_or(WalletError::UnknownCoin)?;

    let proved_cat = parsed_children
        .into_iter()
        .find(|parsed_child| {
            parsed_child.coin_id() == coin.coin_id()
                && parsed_child.lineage_proof.is_some()
                && parsed_child.info.asset_id == DIG_ASSET_ID
        })
        .ok_or_else(|| WalletError::UnknownCoin)?;
    Ok(proved_cat)
}

pub async fn subscribe_to_coin_states(
    peer: &Peer,
    coin_id: Bytes32,
    previous_height: Option<u32>,
    previous_header_hash: Bytes32,
) -> Result<Option<u32>, WalletError> {
    let response = peer
        .request_coin_state(vec![coin_id], previous_height, previous_header_hash, true)
        .await
        .map_err(WalletError::Client)?
        .map_err(|_| WalletError::RejectCoinState)?;

    if let Some(coin_state) = response.coin_states.first() {
        return Ok(coin_state.spent_height);
    }

    Err(WalletError::UnknownCoin)
}

pub async fn unsubscribe_from_coin_states(
    peer: &Peer,
    coin_id: Bytes32,
) -> Result<(), WalletError> {
    peer.remove_coin_subscriptions(Some(vec![coin_id]))
        .await
        .map_err(WalletError::Client)?;

    Ok(())
}

/// Mints a new NFT using a DID string.
///
/// # Arguments
/// * `peer` - The peer to query blockchain data
/// * `synthetic_key` - The synthetic key of the wallet
/// * `selected_coins` - Coins to spend for the transaction
/// * `did_string` - The DID string (e.g., "did:chia:1s8j4pquxfu5mhlldzu357qfqkwa9r35mdx5a0p0ehn76dr4ut4tqs0n6kv")
/// * `recipient_puzzle_hash` - The puzzle hash to send the NFT to
/// * `metadata` - The NFT metadata
/// * `royalty_puzzle_hash` - Optional royalty puzzle hash (defaults to recipient if None)
/// * `royalty_basis_points` - Royalty percentage in basis points (e.g., 300 = 3%)
/// * `fee` - Transaction fee
/// * `network` - The target network (mainnet/testnet)
///
/// # Returns
/// A vector of coin spends that mint the NFT
#[allow(clippy::too_many_arguments)]
pub async fn mint_nft(
    peer: &Peer,
    synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    did_string: &str,
    recipient_puzzle_hash: Bytes32,
    metadata: NftMetadata,
    _royalty_puzzle_hash: Option<Bytes32>,
    royalty_basis_points: u16,
    fee: u64,
    network: TargetNetwork,
) -> Result<Vec<CoinSpend>, WalletError> {
    // Resolve the DID string to get the current coin and proof
    let (did_proof, did_coin) =
        resolve_did_string_and_generate_proof(peer, did_string, network).await?;
    let mut ctx = SpendContext::new();

    // Convert DID proof
    let did_proof = match did_proof {
        chia::puzzles::Proof::Eve(eve) => Proof::Eve(EveProof {
            parent_parent_coin_info: eve.parent_parent_coin_info,
            parent_amount: eve.parent_amount,
        }),
        chia::puzzles::Proof::Lineage(lineage) => Proof::Lineage(LineageProof {
            parent_parent_coin_info: lineage.parent_parent_coin_info,
            parent_inner_puzzle_hash: lineage.parent_inner_puzzle_hash,
            parent_amount: lineage.parent_amount,
        }),
    };

    // Create the DID singleton info (simplified DID structure)
    // Use the first 32 bytes of the public key (truncate from 48 to 32 bytes)
    let public_key_bytes = synthetic_key.derive_synthetic().to_bytes();
    let mut public_key_hash = [0u8; 32];
    public_key_hash.copy_from_slice(&public_key_bytes[..32]);
    let mut meta_data_allocator = Allocator::new();
    let node_metadata = metadata.to_clvm(&mut meta_data_allocator)?;
    let metadata_hashed_ptr = HashedPtr::from_ptr(&meta_data_allocator, node_metadata);
    let did_info: DidInfo = DidInfo::new(
        did_coin.coin_id(),
        None,
        1,
        metadata_hashed_ptr,
        public_key_hash.into(),
    );

    let did = Did::new(did_coin, did_proof, did_info);

    // Create StandardLayer for spending coins
    let p2 = StandardLayer::new(synthetic_key);

    // Create the NFT mint configuration with metadata
    let nft_mint = NftMint::new(
        metadata_hashed_ptr,
        recipient_puzzle_hash,
        royalty_basis_points,
        None, // No DID owner for now - we'll set this up differently
    );

    // Use IntermediateLauncher to mint the NFT
    let (mint_conditions, _nft) = IntermediateLauncher::new(did_coin.coin_id(), 0, 1)
        .create(&mut ctx)?
        .mint_nft(&mut ctx, &nft_mint)?;

    // Update the DID with the mint conditions
    let _updated_did = did.update(&mut ctx, &p2, mint_conditions)?;

    // Handle fee and change
    let total_input = selected_coins.iter().map(|coin| coin.amount).sum::<u64>();
    let total_needed = fee + 1; // 1 mojo for the NFT

    if total_input < total_needed {
        return Err(WalletError::Parse); // Not enough coins
    }

    let _change = total_input - total_needed;
    let change_puzzle_hash = StandardArgs::curry_tree_hash(synthetic_key).into();

    // Spend the selected coins
    spend_coins_together(
        &mut ctx,
        synthetic_key,
        &selected_coins,
        Conditions::new().reserve_fee(fee),
        total_needed as i64,
        change_puzzle_hash,
    )?;

    Ok(ctx.take())
}

/// Generates a DID proof for a DID coin by analyzing its parent.
/// This is a simplified version that automatically determines the proof type.
///
/// # Arguments
/// * `peer` - The peer to query blockchain data
/// * `did_coin` - The DID coin to generate proof for
/// * `network` - The target network (mainnet/testnet)
///
/// # Returns
/// A tuple containing the DID proof and the DID coin
pub async fn generate_did_proof(
    peer: &Peer,
    did_coin: Coin,
    network: TargetNetwork,
) -> Result<(chia::puzzles::Proof, Coin), WalletError> {
    let proof = generate_did_proof_from_chain(peer, did_coin, network).await?;
    Ok((proof, did_coin))
}

/// Generates a DID proof manually when you have the parent information.
///
/// # Arguments
/// * `did_coin` - The current DID coin
/// * `parent_coin` - The parent coin of the DID (None for eve proof)
/// * `parent_inner_puzzle_hash` - The parent's inner puzzle hash (for lineage proof)
///
/// # Returns
/// A DID proof that can be used to spend the DID coin
pub fn generate_did_proof_manual(
    did_coin: Coin,
    parent_coin: Option<Coin>,
    parent_inner_puzzle_hash: Option<Bytes32>,
) -> Result<chia::puzzles::Proof, WalletError> {
    match parent_coin {
        // Eve proof - first spend from launcher
        None => {
            // For eve proof, we need the launcher coin info
            // The parent_parent_coin_info is the coin that created the launcher
            // The parent_amount is the launcher coin amount (typically 1 mojo)
            Ok(chia::puzzles::Proof::Eve(chia::puzzles::EveProof {
                parent_parent_coin_info: did_coin.parent_coin_info,
                parent_amount: 1, // Launcher coins are typically 1 mojo
            }))
        }
        // Lineage proof - subsequent spends
        Some(parent) => {
            let parent_inner_puzzle_hash = parent_inner_puzzle_hash.ok_or(WalletError::Parse)?; // Need inner puzzle hash for lineage proof

            Ok(chia::puzzles::Proof::Lineage(chia::puzzles::LineageProof {
                parent_parent_coin_info: parent.parent_coin_info,
                parent_inner_puzzle_hash,
                parent_amount: parent.amount,
            }))
        }
    }
}

/// Generates a DID proof from a coin spend by analyzing the parent spend.
///
/// # Arguments
/// * `peer` - The peer to query blockchain data
/// * `did_coin` - The DID coin to generate proof for
/// * `network` - The target network (mainnet/testnet)
///
/// # Returns
/// A DID proof that can be used to spend the DID coin
pub async fn generate_did_proof_from_chain(
    peer: &Peer,
    did_coin: Coin,
    network: TargetNetwork,
) -> Result<chia::puzzles::Proof, WalletError> {
    // Get the parent coin state
    let parent_coin_states = peer
        .request_coin_state(
            vec![did_coin.parent_coin_info],
            None,
            match network {
                TargetNetwork::Mainnet => MAINNET_CONSTANTS.genesis_challenge,
                TargetNetwork::Testnet11 => TESTNET11_CONSTANTS.genesis_challenge,
            },
            false,
        )
        .await?
        .map_err(|_| WalletError::RejectCoinState)?
        .coin_states;

    let parent_coin_state = parent_coin_states.first().ok_or(WalletError::UnknownCoin)?;

    // Check if parent is a launcher (puzzle hash matches singleton launcher)
    if parent_coin_state.coin.puzzle_hash == SINGLETON_LAUNCHER_HASH.into() {
        // This is an eve proof - first spend from launcher
        return Ok(chia::puzzles::Proof::Eve(chia::puzzles::EveProof {
            parent_parent_coin_info: parent_coin_state.coin.parent_coin_info,
            parent_amount: parent_coin_state.coin.amount,
        }));
    }

    // This is a lineage proof - need to get the parent's puzzle and solution
    let parent_spend_height = parent_coin_state
        .spent_height
        .ok_or(WalletError::UnknownCoin)?;

    let _parent_spend = peer
        .request_puzzle_and_solution(parent_coin_state.coin.coin_id(), parent_spend_height)
        .await?
        .map_err(|_| WalletError::RejectPuzzleSolution)?;

    let _allocator = Allocator::new();

    // For now, create a basic lineage proof
    // This is a simplified approach - in production you'd want to properly parse the parent DID
    Ok(chia::puzzles::Proof::Lineage(chia::puzzles::LineageProof {
        parent_parent_coin_info: parent_coin_state.coin.parent_coin_info,
        parent_inner_puzzle_hash: Bytes32::default(), // Would need to parse from parent spend
        parent_amount: parent_coin_state.coin.amount,
    }))
}

/// Creates a simple DID from a private key and selected coins.
///
/// # Arguments
/// * `synthetic_key` - The synthetic key that will control the DID
/// * `selected_coins` - Coins to spend for creating the DID
/// * `fee` - Transaction fee
///
/// # Returns
/// A tuple containing the coin spends and the created DID coin
pub fn create_simple_did(
    synthetic_key: PublicKey,
    selected_coins: Vec<Coin>,
    fee: u64,
) -> Result<(Vec<CoinSpend>, Coin), WalletError> {
    let mut ctx = SpendContext::new();

    let p2 = StandardLayer::new(synthetic_key);
    let puzzle_hash = StandardArgs::curry_tree_hash(synthetic_key).into();

    // Calculate total input and needed amount
    let total_input = selected_coins.iter().map(|coin| coin.amount).sum::<u64>();
    let total_needed = fee + 1; // 1 mojo for the DID

    if total_input < total_needed {
        return Err(WalletError::Parse); // Not enough coins
    }

    let change = total_input - total_needed;

    // Create the DID using the first coin as the parent for the launcher
    let first_coin = selected_coins[0];
    let launcher = Launcher::new(first_coin.coin_id(), 1);

    // Create the DID
    let (create_did_conditions, did) = launcher.create_simple_did(&mut ctx, &p2)?;

    // Spend all selected coins together
    let first_coin_id = first_coin.coin_id();

    for (i, &coin) in selected_coins.iter().enumerate() {
        if i == 0 {
            // First coin creates the DID and handles change/fee
            let mut conditions = create_did_conditions.clone();

            if change > 0 {
                let hint = ctx.hint(puzzle_hash)?;
                conditions = conditions.create_coin(puzzle_hash, change, hint);
            }

            if fee > 0 {
                conditions = conditions.reserve_fee(fee);
            }

            p2.spend(&mut ctx, coin, conditions)?;
        } else {
            // Other coins just assert concurrent spend
            p2.spend(
                &mut ctx,
                coin,
                Conditions::new().assert_concurrent_spend(first_coin_id),
            )?;
        }
    }

    Ok((ctx.take(), did.coin))
}

/// Resolves a DID string to find the current DID coin and generates its proof.
///
/// # Arguments
/// * `peer` - The peer to query blockchain data
/// * `did_string` - The DID string (e.g., "did:chia:1s8j4pquxfu5mhlldzu357qfqkwa9r35mdx5a0p0ehn76dr4ut4tqs0n6kv")
/// * `network` - The target network (mainnet/testnet)
///
/// # Returns
/// A tuple containing the DID proof and the current DID coin
pub async fn resolve_did_string_and_generate_proof(
    peer: &Peer,
    did_string: &str,
    network: TargetNetwork,
) -> Result<(chia::puzzles::Proof, Coin), WalletError> {
    // Parse DID string to extract launcher ID
    let parts: Vec<&str> = did_string.split(':').collect();

    if parts.len() != 3 || parts[0] != "did" || parts[1] != "chia" {
        return Err(WalletError::Parse);
    }

    let bech32_part = parts[2];

    // Decode the bech32 address to get the launcher ID
    use chia_wallet_sdk::utils::Address;
    let address = Address::decode(bech32_part).map_err(|_| WalletError::Parse)?;

    let did_id = address.puzzle_hash;

    // First, get the launcher coin state to find the first DID coin
    let launcher_states = peer
        .request_coin_state(
            vec![did_id],
            None,
            match network {
                TargetNetwork::Mainnet => MAINNET_CONSTANTS.genesis_challenge,
                TargetNetwork::Testnet11 => TESTNET11_CONSTANTS.genesis_challenge,
            },
            false,
        )
        .await?
        .map_err(|_| WalletError::RejectCoinState)?
        .coin_states;

    let launcher_state = launcher_states.first().ok_or(WalletError::UnknownCoin)?;

    // Verify this is actually a launcher
    if launcher_state.coin.puzzle_hash != SINGLETON_LAUNCHER_HASH.into() {
        return Err(WalletError::Parse);
    }

    // Get the spend of the launcher to find the first DID coin
    let launcher_spend_height = launcher_state
        .spent_height
        .ok_or(WalletError::UnknownCoin)?;

    let launcher_spend = peer
        .request_puzzle_and_solution(launcher_state.coin.coin_id(), launcher_spend_height)
        .await?
        .map_err(|_| WalletError::RejectPuzzleSolution)?;

    let mut allocator = Allocator::new();

    // Run the launcher spend to find the created DID coin
    let launcher_puzzle = launcher_spend.puzzle.to_clvm(&mut allocator)?;
    let launcher_solution = launcher_spend.solution.to_clvm(&mut allocator)?;

    let output = clvmr::run_program(
        &mut allocator,
        &clvmr::ChiaDialect::new(0),
        launcher_puzzle,
        launcher_solution,
        u64::MAX,
    )
    .map_err(|_| WalletError::Clvm)?;

    let conditions =
        Vec::<Condition>::from_clvm(&allocator, output.1).map_err(|_| WalletError::Parse)?;

    // Find the CREATE_COIN condition to get the first DID coin
    let mut first_did_coin: Option<Coin> = None;
    for condition in conditions {
        if let Some(create_coin) = condition.into_create_coin() {
            // DID coins have odd amounts (singleton property)
            if create_coin.amount % 2 == 1 {
                first_did_coin = Some(Coin::new(
                    launcher_state.coin.coin_id(),
                    create_coin.puzzle_hash,
                    create_coin.amount,
                ));
                break;
            }
        }
    }

    let first_did_coin = first_did_coin.ok_or(WalletError::Parse)?;

    // Now we need to trace the DID through all its spends to find the current coin
    let mut current_did_coin = first_did_coin;

    loop {
        // Check if this coin is spent
        let coin_states = peer
            .request_coin_state(
                vec![current_did_coin.coin_id()],
                None,
                match network {
                    TargetNetwork::Mainnet => MAINNET_CONSTANTS.genesis_challenge,
                    TargetNetwork::Testnet11 => TESTNET11_CONSTANTS.genesis_challenge,
                },
                false,
            )
            .await?
            .map_err(|_| WalletError::RejectCoinState)?
            .coin_states;

        let coin_state = coin_states.first().ok_or(WalletError::UnknownCoin)?;

        // If not spent, this is our current DID coin
        if coin_state.spent_height.is_none() {
            break;
        }

        // If spent, find the child DID coin
        let spend_height = coin_state.spent_height.unwrap();
        let spend = peer
            .request_puzzle_and_solution(current_did_coin.coin_id(), spend_height)
            .await?
            .map_err(|_| WalletError::RejectPuzzleSolution)?;

        // Parse the spend to find the child DID coin
        let spend_puzzle = spend.puzzle.to_clvm(&mut allocator)?;
        let spend_solution = spend.solution.to_clvm(&mut allocator)?;

        let spend_output = clvmr::run_program(
            &mut allocator,
            &clvmr::ChiaDialect::new(0),
            spend_puzzle,
            spend_solution,
            u64::MAX,
        )
        .map_err(|_| WalletError::Clvm)?;

        let spend_conditions = Vec::<Condition>::from_clvm(&allocator, spend_output.1)
            .map_err(|_| WalletError::Parse)?;

        // Find the CREATE_COIN condition for the child DID
        let mut child_did_coin: Option<Coin> = None;
        for condition in spend_conditions {
            if let Some(create_coin) = condition.into_create_coin() {
                // DID coins have odd amounts (singleton property)
                if create_coin.amount % 2 == 1 {
                    child_did_coin = Some(Coin::new(
                        current_did_coin.coin_id(),
                        create_coin.puzzle_hash,
                        create_coin.amount,
                    ));
                    break;
                }
            }
        }

        current_did_coin = child_did_coin.ok_or(WalletError::Parse)?;
    }

    // Now generate the proof for the current DID coin
    let proof = generate_did_proof_from_chain(peer, current_did_coin, network).await?;

    Ok((proof, current_did_coin))
}
