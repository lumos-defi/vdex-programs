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
    utils::USER_LIST_MAGIC_BYTE,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Crank<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub mint: AccountInfo<'info>,

    /// CHECK
    pub oracle: AccountInfo<'info>,

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

    require!(
        ctx.accounts.user_state.key().to_bytes() == data.user_state,
        DexError::UserStateMismatch
    );

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let order = us.borrow().get_order(data.user_order_slot)?;
    us.borrow_mut().unlink_order(data.user_order_slot)?;

    require_eq!(
        order.order_slot,
        data.order_slot,
        DexError::OrderSlotMismatch
    );

    require!(
        order.market < dex.markets_number
            && dex.event_queue == ctx.accounts.event_queue.key()
            && dex.user_list_entry_page == ctx.accounts.user_list_entry_page.key(),
        DexError::InvalidMarketIndex
    );

    let mi = &dex.markets[order.market as usize];
    require!(
        mi.valid && mi.oracle == ctx.accounts.oracle.key(),
        DexError::InvalidMarketIndex
    );

    let ai = if order.long {
        &dex.assets[mi.asset_index as usize]
    } else {
        &dex.assets[dex.usdc_asset_index as usize]
    };
    let mfr = mi.get_fee_rates(ai.borrow_fee_rate);
    let user_state_key = ctx.accounts.user_state.key().to_bytes();

    if order.open {
        require_neq!(order.size, 0u64, DexError::InvalidAmount);

        let mut sufficient_fund = false;
        if let Ok(borrow) = us
            .borrow()
            .calc_borrow_fund(order.size, order.leverage, &mfr)
        {
            if let Ok(_) = dex.has_sufficient_fund(order.market, order.long, borrow) {
                sufficient_fund = true;
            }
        }

        if !sufficient_fund {
            match_queue.remove_head()?;
            return Ok(());
        }

        let (size, collateral, borrow, open_fee) = us.borrow_mut().open_position(
            order.market,
            order.price,
            order.size,
            order.long,
            order.leverage,
            &mfr,
        )?;

        let _ = dex.borrow_fund(order.market, order.long, collateral, borrow, open_fee);

        // Update market global position
        dex.increase_global_position(order.market, order.long, order.price, size, collateral)?;

        // Save to event queue
        event_queue.fill_position(
            user_state_key,
            order.market,
            PositionAct::Open,
            order.long,
            order.price,
            order.size,
            collateral,
            borrow,
            open_fee,
            0,
            0,
        )?;
    } else {
        let seeds = &[
            ctx.accounts.mint.key.as_ref(),
            ctx.accounts.dex.to_account_info().key.as_ref(),
            &[ai.nonce],
        ];

        require!(
            ai.valid
                && ai.mint == ctx.accounts.mint.key()
                && ai.vault == ctx.accounts.vault.key()
                && ai.program_signer == ctx.accounts.program_signer.key(),
            DexError::InvalidMarketIndex
        );

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
            let cpi_accounts = Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.user_mint_acc.to_account_info(),
                authority: ctx.accounts.program_signer.to_account_info(),
            };

            // let cpi_program = ctx.accounts.token_program.clone();
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.clone(),
                cpi_accounts,
                signer,
            );
            token::transfer(cpi_ctx, withdrawable)?;
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
