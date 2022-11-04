use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo, Token, TokenAccount, Transfer};

use crate::{dex::Dex, errors::DexError, errors::DexResult};

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
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

    /// CHECK: This is the token that we want to mint
    #[account(mut)]
    pub vlp_mint: AccountInfo<'info>,

    /// CHECK
    pub vlp_mint_authority: UncheckedAccount<'info>,

    /// CHECK: This is the token account that we want to mint tokens to
    #[account(mut,
        constraint = (user_vlp_account.owner == *authority.key && user_vlp_account.mint == *vlp_mint.key))]
    pub user_vlp_account: Box<Account<'info, TokenAccount>>,

    /// CHECK
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<AddLiquidity>, amount: u64) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;
    let vlp_mint_nonce = dex.vlp_mint_nonce;

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

    //transfer to vault
    {
        let cpi_program = ctx.accounts.token_program.to_account_info().clone();
        let transfer_cpi_accounts = Transfer {
            from: ctx.accounts.user_mint_acc.to_account_info().clone(),
            to: ctx.accounts.vault.to_account_info().clone(),
            authority: ctx.accounts.authority.to_account_info().clone(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, transfer_cpi_accounts);

        token::transfer(cpi_ctx, amount)?;
    }

    //todo: calculate glp amount to mint

    //mint glp token to user
    {
        let seeds = &[
            ctx.accounts.dex.to_account_info().key.as_ref(),
            ctx.accounts.vlp_mint.to_account_info().key.as_ref(),
            &[vlp_mint_nonce],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.vlp_mint.to_account_info(),
            to: ctx.accounts.user_vlp_account.to_account_info(),
            authority: ctx.accounts.vlp_mint_authority.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info().clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::mint_to(cpi_ctx, amount)?;
    }

    asset_info.liquidity_amount += amount;

    Ok(())
}
