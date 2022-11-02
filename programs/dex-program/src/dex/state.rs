use anchor_lang::prelude::*;
use num_enum::TryFromPrimitive;

use crate::{
    errors::DexResult,
    utils::{time::get_timestamp, ISafeAddSub, ISafeMath, SafeMath, FEE_RATE_MAX},
};

#[account(zero_copy)]
pub struct Dex {
    pub magic: u64,
    pub assets: [AssetInfo; 16],
    pub markets: [MarketInfo; 16],
    pub authority: Pubkey,
    pub event_queue: Pubkey,
    pub match_queue: Pubkey,
    pub vlp_mint: Pubkey,
    pub vlp_mint_authority: Pubkey,
    pub user_list_entry_page: Pubkey,
    pub user_list_remaining_pages: [Pubkey; 8],
    pub user_list_remaining_pages_number: u8,
    pub assets_number: u8,
    pub markets_number: u8,
    pub vlp_mint_nonce: u8,
    pub padding: [u8; 252],
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
    pub oracle_source: u8,
    pub asset_index: u8,
    pub significant_decimals: u8,
    pub padding: [u8; 253],
}

pub struct MarketFeeRates {
    pub charge_borrow_fee_interval: u64,
    pub borrow_fee_rate: u16,
    pub open_fee_rate: u16,
    pub close_fee_rate: u16,
    pub base_decimals: u8,
}

#[zero_copy]
#[derive(Default)]
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
    pub fn new(long: bool) -> DexResult<Self> {
        let mut p = Position::default();
        p.zero(long)?;

        Ok(p)
    }

    pub fn zero(&mut self, long: bool) -> DexResult {
        self.size = 0;
        self.collateral = 0;
        self.average_price = 0;
        self.closing_size = 0;
        self.borrowed_amount = 0;
        self.last_fill_time = get_timestamp()?;
        self.cumulative_fund_fee = 0;
        self.loss_stop_price = 0;
        self.profit_stop_price = 0;
        self.long = long;

        Ok(())
    }

    pub fn open(
        &mut self,
        size: u64,
        price: u64,
        collateral: u64,
        mfr: &MarketFeeRates,
    ) -> DexResult<(u64, u64)> {
        // Update cumulative fund fee
        let now = get_timestamp()?;
        let cumulative_fund_fee = if self.borrowed_amount > 0 {
            // TODO: check now is gte last_fill_time
            self.borrowed_amount
                .safe_mul(mfr.borrow_fee_rate as u64)?
                .safe_mul((now - self.last_fill_time) as u128)?
                .safe_div(FEE_RATE_MAX)?
                .safe_div(mfr.charge_borrow_fee_interval as u128)? as u64
                + self.cumulative_fund_fee
        } else {
            0
        };

        // Update borrowed amount
        let borrow = if self.long {
            Ok(size as u128)
        } else {
            price.safe_mul(size)
        }? as u64;

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

        // Calculate open position fee
        let fee = if self.long {
            size.safe_mul(mfr.open_fee_rate as u64)?
                .safe_div(FEE_RATE_MAX)? as u64
        } else {
            let quote_amount =
                size.safe_mul(price)?
                    .safe_div(10u64.pow(mfr.base_decimals.into()) as u128)? as u64;
            quote_amount
                .safe_mul(mfr.open_fee_rate as u64)?
                .safe_div(FEE_RATE_MAX)? as u64
        };

        Ok((borrow, fee))
    }

    pub fn close(
        &mut self,
        size: u64,
        price: u64,
        mfr: &MarketFeeRates,
    ) -> DexResult<(u64, u64, i64, u64)> {
        let mut collateral_unlocked = size
            .safe_mul(self.collateral)?
            .safe_div(self.size as u128)? as u64;

        let mut fund_returned = size
            .safe_mul(self.borrowed_amount)?
            .safe_div(self.size as u128)? as u64;

        // Update cumulative fund fee
        let now = get_timestamp()?;
        let fund_fee = if self.borrowed_amount > 0 {
            // TODO: check now is gte last_fill_time
            self.borrowed_amount
                .safe_mul(mfr.borrow_fee_rate as u64)?
                .safe_mul((now - self.last_fill_time) as u128)?
                .safe_div(FEE_RATE_MAX)?
                .safe_div(mfr.charge_borrow_fee_interval as u128)? as u64
                + self.cumulative_fund_fee
        } else {
            0
        };

        // Calculate close position fee
        let fee = if self.long {
            size.safe_mul(mfr.close_fee_rate as u64)?
                .safe_div(FEE_RATE_MAX)? as u64
        } else {
            let quote_amount =
                size.safe_mul(price)?
                    .safe_div(10u64.pow(mfr.base_decimals.into()) as u128)? as u64;
            quote_amount
                .safe_mul(mfr.close_fee_rate as u64)?
                .safe_div(FEE_RATE_MAX)? as u64
        } + fund_fee;

        let pnl = self.pnl(size, price, self.average_price, mfr.base_decimals)?;
        let pnl_with_fee = pnl.i_safe_sub(fee as i64)?;

        // Update the position
        self.borrowed_amount = self.borrowed_amount.safe_sub(fund_returned)?;
        self.collateral = self.collateral.safe_sub(collateral_unlocked)?;
        self.size = self.size.safe_sub(size)?;
        self.cumulative_fund_fee = 0;
        self.last_fill_time = now;

        // CHECK:
        // If (pnl - fee) < 0, check if the unlocked collateral covers loss + fee
        let user_balance = (collateral_unlocked as i64).i_safe_add(pnl_with_fee)?;
        if user_balance < 0 {
            let abs_user_balance = i64::abs(user_balance) as u64;

            if abs_user_balance < self.collateral {
                self.collateral = self.collateral.safe_sub(abs_user_balance)?;
                collateral_unlocked = collateral_unlocked.safe_add(abs_user_balance)?;
            } else {
                self.collateral = 0;
                fund_returned = fund_returned.safe_add(self.borrowed_amount)?;
                collateral_unlocked = collateral_unlocked.safe_add(self.collateral)?;
            }
        }

        if self.size == 0 || self.collateral == 0 {
            self.zero(self.long)?;
        }

        Ok((fund_returned, collateral_unlocked, pnl, fee))
    }

    fn pnl(
        &self,
        size: u64,
        close_price: u64,
        open_price: u64,
        base_decimals: u8,
    ) -> DexResult<i64> {
        let pnl = if self.long {
            (close_price as i128 - open_price as i128)
                .i_safe_mul(size as i128)?
                .i_safe_div(open_price as i128)? as i64
        } else {
            (open_price as i128 - close_price as i128)
                .i_safe_mul(size as i128)?
                .i_safe_div(10i128.pow(base_decimals as u32))? as i64
        };

        Ok(pnl)
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
