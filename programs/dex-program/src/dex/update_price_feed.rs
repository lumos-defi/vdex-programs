use anchor_lang::prelude::*;

use crate::{
    dex::state::*,
    errors::{DexError, DexResult},
    utils::time::get_timestamp,
};

#[derive(Accounts)]
pub struct UpdatePriceFeed<'info> {
    #[account(has_one = authority, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<UpdatePriceFeed>, prices: [u64; 16]) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;
    let price_feed = &mut ctx.accounts.price_feed.load_mut()?;

    require!(
        dex.delegate == ctx.accounts.authority.key()
            || dex.authority == ctx.accounts.authority.key(),
        DexError::InvalidAdminOrDelegate
    );

    require!(
        prices.iter().filter(|&x| x > &0).count() == dex.assets.iter().filter(|&x| x.valid).count(),
        DexError::InvalidPricesLength
    );

    let time = get_timestamp()?;

    for i in 0..prices.len() {
        if prices[i] == 0 {
            continue;
        }
        let price = &mut price_feed.prices[i];
        let last_update_time = price.last_update_time;
        let last_update_index = price
            .asset_prices
            .iter()
            .position(|price| price.update_time >= last_update_time || price.update_time == 0)
            .ok_or(DexError::InvalidPriceFeedIndex)? as usize;

        let update_index =
            if last_update_index == 4 || price.asset_prices[last_update_index].price == 0 {
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
