use anchor_lang::prelude::*;

use crate::{
    dex::state::*,
    errors::{DexError, DexResult},
    utils::FEED_PRICE_MAGIC_NUMBER,
};

#[derive(Accounts)]
pub struct InitPriceFeed<'info> {
    #[account(
        mut,
        has_one = authority,
        owner = *program_id
    )]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(zero)]
    pub price_feed: AccountLoader<'info, PriceFeed>,

    /// CHECK
    #[account(mut)]
    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<InitPriceFeed>) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    require!(
        dex.delegate == ctx.accounts.authority.key()
            || dex.authority == ctx.accounts.authority.key(),
        DexError::InvalidAdminOrDelegate
    );

    require!(
        dex.price_feed == Pubkey::default(),
        DexError::AccountHasAlreadyBeenInitialized
    );

    let feed_price = &mut ctx.accounts.price_feed.load_init()?;

    feed_price.magic = FEED_PRICE_MAGIC_NUMBER;
    feed_price.authority = ctx.accounts.authority.key();

    dex.price_feed = ctx.accounts.price_feed.key();

    Ok(())
}
