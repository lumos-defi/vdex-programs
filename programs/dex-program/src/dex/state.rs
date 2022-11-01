use anchor_lang::prelude::*;
use num_enum::TryFromPrimitive;

use crate::{
    user::DexResult,
    utils::{time::get_timestamp, SafeMath, FEE_RATE_MAX},
};

#[account(zero_copy)]
pub struct Dex {
    pub magic: u64,
    pub assets: [AssetInfo; 16],
    pub markets: [MarketInfo; 16],
    pub authority: Pubkey,
    pub event_queue: Pubkey,
    pub match_queue: Pubkey,
    pub user_list_entry_page: Pubkey,
    pub user_list_remaining_pages: [Pubkey; 8],
    pub user_list_remaining_pages_number: u8,
    pub assets_number: u8,
    pub markets_number: u8,
    pub padding: [u8; 253],
}

#[zero_copy]
pub struct AssetInfo {
    pub symbol: [u8; 16],
    pub mint: Pubkey,
    pub oracle: Pubkey,
    pub vault: Pubkey,
    pub program_signer: Pubkey,
    pub liquidity_amount: u64,
    pub collateral_amount: u64,
    pub borrowed_amount: u64,
    pub borrowed_fee_rate: u16,
    pub add_liquidity_fee_rate: u16,
    pub remove_liquidity_fee_rate: u16,
    pub target_weight: u16,
    pub valid: bool,
    pub decimals: u8,
    pub nonce: u8,
    pub oracle_source: u8,
    pub padding: [u8; 252],
}

#[zero_copy]
pub struct MarketInfo {
    pub symbol: [u8; 16],
    pub oracle: Pubkey,

    pub long_order_book: Pubkey,
    pub short_order_book: Pubkey,

    pub order_pool_entry_page: Pubkey,
    pub order_pool_remaining_pages: [Pubkey; 16],

    pub global_long: Position,
    pub global_short: Position,

    pub charge_borrow_fee_interval: u64,
    pub open_fee_rate: u16,
    pub close_fee_rate: u16,
    pub borrow_fee_rate: u16,
    pub valid: bool,
    pub decimals: u8,
    pub nonce: u8,
    pub oracle_source: u8,
    pub asset_index: u8,
    pub significant_decimals: u8,
    pub padding: [u8; 252],
}

pub struct MarketFeeRates {
    pub charge_borrow_fee_interval: u64,
    pub open_fee_rate: u16,
    pub close_fee_rate: u16,
    pub borrow_fee_rate: u16,
}

#[zero_copy]
pub struct Position {
    pub size: u64,
    pub collateral: u64,
    pub average_price: u64,
    pub closing_size: u64,
    pub borrowed_amount: u64,
    pub last_fill_time: i64,
    pub cumulative_fund_fee: u64,
    pub loss_stop_price: u64,
    pub profit_stop_price: u64,
    pub long: bool,
    pub _padding: [u8; 7],
}

impl Position {
    pub fn zero(&mut self, long: bool) {
        self.size = 0;
        self.collateral = 0;
        self.average_price = 0;
        self.closing_size = 0;
        self.borrowed_amount = 0;
        self.last_fill_time = 0;
        self.cumulative_fund_fee = 0;
        self.loss_stop_price = 0;
        self.profit_stop_price = 0;
        self.long = long;
    }

    pub fn open(
        &mut self,
        size: u64,
        price: u64,
        collateral: u64,
        ms: &MarketFeeRates,
    ) -> DexResult<(u64, u64)> {
        // Update cumulative fund fee
        let now = get_timestamp()?;
        let cumulative_fund_fee = if self.borrowed_amount > 0 {
            // TODO: check now is gte last_fill_time
            self.borrowed_amount
                .safe_mul(ms.borrow_fee_rate as u64)?
                .safe_mul((now - self.last_fill_time) as u128)?
                .safe_div(FEE_RATE_MAX)?
                .safe_div(ms.charge_borrow_fee_interval as u128)? as u64
        } else {
            0
        };

        // Update borrowed amount
        let borrow = if self.long {
            Ok(size as u128)
        } else {
            price.safe_mul(size)
        }? as u64;

        // Calculate fee
        let fee = collateral
            .safe_mul(ms.borrow_fee_rate as u64)?
            .safe_div(FEE_RATE_MAX)? as u64;

        let merged_size = self.size.safe_add(size)?;

        let average_price = self
            .average_price
            .safe_mul(self.size)?
            .safe_add(price.safe_mul(size)?)?
            .safe_div(merged_size as u128)? as u64;

        self.average_price = average_price;
        self.size = merged_size;
        self.collateral = self.collateral.safe_add(collateral)?;
        self.borrowed_amount = self.borrowed_amount.safe_add(borrow)?;
        self.cumulative_fund_fee = cumulative_fund_fee;
        self.last_fill_time = now;

        Ok((borrow, fee))
    }
}

#[zero_copy]
pub struct Order {
    pub size: u64,
    pub filled_size: u64,
    pub collateral: u64,
    pub limit_price: u64,
    pub list_time: u64,
    pub loss_stop_price: u64,
    pub profit_stop_price: u64,
    pub long_or_short: u8,
    pub open_or_close: u8,
    pub market: u8,
    pub position_index: u8,
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum OracleSource {
    Mock = 0,
    Pyth = 1,
    StableCoin = 2,
}

#[account]
#[repr(C)]
pub struct MockOracle {
    pub magic: u64,
    pub price: u64,
    pub expo: u8,
    pub padding1: [u8; 7],
}
pub struct UserListItem {
    pub user_state: [u8; 32],
    pub serial_number: u32,
}

impl UserListItem {
    pub fn init_serial_number(&mut self, user_state: [u8; 32], serial_number: u32) {
        self.user_state = user_state;
        self.serial_number = serial_number;
    }

    pub fn update_serial_number(&mut self, serial_number: u32) {
        self.serial_number = serial_number;
    }

    pub fn serial_number(&self) -> u32 {
        self.serial_number
    }
}

pub struct MatchEvent {
    pub user_state: [u8; 32],
    pub price: u64,
    pub fill_size: u64,
    pub taker_pnl: i64,
    pub taker_fee: i64,
    pub order_slot: u32,
    pub user_order_slot: u8,
    pub open_or_close: u8,
    pub long_or_short: u8,
    _padding: [u8; 1],
}
