use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

use crate::{
    collections::EventQueue,
    dex::{event::AppendEvent, Dex, PriceFeed},
    errors::DexError,
    errors::DexResult,
    user::UserState,
    utils::ASSET_VDX,
};

#[derive(Accounts)]
pub struct RedeemVdx<'info> {
    #[account(mut, owner = *program_id)]
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
// dex.markets.map({
//    market index price oracle account
// })
pub fn handler(ctx: Context<RedeemVdx>, amount: u64) -> DexResult {
    let mut dex = &mut ctx.accounts.dex.load_mut()?;
    let us = UserState::mount(&ctx.accounts.user_state, true)?;

    let assets_oracles_len = dex.assets.iter().filter(|a| a.valid).count();
    let expected_oracles_len = assets_oracles_len + dex.markets.iter().filter(|m| m.valid).count();

    require_eq!(
        expected_oracles_len,
        ctx.remaining_accounts.len(),
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

    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidEventQueue
    );

    let price_feed = &ctx.accounts.price_feed.load()?;

    dex.update_staking_pool(&ctx.remaining_accounts, price_feed, true)?;

    let (vdx_vested, redeemable) = us.borrow_mut().redeem_vdx(&mut dex, amount)?;

    let seeds = &[
        dex.vdx_pool.mint.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[dex.vdx_pool.nonce],
    ];
    let signer = &[&seeds[..]];
    if vdx_vested > 0 {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.vdx_mint.to_account_info(),
            to: ctx.accounts.vdx_vault.to_account_info(),
            authority: ctx.accounts.vdx_program_signer.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_accounts, signer);

        token::mint_to(cpi_ctx, vdx_vested)?;
    }

    let cpi_accounts = Transfer {
        from: ctx.accounts.vdx_vault.to_account_info(),
        to: ctx.accounts.user_mint_acc.to_account_info(),
        authority: ctx.accounts.vdx_program_signer.to_account_info(),
    };

    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_accounts, signer);
    token::transfer(cpi_ctx, redeemable)?;

    // Save to event queue
    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    let user_state_key = ctx.accounts.user_state.key().to_bytes();
    event_queue.move_liquidity(user_state_key, false, ASSET_VDX, redeemable, 0, 0)
}
