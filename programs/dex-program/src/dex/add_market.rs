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
    open_fee_rate: u16,
    close_fee_rate: u16,
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

    let long_or_short_position = Position {
        size: 0,
        collateral: 0,
        average_price: 0,
        closing_size: 0,
        borrowed_amount: 0,
        last_fill_time: 0,
        cumulative_fund_fee: 0,
        loss_stop_price: 0,
        profit_stop_price: 0,
        long_or_short: 0,
        market: 0,
        _padding: [0; 6],
    };

    let market = MarketInfo {
        symbol: market_symbol,
        oracle: *ctx.accounts.oracle.key,
        long_order_book: *ctx.accounts.long_order_book.key,
        short_order_book: *ctx.accounts.short_order_book.key,
        order_pool_entry_page: *ctx.accounts.order_pool_entry_page.key,
        order_pool_remaining_pages: [Pubkey::default(); 16],
        global_long: long_or_short_position,
        global_short: long_or_short_position,
        open_fee_rate,
        close_fee_rate,
        valid: true,
        decimals,
        nonce: 0,
        oracle_source,
        asset_index,
        significant_decimals,
        padding: [0; 254],
    };

    dex.markets[market_index] = market;
    dex.markets_number += 1;

    Ok(())
}
