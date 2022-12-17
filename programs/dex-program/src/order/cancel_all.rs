use std::cell::RefCell;

use crate::{
    collections::{MountMode, OrderBook, PagedList},
    dex::{Dex, MarketInfo},
    errors::{DexError, DexResult},
    order::{select_side, Order},
    user::state::*,
    utils::ORDER_POOL_MAGIC_BYTE,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CancelAllOrders<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

fn cancel_market_orders(
    market: usize,
    mi: &MarketInfo,
    us: &RefCell<UserState>,
    remaining_accounts: &[AccountInfo],
    offset: usize,
) -> DexResult {
    let order_book = &remaining_accounts[offset];
    let order_pool_entry_page = &remaining_accounts[offset + 1];

    let next_offset = offset + 2;

    require_eq!(
        mi.order_book,
        order_book.key(),
        DexError::InvalidRemainingAccounts
    );
    require_eq!(
        mi.order_pool_entry_page,
        order_pool_entry_page.key(),
        DexError::InvalidRemainingAccounts
    );

    let order_book = OrderBook::mount(order_book, true)?;
    let order_pool = PagedList::<Order>::mount(
        order_pool_entry_page,
        &remaining_accounts
            [next_offset..mi.order_pool_remaining_pages_number as usize + next_offset],
        ORDER_POOL_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedMountOrderPool)?;

    let orders = us.borrow().collect_market_orders(market as u8);
    for user_order_slot in orders {
        let order_slot = us
            .borrow_mut()
            .get_order_info(user_order_slot)
            .map_err(|_| DexError::InvalidOrderSlot)?;

        let order = match order_pool.from_index(order_slot) {
            Ok(o) => {
                if o.in_use() && o.data.user_order_slot == user_order_slot {
                    o
                } else {
                    continue;
                }
            }
            Err(_) => continue,
        };

        let (_, open, long, _, _) = us
            .borrow_mut()
            .unlink_order(user_order_slot, true)
            .map_err(|_| DexError::InvalidOrderSlot)?;

        order_book.unlink_order(select_side(open, long), order, &order_pool)?;
    }

    Ok(())
}

/// Layout of remaining counts:
/// Markets.map( {
///   order book account
///   order pool entry page
///   order pool remaining pages
/// } )
pub fn handler(ctx: Context<CancelAllOrders>) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;

    require_eq!(
        dex.markets
            .iter()
            .filter(|m| m.valid)
            .map(|m| m.order_pool_remaining_pages_number as usize + 2)
            .reduce(|a, b| a + b)
            .ok_or(DexError::InvalidRemainingAccounts)?,
        ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts,
    );
    // Mount user state
    let us = UserState::mount(&ctx.accounts.user_state, true)?;

    let mut offset = 0usize;
    for market in 0..dex.markets_number as usize {
        let mi = &dex.markets[market];
        if !mi.valid {
            continue;
        }

        cancel_market_orders(market, mi, &us, ctx.remaining_accounts, offset)?;

        offset += 2 + mi.order_pool_remaining_pages_number as usize;
    }

    Ok(())
}
