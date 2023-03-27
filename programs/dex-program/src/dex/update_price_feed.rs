use anchor_lang::prelude::*;

use crate::{
    dex::state::*,
    errors::{DexError, DexResult},
    utils::time::get_timestamp,
};

#[derive(Accounts)]
pub struct FeedPrice<'info> {
    #[account(has_one = authority, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub price_feed: AccountLoader<'info, PriceFeed>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<FeedPrice>, prices: [u64; 16]) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;
    let price_feed = &mut ctx.accounts.price_feed.load_mut()?;

    require!(
        dex.delegate == ctx.accounts.authority.key()
            || dex.authority == ctx.accounts.authority.key(),
        DexError::InvalidAdminOrDelegate
    );

    require!(
        prices.len() == dex.assets.iter().filter(|x| x.valid).count(),
        DexError::InvalidPricesLength
    );

    let time = get_timestamp()?;

    for i in 0..prices.len() {
        let price = &mut price_feed.prices[i];
        let last_update_time = price.last_update_time;
        let last_update_index = price
            .asset_prices
            .iter()
            .position(|price| price.update_time >= last_update_time || price.update_time == 0)
            .ok_or(DexError::InvalidPriceFeedIndex)? as usize;

        let update_index = if last_update_index == 4 {
            0
        } else {
            last_update_index + 1
        };
        let asset_price = &mut price.asset_prices[update_index];

        asset_price.price = prices[i];
        asset_price.update_time = time;

        price.last_update_time = time;
    }

    Ok(())
}
