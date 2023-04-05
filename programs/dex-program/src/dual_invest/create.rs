use anchor_lang::prelude::*;

use crate::{
    dex::{get_price, Dex, PriceFeed},
    dual_invest::DI,
    errors::{DexError, DexResult},
    utils::get_timestamp,
};

#[derive(Accounts)]
pub struct DiCreateOption<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    /// CHECK
    pub base_asset_oracle: AccountInfo<'info>,

    pub authority: Signer<'info>,

    /// CHECK
    #[account(owner = *program_id)]
    pub price_feed: AccountLoader<'info, PriceFeed>,
}

pub fn handler(
    ctx: Context<DiCreateOption>,
    id: u64,
    is_call: bool,
    base_asset_index: u8,
    quote_asset_index: u8,
    premium_rate: u16,
    expiry_date: i64,
    strike_price: u64,
    minimum_open_size: u64,
    maximum_open_size: u64,
    stop_before_expiry: u64,
) -> DexResult {
    let dex = &mut ctx.accounts.dex.load()?;
    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );

    require!(
        dex.price_feed == ctx.accounts.price_feed.key(),
        DexError::InvalidPriceFeed
    );

    let di = DI::mount(&ctx.accounts.di_option, true)?;
    require!(
        di.borrow().meta.admin == ctx.accounts.authority.key()
            || dex.authority == ctx.accounts.authority.key(),
        DexError::InvalidDIAdmin
    );

    // Check base & quote asset index
    require!(
        quote_asset_index < dex.assets_number && dex.assets[quote_asset_index as usize].valid,
        DexError::InvalidAssetIndex
    );

    let base_ai = dex.asset_as_ref(base_asset_index)?;
    require!(
        base_ai.oracle == ctx.accounts.base_asset_oracle.key(),
        DexError::InvalidOracle
    );

    let price_feed = &ctx.accounts.price_feed.load()?;
    // Check strike price
    let price = get_price(
        base_asset_index,
        base_ai.oracle_source,
        &ctx.accounts.base_asset_oracle,
        price_feed,
    )?;
    // TODO: need a gap between strike price and market price ?
    if is_call {
        require!(strike_price > price, DexError::InvalidStrikePrice);
    } else {
        require!(strike_price < price, DexError::InvalidStrikePrice);
    }

    require!(premium_rate > 0, DexError::ZeroPremiumRate);

    // Check expiry date
    // TODO: need a gap between expiry date and now?
    let now = get_timestamp()?;
    require!(now < expiry_date, DexError::InvalidExpiryDate);

    di.borrow_mut().create(
        id,
        is_call,
        base_asset_index,
        quote_asset_index,
        premium_rate,
        expiry_date,
        strike_price,
        minimum_open_size,
        maximum_open_size,
        stop_before_expiry,
    )?;

    Ok(())
}
