use crate::{
    collections::EventQueue,
    dex::{
        event::{AppendEvent, PositionAct},
        get_price, Dex, PriceFeed,
    },
    errors::{DexError, DexResult},
    user::state::*,
    utils::{value, SafeMath, LEVERAGE_POW_DECIMALS},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub in_mint: AccountInfo<'info>,

    /// CHECK
    pub in_mint_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub in_mint_vault: AccountInfo<'info>,

    /// CHECK
    pub market_mint: AccountInfo<'info>,

    /// CHECK
    pub market_mint_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub market_mint_vault: AccountInfo<'info>,

    /// CHECK
    pub market_oracle: AccountInfo<'info>,

    #[account(
        mut,
        constraint = (user_mint_acc.owner == *authority.key && user_mint_acc.mint == *in_mint.key)
    )]
    pub user_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,

    /// CHECK
    #[account(owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,
}

pub fn handler(
    ctx: Context<OpenPosition>,
    market: u8,
    long: bool,
    amount: u64,
    leverage: u32,
) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    require!(
        dex.price_feed == ctx.accounts.price_feed.key(),
        DexError::InvalidPriceFeed
    );

    require!(market < dex.markets_number, DexError::InvalidMarketIndex);

    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidEventQueue
    );

    // Read market info
    let mi = &dex.markets[market as usize];
    require!(
        mi.valid && mi.oracle == ctx.accounts.market_oracle.key(),
        DexError::InvalidMarketIndex
    );

    require!(
        leverage >= LEVERAGE_POW_DECIMALS && leverage <= mi.max_leverage,
        DexError::InvalidLeverage
    );

    // Read market asset info
    let (market_asset_index, mai) = if long {
        (mi.asset_index, &dex.assets[mi.asset_index as usize])
    } else {
        (
            dex.usdc_asset_index,
            &dex.assets[dex.usdc_asset_index as usize],
        )
    };

    require!(
        mai.valid
            && mai.mint == ctx.accounts.market_mint.key()
            && mai.oracle == ctx.accounts.market_mint_oracle.key()
            && mai.vault == ctx.accounts.market_mint_vault.key(),
        DexError::InvalidMarketIndex
    );

    let price_feed = &ctx.accounts.price_feed.load()?;
    // Get market price
    let price = get_price(
        mi.asset_index,
        mi.oracle_source,
        &ctx.accounts.market_oracle,
        price_feed,
    )?;
    let market_mint_price = get_price(
        market_asset_index,
        mai.oracle_source,
        &ctx.accounts.market_mint_oracle,
        price_feed,
    )?;
    let market_mint_decimals = mai.decimals;
    let minimum_collateral = mi.minimum_collateral;

    let mfr = mi.get_fee_rates(mai.borrow_fee_rate);

    // Read user input asset info
    let (input_asset_index, ai) = dex.find_asset_by_mint(ctx.accounts.in_mint.key())?;

    let actual_amount = if ai.mint == mai.mint {
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_mint_acc.to_account_info(),
            to: ctx.accounts.market_mint_vault.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        require_neq!(amount, 0u64, DexError::InvalidAmount);

        amount
    } else {
        // Transfer the asset first
        require!(
            ai.valid
                && ai.oracle == ctx.accounts.in_mint_oracle.key()
                && ai.vault == ctx.accounts.in_mint_vault.key(),
            DexError::InvalidMint
        );

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_mint_acc.to_account_info(),
            to: ctx.accounts.in_mint_vault.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Swap input asset to market required mint
        let oracles = &vec![
            &ctx.accounts.in_mint_oracle,
            &ctx.accounts.market_mint_oracle,
        ];
        let (out, fee) = dex.swap(
            input_asset_index,
            market_asset_index,
            amount,
            true,
            &oracles,
            price_feed,
        )?;

        dex.swap_in(input_asset_index, amount.safe_sub(fee)?, fee)?;
        dex.swap_out(market_asset_index, out)?;

        out
    };

    // User open position
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let (size, collateral, borrow, open_fee) =
        us.borrow_mut()
            .open_position(market, price, actual_amount, long, leverage, &mfr)?;

    require!(
        value(collateral, market_mint_price, market_mint_decimals)? >= minimum_collateral,
        DexError::CollateralTooSmall
    );

    // Update asset info (collateral amount, borrow amount, fee)
    dex.borrow_fund(market, long, collateral, borrow, open_fee)?;

    // Update market global position & volume
    dex.increase_global_position(market, long, price, size, collateral)?;
    dex.increase_volume(market, price, size)?;

    // Save to event queue
    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    let user_state_key = ctx.accounts.user_state.key().to_bytes();
    event_queue.fill_position(
        user_state_key,
        us.borrow().get_position_status(),
        market,
        PositionAct::Open,
        long,
        price,
        size,
        collateral,
        borrow,
        open_fee,
        0,
        0,
    )?;

    Ok(())
}
