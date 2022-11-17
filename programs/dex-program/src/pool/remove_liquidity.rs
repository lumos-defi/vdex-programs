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

<<<<<<< HEAD
    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidEventQueue
    );
=======
    let asset_sum = get_asset_aum(&dex, &ctx.remaining_accounts)?;

    let assets = &mut dex.assets;

    let asset_index = assets
        .iter()
        .position(|x| x.mint == *ctx.accounts.mint.key)
        .ok_or(DexError::InvalidMint)? as u8;
    let ai = &mut assets[asset_index as usize];

    require_eq!(
        ai.program_signer,
        *ctx.accounts.program_signer.key,
        DexError::InvalidPDA
    );
    require_eq!(ai.vault, *ctx.accounts.vault.key, DexError::InvalidVault);

    // vlp_in_usdc = amount * assets_sum / glp_supply
    let vlp_in_usdc = amount.safe_mul(asset_sum)?.safe_div(vlp_supply as u128)? as u64;

    let oracle_account = ctx
        .remaining_accounts
        .iter()
        .find(|a| a.key() == ai.oracle)
        .ok_or(DexError::InvalidOracleAccount)?;

    let asset_price = get_oracle_price(ai.oracle_source, oracle_account)?;
    let withdraw_amount = vlp_in_usdc
        .safe_mul(10u64.pow(ai.decimals.into()))?
        .safe_div(asset_price as u128)? as u64;

    let fee_amount = withdraw_amount
        .safe_mul(ai.remove_liquidity_fee_rate as u64)?
        .safe_div(FEE_RATE_BASE)? as u64;
>>>>>>> 88268d1 (fix:bug)

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
