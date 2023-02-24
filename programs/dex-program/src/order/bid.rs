use crate::{
    collections::{MountMode, OrderBook, OrderSide, PagedList},
    dex::{get_oracle_price, Dex, Position},
    errors::{DexError, DexResult},
    order::Order,
    user::state::*,
    utils::{value, LEVERAGE_POW_DECIMALS, ORDER_POOL_MAGIC_BYTE, USDC_DECIMALS},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct LimitBid<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub in_mint: AccountInfo<'info>,

    /// CHECK
    pub in_mint_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub in_mint_vault: AccountInfo<'info>,

    /// CHECK
    pub market_oracle: AccountInfo<'info>,

    /// CHECK
    pub market_mint: AccountInfo<'info>,

    /// CHECK
    pub market_mint_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut, constraint= order_book.owner == program_id)]
    pub order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_pool_entry_page.owner == program_id)]
    pub order_pool_entry_page: UncheckedAccount<'info>,

    #[account(
            mut,
            constraint = (user_mint_acc.owner == *authority.key && user_mint_acc.mint == *in_mint.key)
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

/// Layout of remaining accounts:
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
            && mi.oracle == ctx.accounts.market_oracle.key()
            && mi.order_book == ctx.accounts.order_book.key()
            && mi.order_pool_entry_page == ctx.accounts.order_pool_entry_page.key(),
        DexError::InvalidMarketIndex
    );

    require!(
        leverage >= LEVERAGE_POW_DECIMALS && leverage <= mi.max_leverage,
        DexError::InvalidLeverage
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

    let (asset, ai) = dex.find_asset_by_mint(ctx.accounts.in_mint.key())?;
    require!(
        ai.valid
            && ai.mint == ctx.accounts.in_mint.key()
            && ai.vault == ctx.accounts.in_mint_vault.key(),
        DexError::InvalidMarketIndex
    );
    require_neq!(amount, 0u64, DexError::InvalidAmount);

    // Check if the amount is too small
    // Read market asset info
    let (market_asset_index, mai) = if long {
        (mi.asset_index, &dex.assets[mi.asset_index as usize])
    } else {
        (
            dex.usdc_asset_index,
            &dex.assets[dex.usdc_asset_index as usize],
        )
    };

    require!(
        mai.valid
            && mai.mint == ctx.accounts.market_mint.key()
            && mai.oracle == ctx.accounts.market_mint_oracle.key(),
        DexError::InvalidMarketIndex
    );

    let mfr = mi.get_fee_rates(mai.borrow_fee_rate);

    let actual_amount = if ai.mint == mai.mint {
        amount
    } else {
        // Swap input asset to market required mint
        let oracles = &vec![
            &ctx.accounts.in_mint_oracle,
            &ctx.accounts.market_mint_oracle,
        ];
        let (out, _) = dex.swap(asset, market_asset_index, amount, true, &oracles)?;
        out
    };

    let (collateral, borrow) =
        Position::collateral_and_borrow(long, price, actual_amount, leverage, &mfr)?;
    let market_mint_price = get_oracle_price(mai.oracle_source, &ctx.accounts.market_mint_oracle)?;
    require!(
        value(collateral, market_mint_price, mai.decimals)? >= mi.minimum_collateral,
        DexError::CollateralTooSmall
    );

    let required_liquidity = if ai.mint == mai.mint {
        borrow
    } else {
        borrow + actual_amount
    };

    dex.has_sufficient_liquidity(market, long, required_liquidity)?;

    // Transfer token in
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_mint_acc.to_account_info(),
        to: ctx.accounts.in_mint_vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Check price
    let market_price = get_oracle_price(mi.oracle_source, &ctx.accounts.market_oracle)?;
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
        .init(price, amount, ctx.accounts.authority.key().to_bytes());

    // Save order in user state
    let user_order_slot = us.borrow_mut().new_bid_order(
        order.index(),
        amount,
        price,
        leverage,
        long,
        market,
        asset,
    )?;

    // Link order to order book
    let side = if long { OrderSide::BID } else { OrderSide::ASK };
    let price_node = order_book.link_order(side, order, &order_pool)?;
    order.data.set_extra_slot(price_node, user_order_slot);

    Ok(())
}
