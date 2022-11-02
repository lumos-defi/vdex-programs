use anchor_lang::prelude::ProgramError;
#[cfg(not(test))]
use anchor_lang::prelude::{Clock, SolanaSysvar};

pub fn get_timestamp() -> Result<i64, ProgramError> {
    #[cfg(test)]
    {
        Ok(0)
    }

    #[cfg(not(test))]
    {
        Ok(Clock::get()?.unix_timestamp)
    }
}
