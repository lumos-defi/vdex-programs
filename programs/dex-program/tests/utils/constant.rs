#![allow(dead_code)]

const BORROW_FEE_RATE: u16 = 10; //1-10_000  0.1%
const ADD_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
const REMOVE_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
const SWAP_FEE_RATE: u16 = 10;

const OPEN_FEE_RATE: u16 = 30; // 0.3% (30 / 10000)
const CLOSE_FEE_RATE: u16 = 50; // 0.5%   (50 /  10000)
const LIQUIDATE_FEE_RATE: u16 = 80; // 0.8%   (80 /  10000)

pub const MAX_LEVERAGE: u32 = 30_000; // 30 (30_000 / 1_000);
pub const MAX_ASSET_COUNT: usize = 16;
pub const PRICE_FEED_DECIMALS: u8 = 6;

pub const SECOND: i64 = 1;
pub const DAY: i64 = 3600 * 24;

//USDC ASSET
pub const TEST_USDC_SYMBOL: &str = "USDC";
pub const TEST_USDC_DECIMALS: u8 = 6;
pub const TEST_USDC_ORACLE_PRICE: f64 = 1.0;
pub const TEST_USDC_ORACLE_EXPO: u8 = 5;

pub const TEST_USDC_BORROW_FEE_RATE: u16 = BORROW_FEE_RATE; //1-10_000  0.1%
pub const TEST_USDC_ADD_LIQUIDITY_FEE_RATE: u16 = ADD_LIQUIDITY_FEE_RATE; //0.1%
pub const TEST_USDC_REMOVE_LIQUIDITY_FEE_RATE: u16 = REMOVE_LIQUIDITY_FEE_RATE; //0.1%
pub const TEST_USDC_SWAP_FEE_RATE: u16 = SWAP_FEE_RATE;
pub const TEST_USDC_TARGET_WEIGHT: u16 = 400; //1-1000 //40%

//BTC asset
pub const TEST_BTC_SYMBOL: &str = "BTC";
pub const TEST_BTC_DECIMALS: u8 = 9;
pub const TEST_BTC_ORACLE_PRICE: f64 = 20_000.0;
pub const TEST_BTC_ORACLE_EXPO: u8 = 8;

pub const TEST_BTC_BORROW_FEE_RATE: u16 = BORROW_FEE_RATE; //1-10_000  0.1%
pub const TEST_BTC_ADD_LIQUIDITY_FEE_RATE: u16 = ADD_LIQUIDITY_FEE_RATE; //0.1%
pub const TEST_BTC_REMOVE_LIQUIDITY_FEE_RATE: u16 = REMOVE_LIQUIDITY_FEE_RATE; //0.1%
pub const TEST_BTC_SWAP_FEE_RATE: u16 = SWAP_FEE_RATE;
pub const TEST_BTC_TARGET_WEIGHT: u16 = 300; //1-1000 //30%

//ETH asset
pub const TEST_ETH_SYMBOL: &str = "ETH";
pub const TEST_ETH_DECIMALS: u8 = 6;
pub const TEST_ETH_ORACLE_PRICE: f64 = 2_000.0;
pub const TEST_ETH_ORACLE_EXPO: u8 = 8;

pub const TEST_ETH_BORROW_FEE_RATE: u16 = BORROW_FEE_RATE; //1-10_000  0.1%
pub const TEST_ETH_ADD_LIQUIDITY_FEE_RATE: u16 = ADD_LIQUIDITY_FEE_RATE; //0.1%
pub const TEST_ETH_REMOVE_LIQUIDITY_FEE_RATE: u16 = REMOVE_LIQUIDITY_FEE_RATE; //0.1%
pub const TEST_ETH_SWAP_FEE_RATE: u16 = SWAP_FEE_RATE;
pub const TEST_ETH_TARGET_WEIGHT: u16 = 200; //1-1000 //20%

//SOL asset
pub const TEST_SOL_SYMBOL: &str = "SOL";
pub const TEST_SOL_DECIMALS: u8 = 9;
pub const TEST_SOL_ORACLE_PRICE: f64 = 2_00.0;
pub const TEST_SOL_ORACLE_EXPO: u8 = 8;

pub const TEST_SOL_BORROW_FEE_RATE: u16 = BORROW_FEE_RATE; //1-10_000  0.1%
pub const TEST_SOL_ADD_LIQUIDITY_FEE_RATE: u16 = ADD_LIQUIDITY_FEE_RATE; //0.1%
pub const TEST_SOL_REMOVE_LIQUIDITY_FEE_RATE: u16 = REMOVE_LIQUIDITY_FEE_RATE; //0.1%
pub const TEST_SOL_SWAP_FEE_RATE: u16 = SWAP_FEE_RATE;
pub const TEST_SOL_TARGET_WEIGHT: u16 = 200; //1-1000 //20%

//BTC market
pub const TEST_BTC_MARKET_SYMBOL: &str = "BTC";
pub const TEST_BTC_MINIMUM_COLLATERAL: u64 = 25;
pub const TEST_BTC_CHARGE_BORROW_FEE_INTERVAL: u64 = 3600;
pub const TEST_BTC_OPEN_FEE_RATE: u16 = OPEN_FEE_RATE; // 0.3% (30 / 10000)
pub const TEST_BTC_CLOSE_FEE_RATE: u16 = CLOSE_FEE_RATE; // 0.5%   (50 /  10000)
pub const TEST_BTC_LIQUIDATE_FEE_RATE: u16 = LIQUIDATE_FEE_RATE; // 0.8%   (80 /  10000)
pub const TEST_BTC_MARKET_DECIMALS: u8 = 9;
pub const TEST_BTC_ORACLE_SOURCE: u8 = 0; // 0: mock,1: pyth
pub const TEST_BTC_ASSET_INDEX: u8 = 1; // 0:usdc, 1:btc, 2:eth, 3:sol
pub const TEST_BTC_SIGNIFICANT_DECIMALS: u8 = 2;

//ETH market
pub const TEST_ETH_MARKET_SYMBOL: &str = "ETH";
pub const TEST_ETH_MINIMUM_COLLATERAL: u64 = 25;
pub const TEST_ETH_CHARGE_BORROW_FEE_INTERVAL: u64 = 3600;
pub const TEST_ETH_OPEN_FEE_RATE: u16 = OPEN_FEE_RATE; // 0.3% (30 / 10000)
pub const TEST_ETH_CLOSE_FEE_RATE: u16 = CLOSE_FEE_RATE; // 0.5%   (50 /  10000)
pub const TEST_ETH_LIQUIDATE_FEE_RATE: u16 = LIQUIDATE_FEE_RATE; // 0.8%   (80 /  10000)
pub const TEST_ETH_MARKET_DECIMALS: u8 = 9;
pub const TEST_ETH_ORACLE_SOURCE: u8 = 0; // 0: mock,1: pyth
pub const TEST_ETH_ASSET_INDEX: u8 = 2; // 0:usdc, 1:btc, 2:eth, 3:sol
pub const TEST_ETH_SIGNIFICANT_DECIMALS: u8 = 2;

//SOL market
pub const TEST_SOL_MARKET_SYMBOL: &str = "SOL";
pub const TEST_SOL_MINIMUM_COLLATERAL: u64 = 25;
pub const TEST_SOL_CHARGE_BORROW_FEE_INTERVAL: u64 = 3600;
pub const TEST_SOL_OPEN_FEE_RATE: u16 = OPEN_FEE_RATE; // 0.3% (30 / 10000)
pub const TEST_SOL_CLOSE_FEE_RATE: u16 = CLOSE_FEE_RATE; // 0.5%   (50 /  10000)
pub const TEST_SOL_LIQUIDATE_FEE_RATE: u16 = LIQUIDATE_FEE_RATE; // 0.8%   (80 /  10000)
pub const TEST_SOL_MARKET_DECIMALS: u8 = 9;
pub const TEST_SOL_ORACLE_SOURCE: u8 = 0; // 0: mock,1: pyth
pub const TEST_SOL_ASSET_INDEX: u8 = 3; // 0:usdc, 1:btc, 2:eth, 3:sol
pub const TEST_SOL_SIGNIFICANT_DECIMALS: u8 = 2;

pub const TEST_DI_FEE_RATE: u16 = 30;

pub fn add_fee(a: f64) -> f64 {
    a * ADD_LIQUIDITY_FEE_RATE as f64 / 10000.0
}

pub fn remove_fee(a: f64) -> f64 {
    a * REMOVE_LIQUIDITY_FEE_RATE as f64 / 10000.0
}

pub fn swap_fee(a: f64) -> f64 {
    a * SWAP_FEE_RATE as f64 / 10000.0
}

pub fn minus_add_fee(a: f64) -> f64 {
    a - add_fee(a)
}

pub fn minus_remove_fee(a: f64) -> f64 {
    a - remove_fee(a)
}

pub fn minus_swap_fee(a: f64) -> f64 {
    a - swap_fee(a)
}

pub fn open_fee(a: f64) -> f64 {
    a * OPEN_FEE_RATE as f64 / 10000.0
}

pub fn close_fee(a: f64) -> f64 {
    a * CLOSE_FEE_RATE as f64 / 10000.0
}

pub fn minus_open_fee(a: f64) -> f64 {
    a - open_fee(a)
}

// Add SOL when creating dex
pub const INIT_ADD_SOL_AMOUNT: f64 = 1000.0;

// VLP
pub const TEST_VLP_DECIMALS: u8 = 6;
pub const INIT_VLP_AMOUNT: f64 = 199_800.0; //1000 * 200 -（1000 * 200）*0.1%

pub fn btc(size: f64) -> u64 {
    (size * (10u64.pow(TEST_BTC_DECIMALS as u32) as f64)) as u64
}

pub fn sol(size: f64) -> u64 {
    (size * (10u64.pow(TEST_SOL_DECIMALS as u32) as f64)) as u64
}

pub fn eth(size: f64) -> u64 {
    (size * (10u64.pow(TEST_ETH_DECIMALS as u32) as f64)) as u64
}

pub fn usdc(size: f64) -> u64 {
    (size * (10u64.pow(TEST_USDC_DECIMALS as u32) as f64)) as u64
}

pub fn usdc_i(size: f64) -> i64 {
    (size * (10u64.pow(TEST_USDC_DECIMALS as u32) as f64)) as i64
}

pub fn btc_i(size: f64) -> i64 {
    (size * (10u64.pow(TEST_BTC_DECIMALS as u32) as f64)) as i64
}

pub fn eth_i(size: f64) -> i64 {
    (size * (10u64.pow(TEST_ETH_DECIMALS as u32) as f64)) as i64
}

pub fn sol_i(size: f64) -> i64 {
    (size * (10u64.pow(TEST_SOL_DECIMALS as u32) as f64)) as i64
}

pub fn es_vdx(size: f64) -> u64 {
    (size * (10u64.pow(TEST_USDC_DECIMALS as u32) as f64)) as u64
}
