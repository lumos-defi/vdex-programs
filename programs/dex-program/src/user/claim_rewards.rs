use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, MintTo, TokenAccount, Transfer};

use crate::{
    collections::EventQueue,
    dex::{event::AppendEvent, Dex, PriceFeed},
    errors::DexError,
    errors::DexResult,
    user::UserState,
    utils::ASSET_REWARDS,
};

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut)]
    reward_vault: AccountInfo<'info>,

    /// CHECK
    pub reward_vault_program_signer: AccountInfo<'info>,

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
    #[account(mut, constraint = vdx_vault.mint == vdx_mint.key() && vdx_vault.owner == vdx_program_signer.key()  @DexError::InvalidMint)]
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
pub fn handler(ctx: Context<ClaimRewards>, amount: u64) -> DexResult {
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

    let ai = dex.asset_as_ref(dex.vlp_pool.reward_asset_index)?;

    require!(
        ctx.accounts.reward_vault.key() == ai.vault,
        DexError::InvalidVault
    );
    require!(
        ctx.accounts.user_mint_acc.mint.key() == token::spl_token::native_mint::id(),
        DexError::InvalidUserMintAccount
    );
    require!(
        ctx.accounts.reward_vault_program_signer.key() == ai.program_signer,
        DexError::InvalidProgramSigner
    );

    let reward_vault_seeds: &[&[u8]; 3] = &[
        ctx.accounts.user_mint_acc.mint.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[ai.nonce],
    ];

    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidEventQueue
    );

    let price_feed = &ctx.accounts.price_feed.load()?;

    let reward_asset_debt = dex.update_staking_pool(
        &ctx.remaining_accounts[0..assets_oracles_len],
        price_feed,
        true,
    )?;
    require!(reward_asset_debt == 0, DexError::InsufficientSolLiquidity);

    let (vdx_vested, claimable) = us.borrow_mut().claim_rewards(&mut dex, amount)?;
    if vdx_vested > 0 {
        require!(
            dex.vdx_pool.mint == ctx.accounts.vdx_mint.key(),
            DexError::InvalidMint
        );

        require!(
            dex.vdx_pool.vault == ctx.accounts.vdx_vault.key(),
            DexError::InvalidVault
        );

        require!(
            dex.vdx_pool.program_signer == ctx.accounts.vdx_program_signer.key(),
            DexError::InvalidVault
        );

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

    if claimable > 0 {
        let signer = &[&reward_vault_seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.reward_vault.to_account_info(),
            to: ctx.accounts.user_mint_acc.to_account_info(),
            authority: ctx.accounts.reward_vault_program_signer.to_account_info(),
        };

        let cpi_ctx =
            CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_accounts, signer);
        token::transfer(cpi_ctx, claimable)?;

        let cpi_close = CloseAccount {
            account: ctx.accounts.user_mint_acc.to_account_info(),
            destination: ctx.accounts.authority.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_close);
        token::close_account(cpi_ctx)?;

        // Save to event queue
        let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
            .map_err(|_| DexError::FailedMountEventQueue)?;

        let user_state_key = ctx.accounts.user_state.key().to_bytes();
        event_queue.move_liquidity(user_state_key, false, ASSET_REWARDS, claimable, 0, 0)?;
    }

    Ok(())
}
