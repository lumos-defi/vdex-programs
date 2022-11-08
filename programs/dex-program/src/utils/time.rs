use anchor_lang::prelude::ProgramError;
#[cfg(not(test))]
use anchor_lang::prelude::{Clock, SolanaSysvar};

pub fn get_timestamp() -> Result<i64, ProgramError> {
    #[cfg(test)]
    {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        Ok(time)
    }

    #[cfg(not(test))]
    {
        Ok(Clock::get()?.unix_timestamp)
    }
}
