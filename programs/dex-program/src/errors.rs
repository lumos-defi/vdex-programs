use anchor_lang::{error_code, Result};

#[error_code]
pub enum DexError {
    #[msg("Not initialized")]
    NotInitialized,
}

pub type DexResult<T = ()> = Result<T>;
