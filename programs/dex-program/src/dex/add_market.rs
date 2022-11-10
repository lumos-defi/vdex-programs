use std::convert::TryFrom;

use anchor_lang::prelude::*;

use crate::{
    dex::{MarketInfo, OracleSource, Position},
    errors::{DexError, DexResult},
};

use super::Dex;

#[derive(Accounts)]
pub struct AddMarket<'info> {
    #[account(
        mut,
        has_one = authority,
    )]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub oracle: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= long_order_book.owner == program_id)]
    pub long_order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= short_order_book.owner == program_id)]
    pub short_order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_pool_entry_page.owner == program_id)]
    pub order_pool_entry_page: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

#[allow(clippy::too_many_arguments)]
pub fn handler(
    ctx: Context<AddMarket>,
    symbol: String,
    minimum_position_value: u64,
    charge_borrow_fee_interval: u64,
    open_fee_rate: u16,
    close_fee_rate: u16,
    liquidate_fee_rate: u16,
    decimals: u8,
    oracle_source: u8,
    asset_index: u8,
    significant_decimals: u8,
) -> DexResult {
    require!(
        significant_decimals < decimals,
        DexError::InvalidSignificantDecimals
    );

    OracleSource::try_from(oracle_source).map_err(|_| DexError::InvalidOracleSource)?;

    //todo mount order book
    let _long_order_book = &mut ctx.accounts.long_order_book;
    let _order_pool_entry_page = &mut ctx.accounts.order_pool_entry_page;

    let dex = &mut ctx.accounts.dex.load_mut()?;

    let mut market_symbol: [u8; 16] = Default::default();
    let given_name = symbol.as_bytes();
    let markets = &dex.markets;

    market_symbol[..given_name.len()].copy_from_slice(given_name);
    if markets.iter().any(|market| market.symbol == market_symbol) {
        return Err(error!(DexError::DuplicateMarketName));
    }

    let market_index = dex.markets_number as usize;
    if market_index == dex.markets.len() {
        return Err(error!(DexError::InsufficientMarketIndex));
    }

    let market = MarketInfo {
        symbol: market_symbol,
        oracle: *ctx.accounts.oracle.key,
        long_order_book: *ctx.accounts.long_order_book.key,
        short_order_book: *ctx.accounts.short_order_book.key,
        order_pool_entry_page: *ctx.accounts.order_pool_entry_page.key,
        order_pool_remaining_pages: [Pubkey::default(); 16],
        global_long: Position::new(true)?,
        global_short: Position::new(false)?,
        minimum_position_value,
        charge_borrow_fee_interval,
        open_fee_rate,
        close_fee_rate,
        liquidate_fee_rate,
        valid: true,
        decimals,
        oracle_source,
        asset_index,
        significant_decimals,
        padding: [0; 253],
    };

    dex.markets[market_index] = market;
    dex.markets_number += 1;

    Ok(())
}
