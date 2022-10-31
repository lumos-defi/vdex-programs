use anchor_lang::{error_code, Result};

#[error_code]
pub enum DexError {
    #[msg("Not initialized")]
    NotInitialized,

    #[msg("Invalid mint")]
    InvalidMint,

    #[msg("Invalid oracle source")]
    InvalidOracleSource,

    #[msg("Invalid program signer")]
    InvalidProgramSigner,

    #[msg("Duplicate asset")]
    DuplicateAsset,

    #[msg("Insufficient asset index")]
    InsufficientAssetIndex,
}

pub type DexResult<T = ()> = Result<T>;
