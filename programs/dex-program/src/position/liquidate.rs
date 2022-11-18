use crate::{
    collections::{EventQueue, MountMode, PagedList},
    dex::{
        event::{AppendEvent, PositionAct},
        get_oracle_price, Dex, UserListItem,
    },
    errors::{DexError, DexResult},
    position::update_user_serial_number,
    user::state::*,
    utils::{SafeMath, USER_LIST_MAGIC_BYTE},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct LiquidatePosition<'info> {
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
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= user_list_entry_page.owner == program_id)]
    pub user_list_entry_page: UncheckedAccount<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,
}

// Layout of remaining counts:
//  offset 0 ~ n: user_list remaining pages
pub fn handler(ctx: Context<LiquidatePosition>, market: u8, long: bool) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;
    require!(
        (market < dex.markets.len() as u8)
            && dex.event_queue == ctx.accounts.event_queue.key()
            && dex.user_list_entry_page == ctx.accounts.user_list_entry_page.key(),
        DexError::InvalidMarketIndex
    );

    require!(
        dex.user_list_remaining_pages_number as usize == ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts
    );

    let mi = &dex.markets[market as usize];
    require!(
        mi.valid && mi.oracle == ctx.accounts.oracle.key(),
        DexError::InvalidMarketIndex
    );

    let ai = &dex.assets[mi.asset_index as usize];
    require!(
        ai.valid
            && ai.mint == ctx.accounts.mint.key()
            && ai.vault == ctx.accounts.vault.key()
            && ai.program_signer == ctx.accounts.program_signer.key(),
        DexError::InvalidMarketIndex
    );

    let seeds = &[
        ctx.accounts.mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[ai.nonce],
    ];
    // Get oracle price
    let price = get_oracle_price(mi.oracle_source, &ctx.accounts.oracle)?;

    let mfr = mi.get_fee_rates(ai.borrow_fee_rate);

    // User close position
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let size = us.borrow().get_position_size(market, long)?;
    let (borrow, collateral, pnl, close_fee, borrow_fee) = us
        .borrow_mut()
        .close_position(market, size, price, long, &mfr, true)?;

    // Update market global position
    dex.decrease_global_position(market, long, size, collateral)?;

    let withdrawable =
        dex.settle_pnl(market, long, collateral, borrow, pnl, close_fee, borrow_fee)?;

    // Should the position be liquidated?
    let threshold = collateral.safe_mul(15u64)?.safe_div(100u128)? as u64;
    if withdrawable > threshold {
        return Err(error!(DexError::NeedNoLiquidation));
    }

    if withdrawable > 0 {
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.user_mint_acc.to_account_info(),
            authority: ctx.accounts.program_signer.to_account_info(),
        };

        // let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx =
            CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_accounts, signer);
        token::transfer(cpi_ctx, withdrawable)?;
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
        &ctx.remaining_accounts,
        USER_LIST_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedInitializeUserList)?;

    update_user_serial_number(&user_list, us.borrow_mut(), ctx.accounts.user_state.key())
}
