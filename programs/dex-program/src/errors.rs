use anchor_lang::{error_code, Result};

#[error_code]
pub enum DexError {
    #[msg("Not initialized")]
    NotInitialized = 0,
}

pub type DexResult<T = ()> = Result<T>;
