use crate::{
    collections::{MountMode, OrderBook, PagedList},
    dex::Dex,
    errors::{DexError, DexResult},
    order::{select_side, Order},
    user::state::*,
    utils::ORDER_POOL_MAGIC_BYTE,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CancelOrder<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= order_book.owner == program_id)]
    pub order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_pool_entry_page.owner == program_id)]
    pub order_pool_entry_page: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

/// Layout of remaining counts:
/// 1. Order pool remaining pages
pub fn handler(ctx: Context<CancelOrder>, user_order_slot: u8) -> DexResult {
    // Mount user state
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let order_slot = us
        .borrow_mut()
        .get_order_info(user_order_slot)
        .map_err(|_| DexError::InvalidOrderSlot)?;

    let (market, open, long) = us
        .borrow_mut()
        .unlink_order(user_order_slot)
        .map_err(|_| DexError::InvalidOrderSlot)?;

    let dex = &mut ctx.accounts.dex.load_mut()?;
    require!(market < dex.markets_number, DexError::InvalidMarketIndex);

    let mi = &dex.markets[market as usize];
    require!(
        mi.valid
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

    let ai = &dex.assets[mi.asset_index as usize];
    require!(ai.valid, DexError::InvalidMarketIndex);

    // Mount order book & order pool
    let order_book = OrderBook::mount(&ctx.accounts.order_book, true)?;
    let order_pool = PagedList::<Order>::mount(
        &ctx.accounts.order_pool_entry_page,
        &ctx.remaining_accounts,
        ORDER_POOL_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedMountOrderPool)?;

    let order = order_pool
        .from_index(order_slot)
        .map_err(|_| DexError::InvalidOrderSlot)?;

    require!(order.in_use(), DexError::InvalidOrderSlot);

    require_eq!(
        order.data.user_order_slot,
        user_order_slot,
        DexError::InvalidOrderSlot
    );

    order_book.unlink_order(select_side(open, long), order, &order_pool)
}
