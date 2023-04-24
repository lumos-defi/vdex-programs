use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

use crate::{
    dex::{Dex, PriceFeed},
    errors::DexError,
    errors::DexResult,
    user::UserState,
};

#[derive(Accounts)]
pub struct StakeVdx<'info> {
    #[account(mut)]
    pub dex: AccountLoader<'info, Dex>,

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
    #[account(owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,

    /// CHECK:
    pub vdx_program_signer: AccountInfo<'info>,

    /// CHECK:
    #[account(mut)]
    vdx_mint: Box<Account<'info, Mint>>,

    /// CHECK: Vault for locking asset
    #[account(mut,constraint = vdx_vault.mint == vdx_mint.key() && vdx_vault.owner == vdx_program_signer.key()  @DexError::InvalidMint)]
    vdx_vault: Box<Account<'info, TokenAccount>>,

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

    require!(
        dex.price_feed == ctx.accounts.price_feed.key(),
        DexError::InvalidPriceFeed
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
        dex.vdx_pool.mint == ctx.accounts.vdx_mint.key(),
        DexError::InvalidMint
    );

    require!(
        ctx.accounts.vdx_vault.key() == dex.vdx_pool.vault,
        DexError::InvalidVault
    );

    require!(
        ctx.accounts.vdx_program_signer.key() == dex.vdx_pool.program_signer,
        DexError::InvalidProgramSigner
    );

    require!(
        ctx.accounts.user_mint_acc.mint.key() == dex.vdx_pool.mint,
        DexError::InvalidUserMintAccount
    );

    let price_feed = &ctx.accounts.price_feed.load()?;

    let reward_asset_debt = dex.update_staking_pool(
        &ctx.remaining_accounts[0..assets_oracles_len],
        price_feed,
        true,
    )?;
    require!(reward_asset_debt == 0, DexError::InsufficientSolLiquidity);

    let vdx_vested = us.borrow_mut().stake_and_compound_vdx(&mut dex, amount)?;
    if vdx_vested > 0 {
        let seeds = &[
            dex.vdx_pool.mint.as_ref(),
            ctx.accounts.dex.to_account_info().key.as_ref(),
            &[dex.vdx_pool.nonce],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.vdx_mint.to_account_info(),
            to: ctx.accounts.vdx_vault.to_account_info(),
            authority: ctx.accounts.vdx_program_signer.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_accounts, signer);

        token::mint_to(cpi_ctx, vdx_vested)?;
    }

    if amount > 0 {
        // Transfer vdx
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_mint_acc.to_account_info(),
            to: ctx.accounts.vdx_vault.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;
    }

    Ok(())
}
