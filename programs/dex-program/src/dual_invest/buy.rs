use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

use crate::{
    dex::{get_price, Dex, PriceFeed},
    dual_invest::DI,
    errors::{DexError, DexResult},
    user::UserState,
    utils::{get_timestamp, swap, SafeMath, FEE_RATE_BASE},
};

#[derive(Accounts)]
pub struct DiBuy<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    /// CHECK
    pub base_asset_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub in_mint_vault: AccountInfo<'info>,

    #[account(
        mut,
        constraint = (user_mint_acc.owner == *authority.key)
    )]
    pub user_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,

    /// CHECK
    #[account(owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,
}

pub fn handler(ctx: Context<DiBuy>, id: u64, premium_rate: u16, size: u64) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    require!(
        dex.price_feed == ctx.accounts.price_feed.key(),
        DexError::InvalidPriceFeed
    );

    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );
    let di = DI::mount(&ctx.accounts.di_option, true)?;

    // Get option info
    let option = di.borrow().get_option(id)?;
    let base_ai = dex.asset_as_ref(option.base_asset_index)?;
    let quote_ai = dex.asset_as_ref(option.quote_asset_index)?;

    // Check size
    require!(
        size >= option.minimum_open_size && size <= option.maximum_open_size,
        DexError::DIInvalidSize
    );

    // Check premium
    require!(
        premium_rate == option.premium_rate,
        DexError::DIInvalidPremium
    );

    require!(!option.stopped, DexError::DIOptionNotFound);

    // Check date
    let now = get_timestamp()?;
    require!(now < option.expiry_date, DexError::DIOptionExpired);

    // Check account
    let ai = if option.is_call { base_ai } else { quote_ai };
    require!(
        ai.mint == ctx.accounts.user_mint_acc.mint,
        DexError::InvalidUserMintAccount
    );

    require!(
        base_ai.oracle == ctx.accounts.base_asset_oracle.key(),
        DexError::InvalidOracle
    );

    require!(
        ai.vault == ctx.accounts.in_mint_vault.key(),
        DexError::InvalidVault
    );

    let price_feed = &ctx.accounts.price_feed.load()?;
    // Check price
    // TODO: need a gap between strike price and market price ?
    let price = get_price(
        option.base_asset_index,
        base_ai.oracle_source,
        &ctx.accounts.base_asset_oracle,
        price_feed,
    )?;

    if option.is_call {
        require!(option.strike_price > price, DexError::InvalidStrikePrice);
    } else {
        require!(option.strike_price < price, DexError::InvalidStrikePrice);
    }

    // Calculate asset amount that needs to be borrowed and locked
    let borrow_base_funds = if option.is_call {
        // Call option and market price < strike price, option is not exercised
        size.safe_mul(premium_rate as u64)?
            .safe_div(FEE_RATE_BASE as u128)? as u64
    } else {
        // Put option and market price <= strike price, option is exercised
        let base = swap(
            size,
            10u64.pow(quote_ai.decimals as u32),
            quote_ai.decimals,
            option.strike_price,
            base_ai.decimals,
        )?;

        let premium = base
            .safe_mul(premium_rate as u64)?
            .safe_div(FEE_RATE_BASE as u128)? as u64;

        base + premium
    };

    // TODO: check quote asset price?
    let borrow_quote_funds = if option.is_call {
        // Call option and market price >= strike price, option is exercised
        let base = swap(
            size,
            option.strike_price,
            base_ai.decimals,
            10u64.pow(quote_ai.decimals as u32),
            quote_ai.decimals,
        )?;

        let premium = base
            .safe_mul(premium_rate as u64)?
            .safe_div(FEE_RATE_BASE as u128)? as u64;

        base + premium
    } else {
        // Put option and market price > strike price, option is not exercised
        size.safe_mul(premium_rate as u64)?
            .safe_div(FEE_RATE_BASE as u128)? as u64
    };

    // Transfer the user collateral
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_mint_acc.to_account_info(),
        to: ctx.accounts.in_mint_vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts);
    token::transfer(cpi_ctx, size)?;

    // Borrow funds
    dex.di_option_borrow(option.base_asset_index, borrow_base_funds)?;
    dex.di_option_borrow(option.quote_asset_index, borrow_quote_funds)?;

    // Create the option
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    us.borrow_mut()
        .di_new_option(&option, size, borrow_base_funds, borrow_quote_funds)?;

    di.borrow_mut().add_volume(id, size)?;

    Ok(())
}
