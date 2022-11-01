use anchor_lang::prelude::*;

use crate::errors::DexResult;

use super::MockOracle;

#[derive(Accounts)]
pub struct FeedMockOraclePrice<'info> {
    #[account(mut)]
    pub mock_oracle: Account<'info, MockOracle>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<FeedMockOraclePrice>, price: u64) -> DexResult {
    let mock_oracle = &mut ctx.accounts.mock_oracle;

    mock_oracle.price = price;

    Ok(())
}
