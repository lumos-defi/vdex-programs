use anchor_lang::prelude::*;

use crate::{dex::Dex, errors::DexResult};

#[derive(Accounts)]
pub struct SetLiquidityFeeRate<'info> {
    #[account(
        mut,
        has_one = authority,
    )]
    pub dex: AccountLoader<'info, Dex>,

    pub authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<SetLiquidityFeeRate>,
    index: u8,
    add_fee_rate: u16,
    remove_fee_rate: u16,
) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    let ai = dex.asset_as_mut(index)?;
    if add_fee_rate != u16::MAX {
        ai.add_liquidity_fee_rate = add_fee_rate;
    }

    if remove_fee_rate != u16::MAX {
        ai.remove_liquidity_fee_rate = remove_fee_rate;
    }

    Ok(())
}
