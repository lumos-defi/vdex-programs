use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

use crate::{
    dex::Dex,
    errors::{DexError, DexResult},
    user::UserState,
};

#[derive(Accounts)]
pub struct DiWithdrawSettled<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut)]
    pub mint_vault: AccountInfo<'info>,

    /// CHECK
    pub asset_program_signer: AccountInfo<'info>,

    #[account(
        mut,
        constraint = (user_mint_acc.owner == *authority.key)
    )]
    pub user_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,
}

pub fn handler(ctx: Context<DiWithdrawSettled>, created: u64) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let (asset_index, withdrawable) = us.borrow_mut().di_withdraw_from_settled_option(created)?;

    let ai = dex.asset_as_ref(asset_index)?;
    require!(
        ai.vault == ctx.accounts.mint_vault.key(),
        DexError::InvalidVault
    );
    require!(
        ai.program_signer == ctx.accounts.asset_program_signer.key(),
        DexError::InvalidProgramSigner
    );

    require!(
        ai.mint == ctx.accounts.user_mint_acc.mint,
        DexError::InvalidUserMintAccount
    );

    let seeds = &[
        ai.mint.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[ai.nonce],
    ];

    let cpi_accounts = Transfer {
        from: ctx.accounts.mint_vault.to_account_info(),
        to: ctx.accounts.user_mint_acc.to_account_info(),
        authority: ctx.accounts.asset_program_signer.to_account_info(),
    };

    let signer_seeds = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.clone(),
        cpi_accounts,
        signer_seeds,
    );

    token::transfer(cpi_ctx, withdrawable)
}
