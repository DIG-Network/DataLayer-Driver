use chia::{
    bls::{PublicKey, SecretKey, Signature},
    protocol::{Bytes, Bytes32, Program},
};
use napi::bindgen_prelude::*;
use napi::Result;
use thiserror::Error;

use crate::{js, rust};

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Expected different byte length {0}")]
    DifferentLength(u32),

    #[error("Invalid public key")]
    InvalidPublicKey,

    #[error("Invalid private key")]
    InvalidPrivateKey,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Missing proof")]
    MissingProof,

    #[error("Missing delegated puzzle info")]
    MissingDelegatedPuzzleInfo,

    #[error("Invalid URI: {0}")]
    InvalidUri(String),
}

pub trait FromJs<T> {
    fn from_js(value: T) -> Result<Self>
    where
        Self: Sized;
}

pub trait ToJs<T> {
    fn to_js(&self) -> Result<T>;
}

impl ToJs<Buffer> for String {
    fn to_js(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.clone()))
    }
}

impl FromJs<Buffer> for String {
    fn from_js(value: Buffer) -> Result<String> {
        let string = String::from_utf8(value.to_vec())
            .map_err(|e| Error::from_reason(format!("Invalid UTF-8: {}", e)))?;
        Ok(string)
    }
}

impl FromJs<Buffer> for Bytes32 {
    fn from_js(value: Buffer) -> Result<Self> {
        Self::try_from(value.as_ref().to_vec())
            .map_err(|_| js::err(ConversionError::DifferentLength(32)))
    }
}

impl ToJs<Buffer> for Bytes32 {
    fn to_js(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.to_vec()))
    }
}

impl FromJs<Buffer> for Program {
    fn from_js(value: Buffer) -> Result<Self> {
        Ok(Self::from(value.to_vec()))
    }
}

impl ToJs<Buffer> for Program {
    fn to_js(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.to_vec()))
    }
}

impl FromJs<Buffer> for Bytes {
    fn from_js(value: Buffer) -> Result<Self> {
        Ok(Self::new(value.to_vec()))
    }
}

impl ToJs<Buffer> for Bytes {
    fn to_js(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.to_vec()))
    }
}

impl FromJs<Buffer> for PublicKey {
    fn from_js(value: Buffer) -> Result<Self> {
        Self::from_bytes(
            &<[u8; 48]>::try_from(value.to_vec())
                .map_err(|_| js::err(ConversionError::DifferentLength(48)))?,
        )
        .map_err(|_| js::err(ConversionError::InvalidPublicKey))
    }
}

impl ToJs<Buffer> for PublicKey {
    fn to_js(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.to_bytes().to_vec()))
    }
}

impl FromJs<Buffer> for SecretKey {
    fn from_js(value: Buffer) -> Result<Self> {
        Self::from_bytes(
            &<[u8; 32]>::try_from(value.to_vec())
                .map_err(|_| js::err(ConversionError::DifferentLength(32)))?,
        )
        .map_err(|_| js::err(ConversionError::InvalidPrivateKey))
    }
}

impl ToJs<Buffer> for SecretKey {
    fn to_js(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.to_bytes().to_vec()))
    }
}

impl FromJs<Buffer> for Signature {
    fn from_js(value: Buffer) -> Result<Self> {
        Self::from_bytes(
            &<[u8; 96]>::try_from(value.to_vec())
                .map_err(|_| js::err(ConversionError::DifferentLength(96)))?,
        )
        .map_err(|_| js::err(ConversionError::InvalidSignature))
    }
}

impl ToJs<Buffer> for Signature {
    fn to_js(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.to_bytes().to_vec()))
    }
}

impl FromJs<BigInt> for u64 {
    fn from_js(value: BigInt) -> Result<Self> {
        Ok(value.get_u64().1)
    }
}

impl ToJs<BigInt> for u64 {
    fn to_js(&self) -> Result<BigInt> {
        Ok(BigInt::from(*self))
    }
}

impl FromJs<js::Coin> for rust::Coin {
    fn from_js(value: js::Coin) -> Result<Self> {
        Ok(Self {
            parent_coin_info: Bytes32::from_js(value.parent_coin_info)?,
            puzzle_hash: Bytes32::from_js(value.puzzle_hash)?,
            amount: u64::from_js(value.amount)?,
        })
    }
}

impl ToJs<js::Coin> for rust::Coin {
    fn to_js(&self) -> Result<js::Coin> {
        Ok(js::Coin {
            parent_coin_info: self.parent_coin_info.to_js()?,
            puzzle_hash: self.puzzle_hash.to_js()?,
            amount: self.amount.to_js()?,
        })
    }
}

impl FromJs<js::SimulatorPuzzle> for rust::SimulatorPuzzle {
    fn from_js(value: js::SimulatorPuzzle) -> Result<Self> {
        Ok(Self {
            puzzle_hash: Bytes32::from_js(value.puzzle_hash)?,
            puzzle_reveal: Program::from_js(value.puzzle_reveal)?,
        })
    }
}

impl ToJs<js::SimulatorPuzzle> for rust::SimulatorPuzzle {
    fn to_js(&self) -> Result<js::SimulatorPuzzle> {
        Ok(js::SimulatorPuzzle {
            puzzle_reveal: self.puzzle_reveal.to_js()?,
            puzzle_hash: self.puzzle_hash.to_js()?,
        })
    }
}

impl FromJs<js::BlsPair> for rust::BlsPair {
    fn from_js(value: js::BlsPair) -> Result<Self> {
        Ok(Self {
            puzzle_hash: Bytes32::from_js(value.puzzle_hash)?,
            pk: PublicKey::from_js(value.pk)?,
            sk: SecretKey::from_js(value.sk)?,
        })
    }
}

impl ToJs<js::BlsPair> for rust::BlsPair {
    fn to_js(&self) -> Result<js::BlsPair> {
        Ok(js::BlsPair {
            puzzle_hash: self.puzzle_hash.to_js()?,
            pk: self.pk.to_js()?,
            sk: self.sk.to_js()?,
        })
    }
}

impl FromJs<js::CoinState> for rust::CoinState {
    fn from_js(value: js::CoinState) -> Result<Self> {
        Ok(Self {
            coin: rust::Coin::from_js(value.coin)?,
            spent_height: value
                .spent_height
                .map(|height| {
                    u64::from_js(height).and_then(|height| {
                        height.try_into().map_err(|_| js::err("height exceeds u32"))
                    })
                })
                .transpose()?,
            created_height: value
                .created_height
                .map(|height| {
                    u64::from_js(height).and_then(|height| {
                        height.try_into().map_err(|_| js::err("height exceeds u32"))
                    })
                })
                .transpose()?,
        })
    }
}

impl ToJs<js::CoinState> for rust::CoinState {
    fn to_js(&self) -> Result<js::CoinState> {
        Ok(js::CoinState {
            coin: self.coin.to_js()?,
            spent_height: self
                .spent_height
                .map(|height| (height as u64).to_js())
                .transpose()?,
            created_height: self
                .created_height
                .map(|height| (height as u64).to_js())
                .transpose()?,
        })
    }
}

impl FromJs<js::CoinSpend> for rust::CoinSpend {
    fn from_js(value: js::CoinSpend) -> Result<Self> {
        Ok(Self {
            coin: rust::Coin::from_js(value.coin)?,
            puzzle_reveal: Program::from_js(value.puzzle_reveal)?,
            solution: Program::from_js(value.solution)?,
        })
    }
}

impl ToJs<js::CoinSpend> for rust::CoinSpend {
    fn to_js(&self) -> Result<js::CoinSpend> {
        Ok(js::CoinSpend {
            coin: self.coin.to_js()?,
            puzzle_reveal: self.puzzle_reveal.to_js()?,
            solution: self.solution.to_js()?,
        })
    }
}

impl FromJs<js::LineageProof> for rust::LineageProof {
    fn from_js(value: js::LineageProof) -> Result<Self> {
        Ok(Self {
            parent_parent_coin_info: Bytes32::from_js(value.parent_parent_coin_info)?,
            parent_inner_puzzle_hash: Bytes32::from_js(value.parent_inner_puzzle_hash)?,
            parent_amount: u64::from_js(value.parent_amount)?,
        })
    }
}

impl ToJs<js::LineageProof> for rust::LineageProof {
    fn to_js(&self) -> Result<js::LineageProof> {
        Ok(js::LineageProof {
            parent_parent_coin_info: self.parent_parent_coin_info.to_js()?,
            parent_inner_puzzle_hash: self.parent_inner_puzzle_hash.to_js()?,
            parent_amount: self.parent_amount.to_js()?,
        })
    }
}

impl FromJs<js::EveProof> for rust::EveProof {
    fn from_js(value: js::EveProof) -> Result<Self> {
        Ok(rust::EveProof {
            parent_parent_coin_info: Bytes32::from_js(value.parent_parent_coin_info)?,
            parent_amount: u64::from_js(value.parent_amount)?,
        })
    }
}

impl ToJs<js::EveProof> for rust::EveProof {
    fn to_js(&self) -> Result<js::EveProof> {
        Ok(js::EveProof {
            parent_parent_coin_info: self.parent_parent_coin_info.to_js()?,
            parent_amount: self.parent_amount.to_js()?,
        })
    }
}

impl FromJs<js::Proof> for rust::Proof {
    fn from_js(value: js::Proof) -> Result<Self> {
        if let Some(lineage_proof) = value.lineage_proof {
            Ok(rust::Proof::Lineage(rust::LineageProof::from_js(
                lineage_proof,
            )?))
        } else if let Some(eve_proof) = value.eve_proof {
            Ok(rust::Proof::Eve(rust::EveProof::from_js(eve_proof)?))
        } else {
            Err(js::err(ConversionError::MissingProof))
        }
    }
}

impl ToJs<js::Proof> for rust::Proof {
    fn to_js(&self) -> Result<js::Proof> {
        Ok(match self {
            rust::Proof::Lineage(lineage_proof) => js::Proof {
                lineage_proof: Some(lineage_proof.to_js()?),
                eve_proof: None,
            },
            rust::Proof::Eve(eve_proof) => js::Proof {
                lineage_proof: None,
                eve_proof: Some(eve_proof.to_js()?),
            },
        })
    }
}

impl FromJs<js::ServerCoin> for rust::ServerCoin {
    fn from_js(value: js::ServerCoin) -> Result<Self> {
        Ok(Self {
            coin: rust::Coin::from_js(value.coin)?,
            p2_puzzle_hash: Bytes32::from_js(value.p2_puzzle_hash)?,
            memo_urls: value.memo_urls,
        })
    }
}

impl ToJs<js::ServerCoin> for rust::ServerCoin {
    fn to_js(&self) -> Result<js::ServerCoin> {
        Ok(js::ServerCoin {
            coin: self.coin.to_js()?,
            p2_puzzle_hash: self.p2_puzzle_hash.to_js()?,
            memo_urls: self.memo_urls.clone(),
        })
    }
}

impl FromJs<js::SpendBundle> for rust::SpendBundle {
    fn from_js(value: js::SpendBundle) -> Result<Self> {
        Ok(Self {
            coin_spends: value
                .coin_spends
                .into_iter()
                .map(rust::CoinSpend::from_js)
                .collect::<Result<Vec<_>>>()?,
            aggregated_signature: Signature::from_js(value.aggregated_signature)?,
        })
    }
}

impl ToJs<js::SpendBundle> for rust::SpendBundle {
    fn to_js(&self) -> Result<js::SpendBundle> {
        Ok(js::SpendBundle {
            coin_spends: self
                .coin_spends
                .iter()
                .map(rust::CoinSpend::to_js)
                .collect::<Result<Vec<_>>>()?,
            aggregated_signature: self.aggregated_signature.to_js()?,
        })
    }
}

impl FromJs<js::NftMetadata> for chia::puzzles::nft::NftMetadata {
    fn from_js(value: js::NftMetadata) -> Result<Self> {
        Ok(chia::puzzles::nft::NftMetadata {
            data_uris: value.data_uris,
            data_hash: if let Some(hash) = value.data_hash {
                Some(Bytes32::from_js(hash)?)
            } else {
                None
            },
            metadata_uris: value.metadata_uris,
            metadata_hash: if let Some(hash) = value.metadata_hash {
                Some(Bytes32::from_js(hash)?)
            } else {
                None
            },
            license_uris: value.license_uris,
            license_hash: if let Some(hash) = value.license_hash {
                Some(Bytes32::from_js(hash)?)
            } else {
                None
            },
            edition_number: if let Some(num) = value.edition_number {
                u64::from_js(num)?
            } else {
                0
            },
            edition_total: if let Some(total) = value.edition_total {
                u64::from_js(total)?
            } else {
                0
            },
        })
    }
}

impl ToJs<js::NftMetadata> for chia::puzzles::nft::NftMetadata {
    fn to_js(&self) -> Result<js::NftMetadata> {
        Ok(js::NftMetadata {
            data_uris: self.data_uris.clone(),
            data_hash: if let Some(hash) = self.data_hash {
                Some(hash.to_js()?)
            } else {
                None
            },
            metadata_uris: self.metadata_uris.clone(),
            metadata_hash: if let Some(hash) = self.metadata_hash {
                Some(hash.to_js()?)
            } else {
                None
            },
            license_uris: self.license_uris.clone(),
            license_hash: if let Some(hash) = self.license_hash {
                Some(hash.to_js()?)
            } else {
                None
            },
            edition_number: if self.edition_number > 0 {
                Some(self.edition_number.to_js()?)
            } else {
                None
            },
            edition_total: if self.edition_total > 0 {
                Some(self.edition_total.to_js()?)
            } else {
                None
            },
        })
    }
}
