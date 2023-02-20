use anchor_lang::prelude::*;

use crate::{
    dex::Dex,
    dual_invest::DI,
    errors::{DexError, DexResult},
};

#[derive(Accounts)]
pub struct DISetSettlePrice<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

// TODO: calculate the average price in 30 minutes before the expiry date
pub fn handler(ctx: Context<DISetSettlePrice>, id: u64, price: u64) -> DexResult {
    let dex = &mut ctx.accounts.dex.load()?;
    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );

    let di = DI::mount(&ctx.accounts.di_option, true)?;
    require!(
        di.borrow().meta.admin == ctx.accounts.authority.key(),
        DexError::InvalidDIAdmin
    );

    di.borrow_mut().set_settle_price(id, price)?;

    Ok(())
}
