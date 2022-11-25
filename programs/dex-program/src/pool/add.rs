use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    collections::EventQueue,
    dex::{event::AppendEvent, Dex},
    errors::DexError,
    errors::DexResult,
    user::UserState,
};

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    mint: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    vault: AccountInfo<'info>,

    #[account(
         mut,
         constraint = (user_mint_acc.owner == *authority.key && user_mint_acc.mint == *mint.key)
     )]
    user_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

// Remaining accounts layout:
// dex.assets.map({
//   asset index price oracle account
// })
// dex.markets.map({
//    market index price oracle account
// })
pub fn handler(ctx: Context<AddLiquidity>, amount: u64) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    let assets_oracles_len = dex.assets.iter().filter(|a| a.valid).count();
    let expected_oracles_len = assets_oracles_len + dex.markets.iter().filter(|m| m.valid).count();

    require_eq!(
        expected_oracles_len,
        ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts
    );

    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidEventQueue
    );

    let (index, ai) = dex.find_asset_by_mint(ctx.accounts.mint.key())?;
    require_eq!(ai.vault, ctx.accounts.vault.key(), DexError::InvalidVault);

    //Transfer assets
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_mint_acc.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Update rewards
    dex.collect_rewards(&ctx.remaining_accounts[0..assets_oracles_len])?;

    let (vlp_amount, fee) = dex.add_liquidity(index, amount, &ctx.remaining_accounts)?;

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    us.borrow_mut()
        .enter_staking_vlp(&mut dex.vlp_pool, vlp_amount)?;

    // Save to event queue
    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    event_queue.move_liquidity(
        ctx.accounts.user_state.key().to_bytes(),
        true,
        index,
        amount,
        vlp_amount,
        fee,
    )
}
