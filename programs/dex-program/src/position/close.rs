use crate::{
    collections::{EventQueue, MountMode, PagedList},
    dex::{
        event::{AppendEvent, PositionAct},
        get_price, Dex, PriceFeed, UserListItem,
    },
    errors::{DexError, DexResult},
    position::update_user_serial_number,
    user::state::*,
    utils::USER_LIST_MAGIC_BYTE,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub vault: AccountInfo<'info>,

    /// CHECK
    pub program_signer: AccountInfo<'info>,

    #[account(
        mut,
        constraint = (user_mint_acc.owner == *authority.key)
    )]
    pub user_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump, owner = *program_id)]
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

    /// CHECK
    #[account(owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,
}

// Layout of remaining accounts:
//  offset 0 ~ n: user_list remaining pages
pub fn handler(ctx: Context<ClosePosition>, market: u8, long: bool, size: u64) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    require!(market < dex.markets_number, DexError::InvalidMarketIndex);

    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidEventQueue
    );

    require!(
        dex.user_list_entry_page == ctx.accounts.user_list_entry_page.key(),
        DexError::InvalidUserListEntryPage
    );

    require!(
        dex.price_feed == ctx.accounts.price_feed.key(),
        DexError::InvalidPriceFeed
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

    let ai = if long {
        &dex.assets[mi.asset_index as usize]
    } else {
        &dex.assets[dex.usdc_asset_index as usize]
    };

    require!(
        ai.valid
            && ai.vault == ctx.accounts.vault.key()
            && ai.program_signer == ctx.accounts.program_signer.key(),
        DexError::InvalidMarketIndex
    );
    require!(
        ai.mint == ctx.accounts.user_mint_acc.mint,
        DexError::InvalidUserMintAccount
    );
    let mint = ai.mint;

    let seeds = &[
        mint.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[ai.nonce],
    ];

    let price_feed = &ctx.accounts.price_feed.load()?;
    // Get oracle price
    let price = get_price(
        mi.asset_index,
        mi.oracle_source,
        &ctx.accounts.oracle,
        price_feed,
    )?;

    let mfr = mi.get_fee_rates(ai.borrow_fee_rate);

    // User close position
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let (borrow, collateral, pnl, closed_size, close_fee, borrow_fee) = us
        .borrow_mut()
        .close_position(market, size, price, long, &mfr, false, false)?;

    // Update market global position
    dex.decrease_global_position(market, long, closed_size, collateral)?;

    let withdrawable =
        dex.settle_pnl(market, long, collateral, borrow, pnl, close_fee, borrow_fee)?;
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
        PositionAct::Close,
        long,
        price,
        closed_size,
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
    .map_err(|_| DexError::FailedMountUserList)?;

    update_user_serial_number(&user_list, us.borrow_mut(), ctx.accounts.user_state.key())
}
