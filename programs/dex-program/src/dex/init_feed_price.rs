use anchor_lang::prelude::*;

use crate::{
    dex::state::*,
    errors::{DexError, DexResult},
    utils::FEED_PRICE_MAGIC_NUMBER,
};

#[derive(Accounts)]
pub struct InitFeedPrice<'info> {
    #[account(
        mut,
        has_one = authority,
    )]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(zero)]
    pub feed_price: AccountLoader<'info, FeedPrice>,

    /// CHECK
    #[account(mut)]
    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<InitFeedPrice>) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    require!(
        dex.delegate == ctx.accounts.authority.key()
            || dex.authority == ctx.accounts.authority.key(),
        DexError::InvalidAdminOrDelegate
    );

    require!(
        dex.feed_price == Pubkey::default(),
        DexError::AccountHasAlreadyBeenInitialized
    );

    let feed_price = &mut ctx.accounts.feed_price.load_init()?;

    feed_price.magic = FEED_PRICE_MAGIC_NUMBER;
    feed_price.authority = ctx.accounts.authority.key();

    dex.feed_price = ctx.accounts.feed_price.key();

    Ok(())
}
