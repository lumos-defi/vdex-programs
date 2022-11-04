use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, TokenAccount, Transfer};

use crate::{
    collections::{EventQueue, MountMode, PagedList},
    dex::state::*,
    errors::{DexError, DexResult},
    utils::{NIL8, USER_LIST_MAGIC_BYTE},
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
    #[account(mut)]
    pub vlp_mint: AccountInfo<'info>,

    #[account(mut,
        constraint = (user_vlp_account.owner == *authority.key && user_vlp_account.mint == *vlp_mint.key))]
    pub user_vlp_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,
}

pub fn handler(ctx: Context<RemoveLiquidity>, amount: u64) -> DexResult {
    require_neq!(amount, 0, DexError::InvalidWithdrawAmount);

    let dex = &mut ctx.accounts.dex.load_mut()?;

    let assets = &mut dex.assets;

    let asset = assets
        .iter()
        .position(|x| x.mint == *ctx.accounts.mint.key)
        .ok_or(DexError::InvalidMint)? as u8;
    let asset_info = &mut assets[asset as usize];

    require_eq!(
        asset_info.program_signer,
        *ctx.accounts.program_signer.key,
        DexError::InvalidPDA
    );
    require_eq!(
        asset_info.vault,
        *ctx.accounts.vault.key,
        DexError::InvalidVault
    );

    //remove liquidity
    {
        let seeds = &[
            ctx.accounts.mint.key.as_ref(),
            ctx.accounts.dex.to_account_info().key.as_ref(),
            &[asset_info.nonce],
        ];
        let signer = &[&seeds[..]];

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

        token::transfer(cpi_ctx, amount)?;
    }

    //burn vlp
    {
        let burn_cpi_accounts = Burn {
            authority: ctx.accounts.authority.to_account_info(),
            from: ctx.accounts.user_vlp_account.to_account_info(),
            mint: ctx.accounts.vlp_mint.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            burn_cpi_accounts,
        );

        token::burn(cpi_ctx, amount)?;

        asset_info.liquidity_amount -= amount;
    }

    Ok(())
}
