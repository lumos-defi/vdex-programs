use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

use crate::{dex::Dex, errors::DexError, errors::DexResult, user::UserState};

#[derive(Accounts)]
pub struct StakeVdx<'info> {
    #[account(mut)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut)]
    vault: AccountInfo<'info>,

    #[account(
         mut,
         constraint = (user_mint_acc.owner == *authority.key)
     )]
    user_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    pub authority: Signer<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,
}

// Remaining accounts layout:
// dex.assets.map({
//   asset index price oracle account
// })
pub fn handler(ctx: Context<StakeVdx>, amount: u64) -> DexResult {
    let mut dex = &mut ctx.accounts.dex.load_mut()?;
    let us = UserState::mount(&ctx.accounts.user_state, true)?;

    let assets_oracles_len = dex.assets.iter().filter(|a| a.valid).count();
    require!(
        assets_oracles_len == ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts
    );

    let mut i = 0usize;
    for asset in dex.assets.iter().filter(|a| a.valid) {
        require!(
            asset.oracle == ctx.remaining_accounts[i].key(),
            DexError::InvalidRemainingAccounts
        );
        i += 1;
    }

    require!(
        ctx.accounts.vault.key() == dex.vdx_pool.vault,
        DexError::InvalidVault
    );
    require!(
        ctx.accounts.user_mint_acc.mint.key() == dex.vdx_pool.mint,
        DexError::InvalidUserMintAccount
    );

    let reward_asset_debt =
        dex.update_staking_pool(&ctx.remaining_accounts[0..assets_oracles_len])?;
    require!(reward_asset_debt == 0, DexError::InsufficientSolLiquidity);

    us.borrow_mut().stake_and_compound_vdx(&mut dex, amount)?;

    if amount > 0 {
        // Transfer vdx
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_mint_acc.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;
    }

    Ok(())
}
