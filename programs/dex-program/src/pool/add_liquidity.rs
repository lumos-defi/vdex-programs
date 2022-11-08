use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::{
    dex::{get_oracle_price, Dex},
    errors::DexError,
    errors::DexResult,
    utils::{SafeMath, USDC_DECIMALS, USDC_POW_DECIMALS},
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

    //get pool asset sum
    let mut asset_sum = 0;
    let mut asset_offset = 0;
    let mut asset_price = 0;
    for asset_index in 0..dex.assets.len() {
        let asset_info = &dex.assets[asset_index];
        if !asset_info.valid {
            continue;
        }
        require_eq!(
            dex.assets[asset_index].oracle,
            *ctx.remaining_accounts[asset_offset].key,
            DexError::InvalidOracleAccount
        );

        let oracle_price = get_oracle_price(
            asset_info.oracle_source,
            &ctx.remaining_accounts[asset_offset],
        )?;

        if asset_info.mint == ctx.accounts.mint.key() {
            asset_price = oracle_price
        }

        asset_sum += (asset_info
            .liquidity_amount
            .safe_add(asset_info.collateral_amount)?)
        .safe_mul(oracle_price.into())?
        .safe_div(10u128.pow(asset_info.decimals.into()))? as u64;

        asset_offset += 1;
    }

    //get pool pnl
    let mut pnl = 0;
    let mut market_offset = asset_offset;
    for market_index in 0..dex.markets.len() {
        let market_info = &dex.markets[market_index];
        if !market_info.valid {
            continue;
        }
        require_eq!(
            dex.markets[market_index].oracle,
            *ctx.remaining_accounts[market_offset].key,
            DexError::InvalidOracleAccount
        );

        let oracle_price = get_oracle_price(
            market_info.oracle_source,
            &ctx.remaining_accounts[market_offset],
        )?;

        if market_info.global_long.size > 0 {
            pnl += -(market_info.global_long.pnl(
                market_info.global_long.size,
                oracle_price,
                market_info.global_long.average_price,
                market_info.decimals,
            )?);
        }

        if market_info.global_short.size > 0 {
            pnl += -(market_info.global_short.pnl(
                market_info.global_short.size,
                oracle_price,
                market_info.global_short.average_price,
                market_info.decimals,
            )?);
        }

        market_offset += 1;
    }

    let vlp_mint_nonce = dex.vlp_mint_nonce;
    let assets = &mut dex.assets;

    let asset = assets
        .iter()
        .position(|x| x.mint == *ctx.accounts.mint.key)
        .ok_or(DexError::InvalidMint)? as u8;
    let asset_info = &mut assets[asset as usize];

    let vlp_mint_info =
        Mint::try_deserialize(&mut &**ctx.accounts.vlp_mint.try_borrow_mut_data()?)?;

    if pnl > 0 {
        asset_sum += pnl as u64;
    } else {
        asset_sum -= pnl as u64;
    }

    let asset_in_usdc = amount
        .safe_mul(asset_price)?
        .safe_div(10u128.pow(asset_info.decimals.into()))? as u64;

    msg!(
        "asset_in_usdc:{},amount:{},decimals:{},price:{}",
        asset_in_usdc,
        amount,
        asset_info.decimals,
        asset_price
    );
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
