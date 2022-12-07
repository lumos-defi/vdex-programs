use crate::{
    collections::{EventQueue, MountMode, PagedList, SingleEvent, SingleEventQueue},
    dex::{
        event::{AppendEvent, PositionAct},
        Dex, UserListItem,
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
    pub user: AccountInfo<'info>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), user.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = (user_mint_acc.mint == *market_mint.key)
    )]
    pub user_mint_acc: Box<Account<'info, TokenAccount>>,

    /// Possibly used for bid order that needs swap assets
    /// CHECK
    pub in_mint_oracle: AccountInfo<'info>,

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
}

/// Layout of remaining counts:
///  offset 0 ~ n: user_list remaining pages
pub fn handler(ctx: Context<Crank>) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;
    require_eq!(
        dex.user_list_remaining_pages_number as usize,
        ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts
    );

    let mut match_queue =
        SingleEventQueue::<MatchEvent>::mount(&mut ctx.accounts.match_queue, true)
            .map_err(|_| DexError::FailedMountMatchQueue)?;

    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    let SingleEvent { data } = match_queue.read_head()?;

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let order = us.borrow().get_order(data.user_order_slot)?;
    us.borrow_mut().unlink_order(data.user_order_slot)?;

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

    if order.open {
        require_neq!(order.size, 0u64, DexError::InvalidAmount);

        let ai = dex.asset_as_ref(order.asset)?;
        require!(
            ai.valid && ai.oracle == ctx.accounts.in_mint_oracle.key(),
            DexError::InvalidAssetIndex
        );

        // Check if need to swap asset before opening position
        let actual_amount = if ai.mint == mai.mint {
            order.size
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
            let (out, fee) =
                dex.swap(order.asset, market_asset_index, order.size, true, &oracles)?;

            dex.swap_in(order.asset, order.size.safe_sub(fee)?, fee)?;
            dex.swap_out(market_asset_index, out)?;

            out
        };

        // Ready to open
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
    } else {
        if ctx.accounts.market_mint.key() == token::spl_token::native_mint::id() {
            require_eq!(
                ctx.accounts.user_mint_acc.owner,
                ctx.accounts.authority.key(),
                DexError::InvalidUserMintAccount
            );
        } else {
            require_eq!(
                ctx.accounts.user_mint_acc.owner,
                ctx.accounts.user.key(),
                DexError::InvalidUserMintAccount
            );
        }

        let seeds = &[
            ctx.accounts.market_mint.key.as_ref(),
            ctx.accounts.dex.to_account_info().key.as_ref(),
            &[mai.nonce],
        ];

        let (borrow, collateral, pnl, close_fee, borrow_fee) = us.borrow_mut().close_position(
            order.market,
            order.size,
            order.price,
            order.long,
            &mfr,
            false,
        )?;

        // Update market global position
        dex.decrease_global_position(order.market, order.long, order.size, collateral)?;

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
            let signer = &[&seeds[..]];
            let cpi_transfer = Transfer {
                from: ctx.accounts.market_mint_vault.to_account_info(),
                to: ctx.accounts.user_mint_acc.to_account_info(),
                authority: ctx.accounts.market_mint_program_signer.to_account_info(),
            };

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.clone(),
                cpi_transfer,
                signer,
            );
            token::transfer(cpi_ctx, withdrawable)?;

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
                let cpi_ctx =
                    CpiContext::new(ctx.accounts.system_program.clone(), cpi_sys_transfer);

                system_program::transfer(cpi_ctx, withdrawable)?;
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

    us.borrow_mut().unlink_order(data.user_order_slot)?;

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
