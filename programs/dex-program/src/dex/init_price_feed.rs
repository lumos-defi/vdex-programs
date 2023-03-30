use anchor_lang::prelude::*;

use crate::{
    dex::state::*,
    errors::{DexError, DexResult},
    utils::PRICE_FEED_MAGIC_NUMBER,
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

    let price_feed = &mut ctx.accounts.price_feed.load_init()?;

    price_feed.magic = PRICE_FEED_MAGIC_NUMBER;
    price_feed.authority = ctx.accounts.authority.key();

    dex.price_feed = ctx.accounts.price_feed.key();

    Ok(())
}
