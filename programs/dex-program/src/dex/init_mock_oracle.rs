use crate::{dex::state::MockOracle, errors::DexResult, utils::constant::MOCK_ORACLE_MAGIC_NUMBER};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitMockOracle<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<MockOracle>()
    )]
    pub mock_oracle: Account<'info, MockOracle>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitMockOracle>, price: u64, expo: u8) -> DexResult {
    let mock_oracle = &mut ctx.accounts.mock_oracle;

    mock_oracle.magic = MOCK_ORACLE_MAGIC_NUMBER;
    mock_oracle.price = price;
    mock_oracle.expo = expo;

    Ok(())
}
