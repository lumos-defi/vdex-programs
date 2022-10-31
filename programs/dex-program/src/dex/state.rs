use anchor_lang::prelude::*;
use num_enum::TryFromPrimitive;

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

    pub open_fee_rate: u16,
    pub close_fee_rate: u16,
    pub valid: bool,
    pub decimals: u8,
    pub nonce: u8,
    pub oracle_source: u8,
    pub asset_index: u8,
    pub significant_decimals: u8,
    pub padding: [u8; 254],
}

#[zero_copy]
pub struct Position {
    pub size: u64,
    pub collateral: u64,
    pub average_price: u64,
    pub closing_size: u64,
    pub borrowed_amount: u64,
    pub last_fill_time: u64,
    pub cumulative_fund_fee: u64,
    pub loss_stop_price: u64,
    pub profit_stop_price: u64,
    pub long_or_short: u8,
    pub market: u8,
    pub _padding: [u8; 6],
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
