use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, Mint, MintTo, Token};

use crate::errors::DexResult;

use super::Dex;

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK: This is the token that we want to mint
    #[account(mut)]
    pub vlp_mint: Account<'info, Mint>,

    /// CHECK
    pub vlp_mint_authority: UncheckedAccount<'info>,

    /// CHECK: This is the token account that we want to mint tokens to
    #[account(mut)]
    pub user_token_account: UncheckedAccount<'info>,

    /// CHECK
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<MintToken>, amount: u64) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;

    let seeds = &[
        ctx.accounts.dex.to_account_info().key.as_ref(),
        ctx.accounts.vlp_mint.to_account_info().key.as_ref(),
        &[dex.vlp_mint_nonce],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = MintTo {
        mint: ctx.accounts.vlp_mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.vlp_mint_authority.to_account_info().clone(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

    mint_to(cpi_ctx, amount)?;

    Ok(())
}
