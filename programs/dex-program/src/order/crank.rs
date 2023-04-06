use crate::{
    collections::{EventQueue, MountMode, PagedList, SingleEvent, SingleEventQueue},
    dex::{
        event::{AppendEvent, PositionAct},
        AssetInfo, Dex, Position, PriceFeed, UserListItem,
    },
    errors::{DexError, DexResult},
    order::MatchEvent,
    position::update_user_serial_number,
    user::state::*,
    utils::{SafeMath, USER_LIST_MAGIC_BYTE},
};

use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, CloseAccount, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Crank<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut)]
    pub user: AccountInfo<'info>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), user.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut)]
    pub user_mint_acc: UncheckedAccount<'info>,

    /// Possibly used for bid order that needs swap assets
    /// CHECK
    pub in_mint: AccountInfo<'info>,

    /// CHECK
    pub in_mint_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub in_mint_vault: AccountInfo<'info>,

    /// CHECK
    pub in_mint_program_signer: AccountInfo<'info>,

    /// CHECK
    pub market_mint: AccountInfo<'info>,

    /// CHECK
    pub market_mint_oracle: AccountInfo<'info>,

    /// Only for ask order that transfer collateral(\w PnL) back to use account
    /// CHECK
    #[account(mut)]
    pub market_mint_vault: AccountInfo<'info>,

    /// CHECK
    pub market_mint_program_signer: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(mut, constraint= match_queue.owner == program_id)]
    pub match_queue: UncheckedAccount<'info>,

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

fn refund_in_mint(
    ctx: &Context<Crank>,
    user_mint_acc: &Account<TokenAccount>,
    in_mint_info: &AssetInfo,
    amount: u64,
) -> DexResult {
    require!(
        user_mint_acc.owner == ctx.accounts.user.key()
            && user_mint_acc.mint == ctx.accounts.in_mint.key(),
        DexError::InvalidUserMintAccount
    );

    require!(
        in_mint_info.mint == ctx.accounts.in_mint.key()
            && in_mint_info.vault == ctx.accounts.in_mint_vault.key()
            && in_mint_info.program_signer == ctx.accounts.in_mint_program_signer.key(),
        DexError::InvalidMint
    );

    let seeds = &[
        ctx.accounts.in_mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[in_mint_info.nonce],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.in_mint_vault.to_account_info(),
        to: ctx.accounts.user_mint_acc.to_account_info(),
        authority: ctx.accounts.in_mint_program_signer.to_account_info(),
    };

    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_accounts, signer);
    token::transfer(cpi_ctx, amount)
}

fn withdraw_market_mint(
    ctx: &Context<Crank>,
    user_mint_acc: &Account<TokenAccount>,
    market_asset_nonce: u8,
    amount: u64,
) -> DexResult {
    if ctx.accounts.market_mint.key() == token::spl_token::native_mint::id() {
        require!(
            user_mint_acc.owner == ctx.accounts.authority.key()
                && user_mint_acc.mint == ctx.accounts.market_mint.key(),
            DexError::InvalidUserMintAccount
        );
    } else {
        require!(
            user_mint_acc.owner == ctx.accounts.user.key()
                && user_mint_acc.mint == ctx.accounts.market_mint.key(),
            DexError::InvalidUserMintAccount
        );
    }

    let seeds = &[
        ctx.accounts.market_mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[market_asset_nonce],
    ];

    let signer = &[&seeds[..]];
    let cpi_transfer = Transfer {
        from: ctx.accounts.market_mint_vault.to_account_info(),
        to: ctx.accounts.user_mint_acc.to_account_info(),
        authority: ctx.accounts.market_mint_program_signer.to_account_info(),
    };

    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_transfer, signer);
    token::transfer(cpi_ctx, amount)?;

    // If market mint is SOL,we can't create a temp WSOL account for the end user(we are cranking, no user sign),
    // so have to use the authority as a "replay" to transfer the native mint to user
    if ctx.accounts.market_mint.key() == token::spl_token::native_mint::id() {
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

        system_program::transfer(cpi_ctx, amount)?;
    }

    Ok(())
}

/// Layout of remaining accounts:
///  offset 0 ~ n: user_list remaining pages
pub fn handler(ctx: Context<Crank>) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;
    require_eq!(
        dex.user_list_remaining_pages_number as usize,
        ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts
    );

    require!(
        dex.price_feed == ctx.accounts.price_feed.key(),
        DexError::InvalidPriceFeed
    );

    let mut match_queue =
        SingleEventQueue::<MatchEvent>::mount(&mut ctx.accounts.match_queue, true)
            .map_err(|_| DexError::FailedMountMatchQueue)?;

    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    let SingleEvent { data } = match_queue.read_head()?;

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let order = us.borrow().get_order(data.user_order_slot)?;
    require_eq!(
        order.order_slot,
        data.order_slot,
        DexError::OrderSlotMismatch
    );

    require!(
        data.user == ctx.accounts.user.key().to_bytes(),
        DexError::InvalidUser
    );

    require!(
        order.market < dex.markets_number
            && dex.event_queue == ctx.accounts.event_queue.key()
            && dex.user_list_entry_page == ctx.accounts.user_list_entry_page.key(),
        DexError::InvalidMarketIndex
    );

    let mi = &dex.markets[order.market as usize];
    require!(mi.valid, DexError::InvalidMarketIndex);

    let (market_asset_index, mai) = if order.long {
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
            && mai.program_signer == ctx.accounts.market_mint_program_signer.key(),
        DexError::InvalidMarketIndex
    );

    let mfr = mi.get_fee_rates(mai.borrow_fee_rate);
    let user_state_key = ctx.accounts.user_state.key().to_bytes();

    let user_mint_acc =
        Account::<TokenAccount>::try_from_unchecked(&ctx.accounts.user_mint_acc).ok();

    let price_feed = &ctx.accounts.price_feed.load()?;

    if order.open {
        require_neq!(order.size, 0u64, DexError::InvalidAmount);

        let ai = dex.asset_as_ref(order.asset)?;
        require!(
            ai.valid && ai.oracle == ctx.accounts.in_mint_oracle.key(),
            DexError::InvalidAssetIndex
        );

        // Check if need to swap asset before opening position
        let (need_swap, actual_amount, swap_fee) = if ai.mint == mai.mint {
            (false, order.size, 0)
        } else {
            // Swap input asset to market desired mint
            require!(
                mai.valid && mai.oracle == ctx.accounts.market_mint_oracle.key(),
                DexError::InvalidOracle
            );

            let oracles = &vec![
                &ctx.accounts.in_mint_oracle,
                &ctx.accounts.market_mint_oracle,
            ];
            let (out, fee) = dex.swap(
                order.asset,
                market_asset_index,
                order.size,
                true,
                &oracles,
                price_feed,
            )?;

            (true, out, fee)
        };

        let (_, borrow) = Position::collateral_and_borrow(
            order.long,
            order.price,
            actual_amount,
            order.leverage,
            &mfr,
        )?;

        let required_liquidity = if ai.mint == mai.mint {
            borrow
        } else {
            borrow + actual_amount
        };

        if let Err(_) = dex.has_sufficient_liquidity(order.market, order.long, required_liquidity) {
            if let Some(acc) = user_mint_acc {
                refund_in_mint(&ctx, &acc, ai, order.size)?;
            } else {
                us.borrow_mut().deposit_asset(order.asset, order.size)?;
            }
        } else {
            if need_swap {
                dex.swap_in(order.asset, order.size.safe_sub(swap_fee)?, swap_fee)?;
                dex.swap_out(market_asset_index, actual_amount)?;
            }

            // Ready to swap & open position
            let (size, collateral, borrow, open_fee) = us.borrow_mut().open_position(
                order.market,
                order.price,
                actual_amount,
                order.long,
                order.leverage,
                &mfr,
            )?;

            dex.borrow_fund(order.market, order.long, collateral, borrow, open_fee)?;
            dex.increase_global_position(order.market, order.long, order.price, size, collateral)?;
            dex.increase_volume(order.market, order.price, size)?;

            // Save to event queue
            event_queue.fill_position(
                user_state_key,
                order.market,
                PositionAct::Open,
                order.long,
                order.price,
                size,
                collateral,
                borrow,
                open_fee,
                0,
                0,
            )?;
        }
    } else {
        let market_asset_nonce = mai.nonce;

        let (borrow, collateral, pnl, closed_size, close_fee, borrow_fee) =
            us.borrow_mut().close_position(
                order.market,
                order.size,
                order.price,
                order.long,
                &mfr,
                false,
                true,
            )?;

        // Update market global position
        dex.decrease_global_position(order.market, order.long, closed_size, collateral)?;

        let withdrawable = dex.settle_pnl(
            order.market,
            order.long,
            collateral,
            borrow,
            pnl,
            close_fee,
            borrow_fee,
        )?;

        if withdrawable > 0 {
            if let Some(acc) = user_mint_acc {
                withdraw_market_mint(&ctx, &acc, market_asset_nonce, withdrawable)?;
            } else {
                us.borrow_mut()
                    .deposit_asset(market_asset_index, withdrawable)?;
            }
        }

        // Save to event queue
        event_queue.fill_position(
            user_state_key,
            order.market,
            PositionAct::Close,
            order.long,
            order.price,
            order.size,
            collateral,
            0,
            close_fee,
            borrow_fee,
            pnl,
        )?;
    }

    us.borrow_mut().unlink_order(data.user_order_slot, false)?;

    match_queue.remove_head()?;
    let user_list = PagedList::<UserListItem>::mount(
        &ctx.accounts.user_list_entry_page,
        &ctx.remaining_accounts,
        USER_LIST_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedInitializeUserList)?;

    update_user_serial_number(&user_list, us.borrow_mut(), ctx.accounts.user_state.key())
}
