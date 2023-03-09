use anchor_lang::prelude::*;

use crate::{
    dex::Dex,
    dual_invest::DI,
    errors::{DexError, DexResult},
};

#[derive(Accounts)]
pub struct DiUpdateOption<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<DiUpdateOption>, id: u64, premium_rate: u16, stop: bool) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;
    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );

    let di = DI::mount(&ctx.accounts.di_option, true)?;
    require!(
        di.borrow().meta.admin == ctx.accounts.authority.key()
            || dex.authority.key() == ctx.accounts.authority.key(),
        DexError::InvalidDIAdmin
    );

    di.borrow_mut().update(id, premium_rate, stop)?;

    Ok(())
}
