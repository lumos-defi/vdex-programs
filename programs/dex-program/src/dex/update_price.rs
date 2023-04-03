use anchor_lang::prelude::*;

use crate::{
    dex::state::*,
    errors::{DexError, DexResult},
    utils::{time::get_timestamp, MAX_ASSET_COUNT, MAX_PRICE_COUNT},
};

#[derive(Accounts)]
pub struct UpdatePrice<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<UpdatePrice>, prices: [u64; MAX_ASSET_COUNT]) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;
    let price_feed = &mut ctx.accounts.price_feed.load_mut()?;

    require!(
        dex.delegate == ctx.accounts.authority.key()
            || dex.authority == ctx.accounts.authority.key(),
        DexError::InvalidAdminOrDelegate
    );

    let now = get_timestamp()?;

    for i in 0..prices.len() {
        if prices[i] == 0 {
            continue;
        }

        let price = &mut price_feed.prices[i];
        let current_cursor = price.cursor as usize;

        let next_cursor = if price.asset_prices[current_cursor].price == 0 && current_cursor == 0 {
            0
        } else if price.asset_prices[current_cursor].update_time == now {
            current_cursor
        } else {
            (current_cursor + 1) % MAX_PRICE_COUNT
        };

        let asset_price = &mut price.asset_prices[next_cursor];

        asset_price.price = prices[i];
        asset_price.update_time = now;

        price.cursor = next_cursor as u8;
    }

    price_feed.last_update_time = now;

    Ok(())
}
