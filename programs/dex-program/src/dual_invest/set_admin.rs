use anchor_lang::prelude::*;

use crate::{
    dex::Dex,
    dual_invest::DI,
    errors::{DexError, DexResult},
};

#[derive(Accounts)]
pub struct DISetAdmin<'info> {
    #[account(owner = *program_id, has_one = authority)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    /// CHECK
    pub admin: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<DISetAdmin>) -> DexResult {
    let dex = &mut ctx.accounts.dex.load()?;
    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );

    let di = DI::mount(&ctx.accounts.di_option, true)?;
    di.borrow_mut().set_admin(ctx.accounts.admin.key());

    Ok(())
}
