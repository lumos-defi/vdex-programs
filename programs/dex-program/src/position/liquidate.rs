use crate::{
    collections::{EventQueue, MountMode, OrderBook, PagedList},
    dex::{
        event::{AppendEvent, PositionAct},
        get_price, Dex, PriceFeed, UserListItem,
    },
    errors::{DexError, DexResult},
    order::{select_side, Order},
    position::update_user_serial_number,
    user::state::*,
    utils::{SafeMath, ORDER_POOL_MAGIC_BYTE, USER_LIST_MAGIC_BYTE},
};
use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, CloseAccount, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct LiquidatePosition<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub user: AccountInfo<'info>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), user.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut)]
    pub user_mint_acc: UncheckedAccount<'info>,

    /// CHECK
    pub market_mint: AccountInfo<'info>,

    /// CHECK
    pub market_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub market_mint_vault: AccountInfo<'info>,

    /// CHECK
    pub program_signer: AccountInfo<'info>,

    /// CHECK
    #[account(mut, constraint= order_book.owner == program_id)]
    pub order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_pool_entry_page.owner == program_id)]
    pub order_pool_entry_page: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= user_list_entry_page.owner == program_id)]
    pub user_list_entry_page: UncheckedAccount<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,

    /// CHECK
    #[account(executable, constraint = (system_program.key == &system_program::ID))]
    pub system_program: AccountInfo<'info>,

    /// CHECK
    #[account(owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,
}

fn withdraw(ctx: &Context<LiquidatePosition>, seeds: &[&[u8]; 3], amount: u64) -> DexResult {
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.market_mint_vault.to_account_info(),
        to: ctx.accounts.user_mint_acc.to_account_info(),
        authority: ctx.accounts.program_signer.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.clone(),
        cpi_accounts,
        signer_seeds,
    );

    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

fn relay_native_mint_to_user(ctx: &Context<LiquidatePosition>, lamports: u64) -> DexResult {
    let cpi_close = CloseAccount {
        account: ctx.accounts.user_mint_acc.to_account_info(),
        destination: ctx.accounts.authority.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_close);
    token::close_account(cpi_ctx)?;

    let cpi_sys_transfer = system_program::Transfer {
        from: ctx.accounts.authority.to_account_info(),
        to: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.system_program.clone(), cpi_sys_transfer);

    system_program::transfer(cpi_ctx, lamports)
}

// Layout of remaining accounts:
//  offset 0 ~ m: order pool remaining pages
//  offset m + 1 ~ n: user list remaining pages
pub fn handler(ctx: Context<LiquidatePosition>, market: u8, long: bool) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    require!(
        dex.price_feed == ctx.accounts.price_feed.key(),
        DexError::InvalidPriceFeed
    );

    require!(market < dex.markets_number, DexError::InvalidMarketIndex);

    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidEventQueue
    );

    require!(
        dex.user_list_entry_page == ctx.accounts.user_list_entry_page.key(),
        DexError::InvalidUserListEntryPage
    );

    let mi = &dex.markets[market as usize];
    require!(
        mi.valid
            && mi.oracle == ctx.accounts.market_oracle.key()
            && mi.order_book == ctx.accounts.order_book.key()
            && mi.order_pool_entry_page == ctx.accounts.order_pool_entry_page.key(),
        DexError::InvalidMarketIndex
    );

    // Check remaining accounts
    require_eq!(
        mi.order_pool_remaining_pages_number as usize
            + dex.user_list_remaining_pages_number as usize,
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
    let offset = mi.order_pool_remaining_pages_number as usize;
    for i in 0..dex.user_list_remaining_pages_number as usize {
        require_eq!(
            dex.user_list_remaining_pages[i],
            ctx.remaining_accounts[offset + i].key(),
            DexError::InvalidRemainingAccounts
        );
    }

    let user_mint_acc =
        Account::<TokenAccount>::try_from_unchecked(&ctx.accounts.user_mint_acc).ok();

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
            && mai.vault == ctx.accounts.market_mint_vault.key()
            && mai.program_signer == ctx.accounts.program_signer.key(),
        DexError::InvalidMarketIndex
    );

    let seeds = &[
        ctx.accounts.market_mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[mai.nonce],
    ];

    let price_feed = &ctx.accounts.price_feed.load()?;
    // Get oracle price
    let price = get_price(
        mi.asset_index,
        mi.oracle_source,
        &ctx.accounts.market_oracle,
        price_feed,
    )?;

    let mfr = mi.get_fee_rates(mai.borrow_fee_rate);

    // User close position
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let size = us.borrow().get_position_size(market, long)?;
    let (borrow, collateral, pnl, _closed_size, close_fee, borrow_fee) = us
        .borrow_mut()
        .close_position(market, size, price, long, &mfr, true, false)?;

    // Update market global position
    dex.decrease_global_position(market, long, size, collateral)?;

    let withdrawable =
        dex.settle_pnl(market, long, collateral, borrow, pnl, close_fee, borrow_fee)?;

    // Should the position be liquidated?
    if withdrawable
        > collateral
            .safe_mul(mfr.liquidate_threshold as u64)?
            .safe_div(100u128)? as u64
    {
        return Err(error!(DexError::RequireNoLiquidation));
    }

    if user_mint_acc.is_some() {
        if let Some(acc) = user_mint_acc {
            if ctx.accounts.market_mint.key() == token::spl_token::native_mint::id() {
                require!(
                    acc.owner == ctx.accounts.authority.key()
                        && acc.mint == ctx.accounts.market_mint.key(),
                    DexError::InvalidUserMintAccount
                );
            } else {
                require!(
                    acc.owner == ctx.accounts.user.key()
                        && acc.mint == ctx.accounts.market_mint.key(),
                    DexError::InvalidUserMintAccount
                );
            }
        }

        withdraw(&ctx, seeds, withdrawable)?;

        if ctx.accounts.market_mint.key() == token::spl_token::native_mint::id() {
            relay_native_mint_to_user(&ctx, withdrawable)?;
        }
    } else {
        us.borrow_mut()
            .deposit_asset(market_asset_index, withdrawable)?;
    }

    // Cancel pending ask orders
    let order_book = OrderBook::mount(&ctx.accounts.order_book, true)?;
    let order_pool = PagedList::<Order>::mount(
        &ctx.accounts.order_pool_entry_page,
        &ctx.remaining_accounts[0..offset],
        ORDER_POOL_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedMountOrderPool)?;

    let orders = us.borrow().collect_ask_orders(market as u8, long);
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

    // Save to event queue
    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    let user_state_key = ctx.accounts.user_state.key().to_bytes();
    event_queue.fill_position(
        user_state_key,
        market,
        PositionAct::Liquidate,
        long,
        price,
        size,
        collateral,
        0,
        close_fee,
        borrow_fee,
        pnl,
    )?;

    // Update user list
    let user_list = PagedList::<UserListItem>::mount(
        &ctx.accounts.user_list_entry_page,
        &ctx.remaining_accounts[offset..],
        USER_LIST_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedMountUserList)?;

    update_user_serial_number(&user_list, us.borrow_mut(), ctx.accounts.user_state.key())
}
