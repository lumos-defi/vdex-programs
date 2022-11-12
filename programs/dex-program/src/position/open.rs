use crate::{
    collections::{EventQueue, MountMode, PagedList},
    dex::{
        event::{PositionAct, PositionFilled},
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
pub struct OpenPosition<'info> {
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
#[allow(clippy::too_many_arguments)]
pub fn handler(
    ctx: Context<OpenPosition>,
    market: u8,
    long: bool,
    amount: u64,
    leverage: u32,
) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;
    require!(
        market < dex.markets_number
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

    require_neq!(amount, 0u64, DexError::InvalidAmount);

    let cpi_accounts = Transfer {
        from: ctx.accounts.user_mint_acc.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Get oracle price
    let price = get_oracle_price(mi.oracle_source, &ctx.accounts.oracle)?;

    let mfr = mi.get_fee_rates(ai.borrow_fee_rate);

    // User open position
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let (size, collateral, borrow, fee) = us
        .borrow_mut()
        .open_position(market, price, amount, long, leverage, &mfr)?;

    // Check if satisfy the minimum open size
    require!(
        size.safe_mul(price)? as u64 >= mi.minimum_position_value,
        DexError::PositionTooSmall
    );

    // Update asset info (collateral amount, borrow amount, fee)
    dex.borrow_fund(market as usize, long, collateral, borrow, fee)?;

    // Update market global position
    dex.increase_global_position(market as usize, long, price, size, collateral)?;

    // Save to event queue
    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    let user_state_key = ctx.accounts.user_state.key().to_bytes();
    let event_seq = event_queue
        .append(PositionFilled {
            user_state: user_state_key,
            price,
            size,
            collateral,
            borrow,
            market,
            action: PositionAct::Open as u8,
            long_or_short: if long { 0 } else { 1 },
            fee,
            pnl: 0,
        })
        .map_err(|_| DexError::FailedAppendEvent)?;

    msg!(
        "Position filled: open {:?} {} {} {} {} {} {} {}",
        user_state_key,
        price,
        size,
        collateral,
        borrow,
        market,
        fee,
        event_seq
    );

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
