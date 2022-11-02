use crate::{
    dex::{Dex, UserListItem},
    errors::{DexError, DexResult},
    user::state::*,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    mint: AccountInfo<'info>,

    /// CHECK
    pub oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    vault: AccountInfo<'info>,

    /// CHECK
    pub program_signer: AccountInfo<'info>,

    #[account(
        mut,
        constraint = (user_mint_acc.owner == *authority.key && user_mint_acc.mint == *mint.key)
    )]
    user_mint_acc: Box<Account<'info, TokenAccount>>,

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
        (market < dex.markets.len() as u8)
            && dex.event_queue == ctx.accounts.event_queue.key()
            && dex.user_list_entry_page == ctx.accounts.user_list_entry_page.key(),
        DexError::InvalidMarketIndex
    );

    // require_eq!(
    //     dex.user_list_remaining_pages_number as usize,
    //     ctx.remaining_accounts.len(),
    //     DexError::InvalidRemainingAccounts
    // );

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
    let price = 0u64;

    let mfr = mi.get_fee_rates();

    // User open position
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let (size, collateral, borrow, fee) = us
        .borrow_mut()
        .open_position(market, price, amount, long, leverage, &mfr)?;

    // TODO: check if satisfy the minimum open size

    // Update asset info (collateral amount, borrow amount, fee)
    dex.update_asset(market as usize, long, collateral, borrow, fee)?;

    // Update market global position
    dex.increase_global_position(market as usize, long, price, size, collateral)?;

    // Update user list

    Ok(())
}
