use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::{
    dex::{get_oracle_price, Dex},
    errors::DexError,
    errors::DexResult,
    pool::get_asset_aum,
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

// Remaining accounts layout:
// dex.assets.map({
//   asset index price oracle account
// })
// dex.markets.map({
//    market index price oracle account
// })
pub fn handler(ctx: Context<AddLiquidity>, amount: u64) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    require_eq!(
        dex.vlp_mint,
        ctx.accounts.vlp_mint.key(),
        DexError::InvalidVlpMint
    );

    require_eq!(
        dex.vlp_mint_authority,
        ctx.accounts.vlp_mint_authority.key(),
        DexError::InvalidVlpMintAuthority
    );

    let oracle_accounts_len = dex.assets.iter().filter(|a| a.valid).count()
        + dex.markets.iter().filter(|m| m.valid).count();

    require_eq!(
        oracle_accounts_len,
        ctx.remaining_accounts.len(),
        DexError::InvalidRemainingAccounts
    );

    let asset_sum = get_asset_aum(&dex, &ctx.remaining_accounts)?;

    let vlp_mint_nonce = dex.vlp_mint_nonce;
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

    let oracle_account = ctx
        .remaining_accounts
        .iter()
        .find(|a| a.key() == asset_info.oracle)
        .ok_or(DexError::InvalidOracleAccount)?;

    let asset_price = get_oracle_price(asset_info.oracle_source, oracle_account)?;

    let fee_amount = amount
        .safe_mul(asset_info.add_liquidity_fee_rate as u64)?
        .safe_div(FEE_RATE_BASE)? as u64;

    let asset_in_usdc = amount
        .safe_sub(fee_amount)?
        .safe_mul(asset_price)?
        .safe_div(10u128.pow(asset_info.decimals.into()))? as u64;

    msg!(
        "asset_in_usdc:{},amount:{},fee_amount:{},decimals:{},price:{}",
        asset_in_usdc,
        amount,
        fee_amount,
        asset_info.decimals,
        asset_price
    );

    let vlp_mint_info =
        Mint::try_deserialize(&mut &**ctx.accounts.vlp_mint.try_borrow_mut_data()?)?;

    // mintAmount = asset_in_usdc * glp_supply / assets_sum
    let mint_amount = if asset_sum == 0 {
        asset_in_usdc
            .safe_mul(10u64.pow(vlp_mint_info.decimals.into()))?
            .safe_div(USDC_POW_DECIMALS as u128)? as u64
    } else {
        asset_in_usdc
            .safe_mul(vlp_mint_info.supply)?
            .safe_div(10u128.pow(vlp_mint_info.decimals.into()))?
            .safe_div(asset_sum as u128)?
            .safe_mul(10u128.pow(vlp_mint_info.decimals.into()))?
            .safe_div(USDC_POW_DECIMALS as u128)? as u64
    };

    //add liquidity
    {
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
    }

    //mint vlp
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
            authority: ctx.accounts.vlp_mint_authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );

        token::mint_to(cpi_ctx, mint_amount)?;
    }

    asset_info.liquidity_amount += amount;

    Ok(())
}
