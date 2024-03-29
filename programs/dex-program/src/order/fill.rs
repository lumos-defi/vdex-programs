use crate::{
    collections::{MountMode, OrderBook, OrderSide, OrderType, PagedList, SingleEventQueue},
    dex::{get_price, Dex, PriceFeed},
    errors::{DexError, DexResult},
    order::{AppendSingleEvent, MatchEvent, Order},
    utils::{MAX_FILLED_PER_INSTRUCTION, ORDER_POOL_MAGIC_BYTE},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct FillOrder<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub oracle: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= match_queue.owner == program_id)]
    pub match_queue: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_book.owner == program_id)]
    pub order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_pool_entry_page.owner == program_id)]
    pub order_pool_entry_page: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,
}

/// Layout of remaining accounts:
/// 1. Order pool remaining pages
pub fn handler(ctx: Context<FillOrder>, market: u8) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;
    require!(market < dex.markets_number, DexError::InvalidMarketIndex);
    require!(
        dex.match_queue == ctx.accounts.match_queue.key(),
        DexError::InvalidMatchQueue
    );

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

    for i in 0..mi.order_pool_remaining_pages_number as usize {
        require_eq!(
            mi.order_pool_remaining_pages[i],
            ctx.remaining_accounts[i].key(),
            DexError::InvalidRemainingAccounts
        );
    }

    // Mount order book & order pool
    let order_book = OrderBook::mount(&ctx.accounts.order_book, true)?;
    let order_pool = PagedList::<Order>::mount(
        &ctx.accounts.order_pool_entry_page,
        &ctx.remaining_accounts,
        ORDER_POOL_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedMountOrderPool)?;

    //Mount match queue
    let mut match_queue =
        SingleEventQueue::<MatchEvent>::mount(&mut ctx.accounts.match_queue, true)?;

    let price_feed = &ctx.accounts.price_feed.load()?;

    let market_price = get_price(
        mi.asset_index,
        mi.oracle_source,
        &ctx.accounts.oracle,
        price_feed,
    )?;

    let mut filled_bid_orders = 0u32;
    loop {
        let (user_order_slot, order_slot, user) = match order_book.get_next_match_order(
            market_price,
            OrderSide::ASK,
            OrderType::LIMIT,
            &order_pool,
        ) {
            Some((order, _)) => {
                let user_order_slot = order.data.user_order_slot;
                let order_slot = order.index();
                let user = order.data.user;

                order_book.fill_order(u64::MAX, OrderSide::ASK, order, &order_pool)?;

                (user_order_slot, order_slot, user)
            }
            None => break,
        };

        match_queue
            .append(user, order_slot, user_order_slot)
            .map_err(|_| DexError::FailedAppendMatchEvent)?;

        filled_bid_orders += 1;
        if filled_bid_orders >= MAX_FILLED_PER_INSTRUCTION {
            break;
        }
    }

    let mut filled_ask_orders = 0u32;
    loop {
        let (user_order_slot, order_slot, user) = match order_book.get_next_match_order(
            market_price,
            OrderSide::BID,
            OrderType::LIMIT,
            &order_pool,
        ) {
            Some((order, _)) => {
                let user_order_slot = order.data.user_order_slot;
                let order_slot = order.index();
                let user = order.data.user;

                order_book.fill_order(u64::MAX, OrderSide::BID, order, &order_pool)?;

                (user_order_slot, order_slot, user)
            }
            None => break,
        };

        match_queue
            .append(user, order_slot, user_order_slot)
            .map_err(|_| DexError::FailedAppendMatchEvent)?;

        filled_ask_orders += 1;
        if filled_ask_orders >= MAX_FILLED_PER_INSTRUCTION {
            break;
        }
    }

    Ok(())
}
