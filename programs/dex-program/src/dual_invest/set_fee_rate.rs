use anchor_lang::prelude::*;

use crate::{
    dex::Dex,
    dual_invest::DI,
    errors::{DexError, DexResult},
};

#[derive(Accounts)]
pub struct DiSetFeeRate<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<DiSetFeeRate>, fee_rate: u16) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;
    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );

    let di = DI::mount(&ctx.accounts.di_option, true)?;
    require!(
        di.borrow().meta.admin == ctx.accounts.authority.key(),
        DexError::InvalidDIAdmin
    );

    di.borrow_mut().set_fee_rate(fee_rate);

    Ok(())
}
