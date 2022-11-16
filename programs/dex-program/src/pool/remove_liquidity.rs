use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

use crate::{
    collections::EventQueue,
    dex::{get_oracle_price, state::*},
    errors::{DexError, DexResult},
    pool::get_asset_aum,
    user::UserState,
    utils::{SafeMath, FEE_RATE_BASE, USDC_POW_DECIMALS},
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
    require_neq!(amount, 0, DexError::InvalidWithdrawAmount);

    let dex = &mut ctx.accounts.dex.load_mut()?;
    let vlp_supply = dex.vlp_pool.staked_total;
    let vlp_decimals = dex.vlp_pool.decimals;

    let oracle_accounts_len = dex.assets.iter().filter(|a| a.valid).count()
        + dex.markets.iter().filter(|m| m.valid).count();

    require_eq!(
        oracle_accounts_len,
        ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts
    );

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
    let vlp_in_usdc = amount
        .safe_mul(asset_sum)?
        .safe_div(vlp_supply as u128)?
        .safe_mul(USDC_POW_DECIMALS as u128)?
        .safe_div(10u128.pow(vlp_decimals.into()))? as u64;

    let oracle_account = ctx
        .remaining_accounts
        .iter()
        .find(|a| a.key() == ai.oracle)
        .ok_or(DexError::InvalidOracleAccount)?;

    let asset_price = get_oracle_price(ai.oracle_source, oracle_account)?;
    let withdraw_amount = vlp_in_usdc
        .safe_mul(10u64.pow(ai.decimals.into()))?
        .safe_div(asset_price as u128)? as u64;

    msg!("withdraw amount=====>{}", withdraw_amount);
    let fee_amount = withdraw_amount
        .safe_mul(ai.remove_liquidity_fee_rate as u64)?
        .safe_div(FEE_RATE_BASE)? as u64;

    //remove liquidity
    let seeds = &[
        ctx.accounts.mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[ai.nonce],
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

    token::transfer(cpi_ctx, withdraw_amount - fee_amount)?;

    ai.liquidity_amount -= withdraw_amount;

    // TODO: update rewards

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    us.borrow_mut()
        .leave_staking_vlp(&mut dex.vlp_pool, amount)?;

    // TODO: save to event queue
    let mut _event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    Ok(())
}
