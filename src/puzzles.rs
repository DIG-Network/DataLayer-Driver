use chia_protocol::Bytes32;
use clvm_traits::{apply_constants, FromClvm, ToClvm};
use clvm_utils::ToTreeHash;
use clvm_utils::{CurriedProgram, TreeHash};
use hex_literal::hex;

#[derive(ToClvm, FromClvm)]
#[apply_constants]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[clvm(curry)]
pub struct DelegationLayerArgs {
    pub mod_hash: Bytes32,
    pub inner_puzzle_hash: Bytes32,
    pub merkle_root: Bytes32,
}

impl DelegationLayerArgs {
    pub fn new(inner_puzzle_hash: Bytes32, merkle_root: Bytes32) -> Self {
        Self {
            mod_hash: DELEGATION_LAYER_PUZZLE_HASH.into(),
            inner_puzzle_hash,
            merkle_root,
        }
    }

    pub fn curry_tree_hash(inner_puzzle_hash: Bytes32, merkle_root: Bytes32) -> TreeHash {
        CurriedProgram {
            program: DELEGATION_LAYER_PUZZLE_HASH,
            args: DelegationLayerArgs {
                mod_hash: DELEGATION_LAYER_PUZZLE_HASH.into(),
                inner_puzzle_hash,
                merkle_root,
            },
        }
        .tree_hash()
    }
}

#[derive(ToClvm, FromClvm)]
#[apply_constants]
#[derive(Debug, Clone, PartialEq, Eq)]
#[clvm(list)]
pub struct DelegationLayerSolution<P, S> {
    pub merkle_proof: Option<(u32, Vec<chia_protocol::Bytes32>)>,
    pub puzzle_reveal: P,
    pub puzzle_solution: S,
}

pub const DELEGATION_LAYER_PUZZLE: [u8; 956] = hex!(
    "
    ff02ffff01ff02ffff03ff2fffff01ff02ffff03ffff09ff17ffff02ff1effff04ff02ffff04ffff
    0bffff0101ffff02ff16ffff04ff02ffff04ff5fff8080808080ffff04ff2fff808080808080ffff
    01ff02ff1affff04ff02ffff04ff05ffff04ff0bffff04ffff02ff5fff81bf80ffff04ffff04ff17
    ff8080ffff01ff8080808080808080ffff01ff08ffff019070682070726f6f6620696e76616c6964
    8080ff0180ffff01ff02ffff03ffff09ffff02ff16ffff04ff02ffff04ff5fff80808080ff0b80ff
    ff01ff02ff5fff81bf80ffff01ff08ffff018a706820696e76616c69648080ff018080ff0180ffff
    04ffff01ffff33ff81f302ffffffffa04bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5
    d7cce23c7785459aa09dcf97a184f32623d11a73124ceb99a5709b083721e878a16d78f596718ba7
    b2ffa102a12871fee210fb8619291eaea194581cbd2531e4b23759d225f6806923f63222a102a8d5
    dd63fba471ebcb1f3e8f7c1e1879b7152a6e7298a91ce119a63400ade7c5ff02ffff03ff17ffff01
    ff02ffff03ffff22ffff09ff47ff1480ffff09ffff0dff81a780ffff01208080ffff01ff02ff1aff
    ff04ff02ffff04ff05ffff04ff0bffff04ff37ffff04ff67ffff04ff5fff8080808080808080ffff
    01ff04ff27ffff02ff1affff04ff02ffff04ff05ffff04ff0bffff04ff37ffff04ff2fffff04ffff
    21ff5fffff22ffff09ff47ff0880ffff09ffff18ff820167ffff010180ffff0101808080ff808080
    80808080808080ff0180ffff01ff02ffff03ff5fff80ffff01ff04ffff04ff08ffff04ffff0bff52
    ffff0bff1cffff0bff1cff62ff0580ffff0bff1cffff0bff72ffff0bff1cffff0bff1cff62ffff0b
    ffff0101ff058080ffff0bff1cffff0bff72ffff0bff1cffff0bff1cff62ffff0bffff0101ff0b80
    80ffff0bff1cffff0bff72ffff0bff1cffff0bff1cff62ffff0bffff0101ff4f8080ffff0bff1cff
    62ff42808080ff42808080ff42808080ff42808080ffff04ffff0101ffff04ff6fff8080808080ff
    808080ff018080ff0180ffff02ffff03ffff07ff0580ffff01ff0bffff0102ffff02ff16ffff04ff
    02ffff04ff09ff80808080ffff02ff16ffff04ff02ffff04ff0dff8080808080ffff01ff0bffff01
    01ff058080ff0180ff02ffff03ff1bffff01ff02ff1effff04ff02ffff04ffff02ffff03ffff18ff
    ff0101ff1380ffff01ff0bffff0102ff2bff0580ffff01ff0bffff0102ff05ff2b8080ff0180ffff
    04ffff04ffff17ff13ffff0181ff80ff3b80ff8080808080ffff010580ff0180ff018080
    "
);

pub const DELEGATION_LAYER_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "
    acedec639683bb8d7ec77917ae5b2f07a8fc714a5f045b23de0b529d86baa5b6
    "
));

#[derive(ToClvm, FromClvm)]
#[apply_constants]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[clvm(curry)]
pub struct AdminFilterArgs<I> {
    pub inner_puzzle: I,
}

impl<I> AdminFilterArgs<I> {
    pub fn new(inner_puzzle: I) -> Self {
        Self { inner_puzzle }
    }
}

#[derive(ToClvm, FromClvm)]
#[apply_constants]
#[derive(Debug, Clone, PartialEq, Eq)]
#[clvm(list)]
pub struct AdminFilterSolution<I> {
    pub inner_solution: I,
}

pub const ADMIN_FILTER_PUZZLE: [u8; 153] = hex!(
    "
    ff02ffff01ff02ff06ffff04ff02ffff04ffff02ff05ff0b80ff80808080ffff04ffff01ff33ff02
    ffff03ff05ffff01ff02ffff03ffff21ffff09ff11ff0480ffff09ff11ffff01818f80ffff22ffff
    09ff11ffff0181e880ffff20ffff09ff820159ff8080808080ffff01ff0880ffff01ff04ff09ffff
    02ff06ffff04ff02ffff04ff0dff808080808080ff0180ff8080ff0180ff018080
    "
);

pub const ADMIN_FILTER_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "
    6332f1941eb746501c02026c711d9cacf20e8372d33cf558b360055b602aa471
    "
));

#[derive(ToClvm, FromClvm)]
#[apply_constants]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[clvm(curry)]
pub struct WriterFilterArgs<I> {
    pub inner_puzzle: I,
}

impl<I> WriterFilterArgs<I> {
    pub fn new(inner_puzzle: I) -> Self {
        Self { inner_puzzle }
    }
}

#[derive(ToClvm, FromClvm)]
#[apply_constants]
#[derive(Debug, Clone, PartialEq, Eq)]
#[clvm(list)]
pub struct WriterFilterSolution<I> {
    pub inner_solution: I,
}

pub const WRITER_FILTER_PUZZLE: [u8; 36] = hex!(
    "
    ff02ffff01ff04ffff04ff02ffff01ff8379616b8080ff8080ffff04ffff0101ff018080
    "
);

pub const WRITER_FILTER_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "
    db508bfd0dba412fe607269466e6743453b9410c92a4fa3a1f1d6f2c582c53ca
    "
));

// bytes(ACS_MU).hex()
pub const DL_METADATA_UPDATER_PUZZLE: [u8; 1] = hex!(
    "
    0b
    "
);

pub const DL_METADATA_UPDATER_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "
    57bfd1cb0adda3d94315053fda723f2028320faa8338225d99f629e3d46d43a9
    "
));

#[cfg(test)]
mod tests {
    use super::*;

    // unfortunately, this isn't publicly exported, so I had to copy-paste
    // use chia_puzzles::assert_puzzle_hash;
    #[macro_export]
    macro_rules! assert_puzzle_hash {
        ($puzzle:ident => $puzzle_hash:ident) => {
            let mut a = clvmr::Allocator::new();
            let ptr = clvmr::serde::node_from_bytes(&mut a, &$puzzle).unwrap();
            let hash = clvm_utils::tree_hash(&mut a, ptr);
            assert_eq!($puzzle_hash, hash);
        };
    }

    #[test]
    fn puzzle_hashes() {
        assert_puzzle_hash!(DELEGATION_LAYER_PUZZLE => DELEGATION_LAYER_PUZZLE_HASH);
        assert_puzzle_hash!(ADMIN_FILTER_PUZZLE => ADMIN_FILTER_PUZZLE_HASH);
        assert_puzzle_hash!(WRITER_FILTER_PUZZLE => WRITER_FILTER_PUZZLE_HASH);
        assert_puzzle_hash!(DL_METADATA_UPDATER_PUZZLE => DL_METADATA_UPDATER_PUZZLE_HASH);
    }
}
