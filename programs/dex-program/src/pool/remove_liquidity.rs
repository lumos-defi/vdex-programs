use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

use crate::{
    collections::EventQueue,
    dex::{event::AppendEvent, state::*},
    errors::{DexError, DexResult},
    user::UserState,
};

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut,owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    mint: AccountInfo<'info>,

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

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,
}

// Remaining accounts layout:
// dex.assets.map({
//   asset index price oracle account
// })
// dex.markets.map({
//    market index price oracle account
// })
pub fn handler(ctx: Context<RemoveLiquidity>, amount: u64) -> DexResult {
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
    require_eq!(ai.vault, *ctx.accounts.vault.key, DexError::InvalidVault);
    let seeds = &[
        ctx.accounts.mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[ai.nonce],
    ];
    let signer = &[&seeds[..]];

    let (withdraw, fee) = dex.remove_liquidity(index, amount, &ctx.remaining_accounts)?;

    if withdraw > 0 {
        // Withdraw assets
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.user_mint_acc.to_account_info(),
            authority: ctx.accounts.program_signer.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );
        token::transfer(cpi_ctx, withdraw)?;
    }

    // Update rewards
    dex.collect_rewards(&ctx.remaining_accounts[0..assets_oracles_len])?;

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    us.borrow_mut()
        .leave_staking_vlp(&mut dex.vlp_pool, amount)?;

    // Save to event queue
    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;
    event_queue.move_liquidity(
        ctx.accounts.user_state.key().to_bytes(),
        false,
        index,
        withdraw,
        amount,
        fee,
    )
}
