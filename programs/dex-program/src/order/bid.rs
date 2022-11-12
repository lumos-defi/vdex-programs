use crate::{
    collections::{MountMode, OrderBook, OrderSide, PagedList},
    dex::{get_oracle_price, Dex},
    errors::{DexError, DexResult},
    order::Order,
    user::state::*,
    utils::ORDER_POOL_MAGIC_BYTE,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct LimitBid<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub mint: AccountInfo<'info>,

    /// CHECK
    pub oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut, constraint= order_book.owner == program_id)]
    pub order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_pool_entry_page.owner == program_id)]
    pub order_pool_entry_page: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut)]
    pub vault: AccountInfo<'info>,

    /// CHECK
    pub program_signer: AccountInfo<'info>,

    #[account(
            mut,
            constraint = (user_mint_acc.owner == *authority.key && user_mint_acc.mint == *mint.key)
        )]
    pub user_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,
}

/// Layout of remaining counts:
/// 1. Order pool remaining pages
#[allow(clippy::too_many_arguments)]
pub fn handler(
    ctx: Context<LimitBid>,
    market: u8,
    long: bool,
    price: u64,
    amount: u64,
    leverage: u32,
) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;
    require!(market < dex.markets_number, DexError::InvalidMarketIndex);

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

    let ai = &dex.assets[mi.asset_index as usize];
    require!(
        ai.valid
            && ai.mint == ctx.accounts.mint.key()
            && ai.vault == ctx.accounts.vault.key()
            && ai.program_signer == ctx.accounts.program_signer.key(),
        DexError::InvalidMarketIndex
    );

    require_neq!(amount, 0u64, DexError::InvalidAmount);

    // Check price
    let market_price = get_oracle_price(mi.oracle_source, &ctx.accounts.oracle)?;
    if long {
        require!(market_price > price, DexError::PriceGTMarketPrice)
    } else {
        require!(market_price < price, DexError::PriceLTMarketPrice)
    }

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

    // Try to allocate from center order pool
    let order = order_pool
        .new_slot()
        .map_err(|_| DexError::NoFreeSlotInOrderPool)?;

    order
        .data
        .init(price, amount, ctx.accounts.user_state.key().to_bytes());

    // Save order in user state
    let user_order_slot = us.borrow_mut().new_bid_order(
        order.index(),
        amount,
        price,
        leverage,
        long,
        market,
        mi.decimals,
    )?;

    // Link order to order book
    let side = if long { OrderSide::BID } else { OrderSide::ASK };
    let price_node = order_book.link_order(side, order, &order_pool)?;
    order.data.set_extra_slot(price_node, user_order_slot);

    // Transfer token in
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_mint_acc.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts);
    token::transfer(cpi_ctx, amount)
}