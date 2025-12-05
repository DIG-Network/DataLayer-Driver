use crate::dig_coin::DigCoin;
use crate::error::WalletError;
use crate::wallet::DIG_ASSET_ID;
use crate::{Bytes, Bytes32, Coin, CoinSpend, CoinState, P2ParentCoin, Peer, PublicKey};
use chia::puzzles::Memos;
use chia::traits::Streamable;
use chia_wallet_sdk::driver::{
    Action, Id, Puzzle, Relation, SpendContext, SpendWithConditions, Spends, StandardLayer,
};
use chia_wallet_sdk::prelude::{AssertConcurrentSpend, Conditions, ToTreeHash, MAINNET_CONSTANTS};
use clvm_traits::{FromClvm, ToClvm};
use clvmr::Allocator;
use indexmap::indexmap;
use num_bigint::BigInt;

pub struct DigCollateralCoin {
    inner: P2ParentCoin,
    #[allow(dead_code)]
    morphed_store_id: Option<Bytes32>,
    #[allow(dead_code)]
    mirror_urls: Option<Vec<String>>,
}

impl DigCollateralCoin {
    /// Morphs a DIG store launcher ID into the DIG store collateral coin namespace.
    pub fn morph_store_launcher_if_for_collateral(store_launcher_id: Bytes32) -> Bytes32 {
        (store_launcher_id, "DIG_STORE_COLLATERAL")
            .tree_hash()
            .into()
    }

    /// Morphs a DIG store launcher ID into the DIG mirror collateral coin namespace.
    pub fn morph_store_launcher_id_for_mirror(
        store_launcher_id: Bytes32,
        offset: &BigInt,
    ) -> Bytes32 {
        let launcher_id_int = BigInt::from_signed_bytes_be(&store_launcher_id);
        let offset_launcher_id = launcher_id_int + offset;

        (offset_launcher_id, "DIG_STORE_MIRROR_COLLATERAL")
            .tree_hash()
            .into()
    }

    /// Instantiates a $DIG collateral coin
    /// Verifies that coin is unspent and locked by the $DIG P2Parent puzzle
    pub async fn from_coin_state(peer: &Peer, coin_state: CoinState) -> Result<Self, WalletError> {
        let coin = coin_state.coin;

        // verify coin is unspent
        if matches!(coin_state.spent_height, Some(x) if x != 0) {
            return Err(WalletError::CoinIsAlreadySpent);
        }

        // verify that the coin is $DIG p2 parent
        let p2_parent_hash = P2ParentCoin::puzzle_hash(Some(DIG_ASSET_ID));
        if coin.puzzle_hash != p2_parent_hash.into() {
            return Err(WalletError::PuzzleHashMismatch(format!(
                "Coin {} is not locked by the $DIG collateral puzzle",
                coin.coin_id()
            )));
        }

        let Some(created_height) = coin_state.created_height else {
            return Err(WalletError::UnknownCoin);
        };

        let parent_state = peer
            .request_coin_state(
                vec![coin.parent_coin_info],
                None,
                MAINNET_CONSTANTS.genesis_challenge,
                false,
            )
            .await?
            .map_err(|_| WalletError::RejectCoinState)?
            .coin_states
            .first()
            .copied()
            .ok_or(WalletError::UnknownCoin)?;

        let parent_puzzle_and_solution_response = peer
            .request_puzzle_and_solution(coin.parent_coin_info, created_height)
            .await?
            .map_err(|_| WalletError::RejectPuzzleSolution)?;

        let mut allocator = Allocator::new();
        let parent_puzzle_ptr = parent_puzzle_and_solution_response
            .puzzle
            .to_clvm(&mut allocator)?;
        let parent_solution_ptr = parent_puzzle_and_solution_response
            .solution
            .to_clvm(&mut allocator)?;

        let parent_puzzle = Puzzle::parse(&allocator, parent_puzzle_ptr);

        let (p2_parent, memos) = P2ParentCoin::parse_child(
            &mut allocator,
            parent_state.coin,
            parent_puzzle,
            parent_solution_ptr,
        )?
        .ok_or(WalletError::Parse(
            "Failed to instantiate from parent state".to_string(),
        ))?;

        let memos_vec = match memos {
            Memos::Some(node) => Vec::<Bytes>::from_clvm(&allocator, node)
                .ok()
                .unwrap_or_default(),
            Memos::None => Vec::new(),
        };

        let morphed_store_id: Option<Bytes32> = if memos_vec.is_empty() {
            None
        } else {
            Bytes32::from_bytes(&memos_vec[0]).ok()
        };

        let mut mirror_urls_vec = Vec::new();
        for memo in memos_vec.iter().skip(1) {
            if let Ok(url_string) = String::from_utf8(memo.to_vec()) {
                mirror_urls_vec.push(url_string);
            }
        }

        let mirror_urls = if mirror_urls_vec.is_empty() {
            None
        } else {
            Some(mirror_urls_vec)
        };

        Ok(Self {
            inner: p2_parent,
            morphed_store_id,
            mirror_urls,
        })
    }

    /// Uses the specified $DIG to create a collateral coin for the provided DIG store ID (launcher ID)
    pub fn create(
        dig_coins: Vec<DigCoin>,
        collateral_amount: u64,
        store_id: Bytes32,
        mirror_urls: Option<Vec<String>>,
        synthetic_key: PublicKey,
        fee_coins: Vec<Coin>,
        fee: u64,
    ) -> Result<Vec<CoinSpend>, WalletError> {
        let p2_parent_inner_hash = P2ParentCoin::inner_puzzle_hash(Some(DIG_ASSET_ID));

        let mut ctx = SpendContext::new();

        let morphed_store_id = Self::morph_store_launcher_if_for_collateral(store_id);

        let memos = match mirror_urls {
            Some(urls) => {
                let mut memos_vec = Vec::with_capacity(urls.len() + 1);
                memos_vec.push(morphed_store_id.to_vec());

                for url in &urls {
                    memos_vec.push(url.as_bytes().to_vec());
                }

                let memos_node_ptr = ctx.alloc(&memos_vec)?;
                Memos::Some(memos_node_ptr)
            }
            None => ctx.hint(morphed_store_id)?,
        };

        let actions = [
            Action::fee(fee),
            Action::send(
                Id::Existing(DIG_ASSET_ID),
                p2_parent_inner_hash.into(),
                collateral_amount,
                memos,
            ),
        ];

        let p2_layer = StandardLayer::new(synthetic_key);
        let p2_puzzle_hash: Bytes32 = p2_layer.tree_hash().into();
        let mut spends = Spends::new(p2_puzzle_hash);

        // add collateral coins to spends
        for dig_coin in dig_coins {
            spends.add(dig_coin.cat());
        }

        // add fee coins to spends
        for fee_xch_coin in fee_coins {
            spends.add(fee_xch_coin);
        }

        let deltas = spends.apply(&mut ctx, &actions)?;
        let index_map = indexmap! {p2_puzzle_hash => synthetic_key};

        let _outputs =
            spends.finish_with_keys(&mut ctx, &deltas, Relation::AssertConcurrent, &index_map)?;

        Ok(ctx.take())
    }

    /// Builds the spend bundle for spending the $DIG collateral coin to de-collateralize
    /// the store and return spendable $DIG to the wallet that created the collateral coin.
    pub fn spend(
        &self,
        synthetic_key: PublicKey,
        fee_coins: Vec<Coin>,
        fee: u64,
    ) -> Result<Vec<CoinSpend>, WalletError> {
        let p2_layer = StandardLayer::new(synthetic_key);
        let p2_puzzle_hash: Bytes32 = p2_layer.tree_hash().into();

        if p2_puzzle_hash != self.inner.proof.parent_inner_puzzle_hash {
            return Err(WalletError::PuzzleHashMismatch(
                "This coin is not owned by this wallet".to_string(),
            ));
        }

        let collateral_spend_conditions =
            Conditions::new().create_coin(p2_puzzle_hash, self.inner.coin.amount, Memos::None);

        let mut ctx = SpendContext::new();

        // add the collateral p2 parent spend to the spend context
        let p2_delegated_spend =
            p2_layer.spend_with_conditions(&mut ctx, collateral_spend_conditions)?;

        self.inner.spend(&mut ctx, p2_delegated_spend, ())?;

        // use actions and spends to attach fee to transaction and generate change
        let actions = [Action::fee(fee)];
        let mut fee_spends = Spends::new(p2_puzzle_hash);
        fee_spends
            .conditions
            .required
            .push(AssertConcurrentSpend::new(self.inner.coin.coin_id()));

        // add fee coins to spends
        for fee_xch_coin in fee_coins {
            fee_spends.add(fee_xch_coin);
        }

        let deltas = fee_spends.apply(&mut ctx, &actions)?;
        let index_map = indexmap! {p2_puzzle_hash => synthetic_key};

        let _outputs = fee_spends.finish_with_keys(
            &mut ctx,
            &deltas,
            Relation::AssertConcurrent,
            &index_map,
        )?;

        Ok(ctx.take())
    }
}
