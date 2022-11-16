use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    collections::EventQueue,
    dex::{get_oracle_price, Dex},
    errors::DexError,
    errors::DexResult,
    pool::get_asset_aum,
    user::UserState,
    utils::{SafeMath, FEE_RATE_BASE, USDC_POW_DECIMALS},
};

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

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

// Remaining accounts layout:
// dex.assets.map({
//   asset index price oracle account
// })
// dex.markets.map({
//    market index price oracle account
// })
pub fn handler(ctx: Context<AddLiquidity>, amount: u64) -> DexResult {
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

    require_eq!(ai.vault, *ctx.accounts.vault.key, DexError::InvalidVault);

    let oracle_account = ctx
        .remaining_accounts
        .iter()
        .find(|a| a.key() == ai.oracle)
        .ok_or(DexError::InvalidOracleAccount)?;

    let asset_price = get_oracle_price(ai.oracle_source, oracle_account)?;

    let fee_amount = amount
        .safe_mul(ai.add_liquidity_fee_rate as u64)?
        .safe_div(FEE_RATE_BASE)? as u64;

    let asset_in_usdc = amount
        .safe_sub(fee_amount)?
        .safe_mul(asset_price)?
        .safe_div(10u128.pow(ai.decimals.into()))? as u64;

    // vlp_amount = asset_in_usdc * glp_supply / assets_sum
    let vlp_amount = if asset_sum == 0 {
        asset_in_usdc
            .safe_mul(10u64.pow(vlp_decimals.into()))?
            .safe_div(USDC_POW_DECIMALS as u128)? as u64
    } else {
        asset_in_usdc
            .safe_mul(vlp_supply)?
            .safe_div(10u128.pow(vlp_decimals.into()))?
            .safe_div(asset_sum as u128)?
            .safe_mul(10u128.pow(vlp_decimals.into()))?
            .safe_div(USDC_POW_DECIMALS as u128)? as u64
    };

    //Transfer assets
    let transfer_cpi_accounts = Transfer {
        from: ctx.accounts.user_mint_acc.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        transfer_cpi_accounts,
    );

    token::transfer(cpi_ctx, amount)?;

    ai.liquidity_amount += amount - fee_amount;

    // TODO: update rewards

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    us.borrow_mut()
        .enter_staking_vlp(&mut dex.vlp_pool, vlp_amount)?;

    // TODO: save to event queue
    let mut _event_queue = EventQueue::mount(&ctx.accounts.event_queue, false)?
        .initialize(true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    Ok(())
}
