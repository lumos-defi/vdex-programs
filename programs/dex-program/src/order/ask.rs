use crate::{
    collections::{MountMode, OrderBook, OrderSide, PagedList},
    dex::{get_price, Dex, PriceFeed},
    errors::{DexError, DexResult},
    order::Order,
    user::state::*,
    utils::{ORDER_POOL_MAGIC_BYTE, USDC_DECIMALS},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct LimitAsk<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut, constraint= order_book.owner == program_id)]
    pub order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_pool_entry_page.owner == program_id)]
    pub order_pool_entry_page: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,
}

/// Layout of remaining accounts:
/// 1. Order pool remaining pages
pub fn handler(ctx: Context<LimitAsk>, market: u8, long: bool, price: u64, size: u64) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;
    require!(market < dex.markets_number, DexError::InvalidMarketIndex);

    require!(
        dex.price_feed == ctx.accounts.price_feed.key(),
        DexError::InvalidPriceFeed
    );

    let mi = &dex.markets[market as usize];
    require!(
        mi.valid
            && mi.oracle == ctx.accounts.oracle.key()
            && mi.order_book == ctx.accounts.order_book.key()
            && mi.order_pool_entry_page == ctx.accounts.order_pool_entry_page.key(),
        DexError::InvalidMarketIndex
    );

    require_eq!(
        mi.order_pool_remaining_pages_number as usize,
        ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts
    );

    require!(
        price % 10u64.pow((USDC_DECIMALS - mi.significant_decimals) as u32) == 0,
        DexError::InvalidSignificantDecimals
    );

    for i in 0..mi.order_pool_remaining_pages_number as usize {
        require_eq!(
            mi.order_pool_remaining_pages[i],
            ctx.remaining_accounts[i].key(),
            DexError::InvalidRemainingAccounts
        );
    }

    let ai = if long {
        &dex.assets[mi.asset_index as usize]
    } else {
        &dex.assets[dex.usdc_asset_index as usize]
    };
    require!(ai.valid, DexError::InvalidMarketIndex);

    let price_feed = &ctx.accounts.price_feed.load()?;
    // Check price
    let market_price = get_price(
        mi.asset_index,
        mi.oracle_source,
        &ctx.accounts.oracle,
        price_feed,
    )?;
    require!(market_price != price, DexError::PriceEQMarketPrice);

    let side = if market_price > price {
        OrderSide::BID
    } else {
        OrderSide::ASK
    };

    // Mount user state
    let us = UserState::mount(&ctx.accounts.user_state, true)?;

    // Mount order book & order pool
    let order_book = OrderBook::mount(&ctx.accounts.order_book, true)?;
    let order_pool = PagedList::<Order>::mount(
        &ctx.accounts.order_pool_entry_page,
        &ctx.remaining_accounts,
        ORDER_POOL_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedMountOrderPool)?;

    // Save order in user state
    let (user_order_slot, closing_size) =
        us.borrow_mut().new_ask_order(size, price, long, market)?;

    // Try to allocate from center order pool
    let order = order_pool
        .new_slot()
        .map_err(|_| DexError::NoFreeSlotInOrderPool)?;
    order
        .data
        .init(price, closing_size, ctx.accounts.authority.key().to_bytes());

    us.borrow_mut()
        .set_ask_order_slot(user_order_slot, order.index())?;

    // Link order to order book
    // let side = if long { OrderSide::ASK } else { OrderSide::BID };
    let price_node = order_book.link_order(side, order, &order_pool)?;
    order.data.set_extra_slot(price_node, user_order_slot);

    Ok(())
}
