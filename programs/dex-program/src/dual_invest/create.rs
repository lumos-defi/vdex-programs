use anchor_lang::prelude::*;

use crate::{
    dex::{get_oracle_price, Dex},
    dual_invest::DI,
    errors::{DexError, DexResult},
    utils::get_timestamp,
};

#[derive(Accounts)]
pub struct DICreateOption<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    /// CHECK
    pub base_asset_oracle: AccountInfo<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<DICreateOption>,
    is_call: bool,
    base_asset_index: u8,
    quote_asset_index: u8,
    premium_rate: u16,
    expiry_date: i64,
    strike_price: u64,
    minimum_open_size: u64,
) -> DexResult {
    let dex = &mut ctx.accounts.dex.load()?;
    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );

    let di = DI::mount(&ctx.accounts.di_option, true)?;
    require!(
        di.borrow().meta.admin == ctx.accounts.authority.key(),
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

    // Check strike price
    let price = get_oracle_price(base_ai.oracle_source, &ctx.accounts.base_asset_oracle)?;
    // TODO: need a gap between strike price and market price ?
    if is_call {
        require!(strike_price > price, DexError::InvalidStrikePrice);
    } else {
        require!(strike_price < price, DexError::InvalidStrikePrice);
    }

    // Check expiry date
    // TODO: need a gap between expiry date and now?
    let now = get_timestamp()?;
    require!(now < expiry_date, DexError::InvalidExpiryDate);

    di.borrow_mut().create(
        is_call,
        base_asset_index,
        quote_asset_index,
        premium_rate,
        expiry_date,
        strike_price,
        minimum_open_size,
    )?;

    Ok(())
}
