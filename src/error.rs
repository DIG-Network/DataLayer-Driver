use chia::consensus::validation_error::ValidationErr;
use chia_wallet_sdk::client::ClientError;
use chia_wallet_sdk::driver::DriverError;
use thiserror::Error;

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

    #[error("ParseError: {0}")]
    Parse(String),

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

    #[error("Failed to retrieve coin(s): {0}")]
    CoinRetrievalFailure(String),

    #[error("Puzzle hash mismatch: {0}")]
    PuzzleHashMismatch(String),

    #[error("Insufficient coin amount")]
    InsufficientCoinAmount,

    #[error("Coin is already spent")]
    CoinIsAlreadySpent,
}
