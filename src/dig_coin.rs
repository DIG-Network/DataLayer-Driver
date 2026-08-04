use crate::error::WalletError;
use crate::wallet::DIG_ASSET_ID;
use crate::{Bytes32, Coin, Peer};
use chia_protocol::CoinState;
use chia_puzzle_types::cat::CatArgs;
use chia_wallet_sdk::driver::{Asset, Cat, Puzzle, SpendContext};
use chia_wallet_sdk::prelude::{TreeHash, MAINNET_CONSTANTS};

pub struct DigCoin {
    cat: Cat,
}

impl DigCoin {
    #[inline]
    pub fn cat(&self) -> Cat {
        self.cat
    }

    pub fn puzzle_hash(wallet_puzzle_hash: Bytes32) -> Bytes32 {
        let ph_bytes =
            CatArgs::curry_tree_hash(DIG_ASSET_ID, TreeHash::from(wallet_puzzle_hash)).to_bytes();
        Bytes32::from(ph_bytes)
    }

    pub async fn from_coin_state(peer: &Peer, coin_state: &CoinState) -> Result<Self, WalletError> {
        let coin_created_height = coin_state.created_height.ok_or(WalletError::Parse(
            "Cannot determine coin creation height".to_string(),
        ))?;
        Self::from_coin(peer, &coin_state.coin, coin_created_height).await
    }

    /// Function to validate that a coin is a $DIG CAT coin. Returns an instantiated DIG token
    /// CAT for the coin if it's a valid $DIG CAT
    pub async fn from_coin(
        peer: &Peer,
        coin: &Coin,
        coin_created_height: u32,
    ) -> Result<Self, WalletError> {
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
        let parent_puzzle = Puzzle::parse(&ctx, parent_puzzle_ptr);

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

        Ok(Self { cat: proved_cat })
    }
}
