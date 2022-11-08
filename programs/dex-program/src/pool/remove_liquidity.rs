use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, TokenAccount, Transfer};

use crate::{
    dex::{get_oracle_price, state::*},
    errors::{DexError, DexResult},
    pool::get_asset_aum,
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

    require_eq!(
        dex.vlp_mint,
        ctx.accounts.vlp_mint.key(),
        DexError::InvalidVlpMint
    );

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
    let asset_info = &mut assets[asset_index as usize];

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

    let vlp_mint_info =
        Mint::try_deserialize(&mut &**ctx.accounts.vlp_mint.try_borrow_mut_data()?)?;

    // vlp_in_usdc = amount * assets_sum / glp_supply
    let vlp_in_usdc = amount
        .safe_mul(asset_sum)?
        .safe_div(vlp_mint_info.supply as u128)?
        .safe_mul(USDC_POW_DECIMALS as u128)?
        .safe_div(10u128.pow(vlp_mint_info.decimals.into()))? as u64;

    msg!(
        "vlp_in_usdc:{},amount:{},asset_sum:{},vlp_supply:{},vlp_decimals:{}",
        vlp_in_usdc,
        amount,
        asset_sum,
        vlp_mint_info.supply,
        vlp_mint_info.decimals
    );

    let oracle_account = ctx
        .remaining_accounts
        .iter()
        .find(|a| a.key() == asset_info.oracle)
        .ok_or(DexError::InvalidOracleAccount)?;

    let asset_price = get_oracle_price(asset_info.oracle_source, oracle_account)?;
    let withdraw_amount = vlp_in_usdc
        .safe_mul(10u64.pow(asset_info.decimals.into()))?
        .safe_div(asset_price as u128)? as u64;

    let fee_amount = withdraw_amount
        .safe_mul(asset_info.remove_liquidity_fee_rate as u64)?
        .safe_div(FEE_RATE_BASE)? as u64;

    msg!(
        "withdraw_amount:{},fee_amount:{},asset_price:{}",
        withdraw_amount,
        fee_amount,
        asset_price
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

        token::transfer(cpi_ctx, withdraw_amount - fee_amount)?;
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
    }

    asset_info.liquidity_amount -= withdraw_amount;

    Ok(())
}
