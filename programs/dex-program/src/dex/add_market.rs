use std::convert::TryFrom;

use anchor_lang::prelude::*;

use crate::{
    collections::{MountMode, OrderBook, PagedList},
    dex::{MarketInfo, OracleSource, Position},
    errors::{DexError, DexResult},
    order::Order,
    utils::ORDER_POOL_MAGIC_BYTE,
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
    #[account(mut, constraint= order_book.owner == program_id)]
    pub order_book: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= order_pool_entry_page.owner == program_id)]
    pub order_pool_entry_page: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

#[allow(clippy::too_many_arguments)]
pub fn handler(
    ctx: Context<AddMarket>,
    symbol: String,
    minimum_open_amount: u64,
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

    let order_book = &mut ctx.accounts.order_book;
    let order_pool_entry_page = &mut ctx.accounts.order_pool_entry_page;

    let dex = &mut ctx.accounts.dex.load_mut()?;

    let mut market_symbol: [u8; 16] = Default::default();
    let given_name = symbol.as_bytes();
    let markets = &dex.markets;

    market_symbol[..given_name.len()].copy_from_slice(given_name);
    if markets.iter().any(|market| market.symbol == market_symbol) {
        return Err(error!(DexError::DuplicateMarketName));
    }

    OrderBook::mount(order_book, false)?.initialize()?;
    PagedList::<Order>::mount(
        order_pool_entry_page,
        &[],
        ORDER_POOL_MAGIC_BYTE,
        MountMode::Initialize,
    )
    .map_err(|_| DexError::FailedInitOrderPool)?;

    let market_index = dex.markets_number as usize;
    if market_index == dex.markets.len() {
        return Err(error!(DexError::InsufficientMarketIndex));
    }

    let market = MarketInfo {
        symbol: market_symbol,
        oracle: ctx.accounts.oracle.key(),
        order_book: ctx.accounts.order_book.key(),
        order_pool_entry_page: ctx.accounts.order_pool_entry_page.key(),
        order_pool_remaining_pages: [Pubkey::default(); 16],
        global_long: Position::new(true)?,
        global_short: Position::new(false)?,
        minimum_position_value: minimum_open_amount,
        charge_borrow_fee_interval,
        open_fee_rate,
        close_fee_rate,
        liquidate_fee_rate,
        valid: true,
        decimals,
        oracle_source,
        asset_index,
        significant_decimals,
        order_pool_remaining_pages_number: 0,
        padding: [0; 252],
    };

    dex.markets[market_index] = market;
    dex.markets_number += 1;

    Ok(())
}
