//init user wallet asset amount
pub const INIT_WALLET_BTC_ASSET_AMOUNT: f64 = 10.0;
pub const INIT_WALLET_ETH_ASSET_AMOUNT: f64 = 100.0;
pub const INIT_WALLET_SOL_ASSET_AMOUNT: f64 = 1000.0;
pub const INIT_WALLET_USDC_ASSET_AMOUNT: f64 = 100_000.0;

//USDC ASSET
pub const TEST_USDC_SYMBOL: &str = "USDC";
pub const TEST_USDC_DECIMALS: u8 = 6;
pub const TEST_USDC_ORACLE_PRICE: f64 = 1.0;
pub const TEST_USDC_ORACLE_EXPO: u8 = 5;

pub const TEST_USDC_BORROW_FEE_RATE: u16 = 10; //1-10_000  0.1%
pub const TEST_USDC_ADD_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
pub const TEST_USDC_REMOVE_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
pub const TEST_USDC_TARGET_WEIGHT: u16 = 400; //1-1000 //40%

//BTC asset
pub const TEST_BTC_SYMBOL: &str = "BTC";
pub const TEST_BTC_DECIMALS: u8 = 9;
pub const TEST_BTC_ORACLE_PRICE: f64 = 20_000.0;
pub const TEST_BTC_ORACLE_EXPO: u8 = 8;

pub const TEST_BTC_BORROW_FEE_RATE: u16 = 10; //1-10_000  0.1%
pub const TEST_BTC_ADD_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
pub const TEST_BTC_REMOVE_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
pub const TEST_BTC_TARGET_WEIGHT: u16 = 300; //1-1000 //30%

//ETH asset
pub const TEST_ETH_SYMBOL: &str = "ETH";
pub const TEST_ETH_DECIMALS: u8 = 9;
pub const TEST_ETH_ORACLE_PRICE: f64 = 2_000.0;
pub const TEST_ETH_ORACLE_EXPO: u8 = 8;

pub const TEST_ETH_BORROW_FEE_RATE: u16 = 10; //1-10_000  0.1%
pub const TEST_ETH_ADD_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
pub const TEST_ETH_REMOVE_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
pub const TEST_ETH_TARGET_WEIGHT: u16 = 200; //1-1000 //20%

//SOL asset
pub const TEST_SOL_SYMBOL: &str = "SOL";
pub const TEST_SOL_DECIMALS: u8 = 9;
pub const TEST_SOL_ORACLE_PRICE: f64 = 2_00.0;
pub const TEST_SOL_ORACLE_EXPO: u8 = 8;

pub const TEST_SOL_BORROW_FEE_RATE: u16 = 10; //1-10_000  0.1%
pub const TEST_SOL_ADD_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
pub const TEST_SOL_REMOVE_LIQUIDITY_FEE_RATE: u16 = 10; //0.1%
pub const TEST_SOL_TARGET_WEIGHT: u16 = 200; //1-1000 //20%

//BTC market
pub const TEST_BTC_MARKET_SYMBOL: &str = "BTC";
pub const TEST_BTC_MINIMUM_POSITION_VALUE: u64 = 10_000;
pub const TEST_BTC_CHARGE_BORROW_FEE_INTERVAL: u64 = 3600;
pub const TEST_BTC_OPEN_FEE_RATE: u16 = 30; // 0.3% (30 / 10000)
pub const TEST_BTC_CLOSE_FEE_RATE: u16 = 50; // 0.5%   (50 /  10000)
pub const TEST_BTC_LIQUIDITY_FEE_RATE: u16 = 80; // 0.8%   (80 /  10000)
pub const TEST_BTC_MARKET_DECIMALS: u8 = 9;
pub const TEST_BTC_ORACLE_SOURCE: u8 = 0; // 0: mock,1: pyth
pub const TEST_BTC_ASSET_INDEX: u8 = 1; // 0:usdc, 1:btc, 2:eth, 3:sol
pub const TEST_BTC_SIGNIFICANT_DECIMALS: u8 = 2;

//ETH market
pub const TEST_ETH_MARKET_SYMBOL: &str = "ETH";
pub const TEST_ETH_MINIMUM_POSITION_VALUE: u64 = 10_000;
pub const TEST_ETH_CHARGE_BORROW_FEE_INTERVAL: u64 = 3600;
pub const TEST_ETH_OPEN_FEE_RATE: u16 = 30; // 0.3% (30 / 10000)
pub const TEST_ETH_CLOSE_FEE_RATE: u16 = 50; // 0.5%   (50 /  10000)
pub const TEST_ETH_LIQUIDITY_FEE_RATE: u16 = 80; // 0.8%   (80 /  10000)
pub const TEST_ETH_MARKET_DECIMALS: u8 = 9;
pub const TEST_ETH_ORACLE_SOURCE: u8 = 0; // 0: mock,1: pyth
pub const TEST_ETH_ASSET_INDEX: u8 = 2; // 0:usdc, 1:btc, 2:eth, 3:sol
pub const TEST_ETH_SIGNIFICANT_DECIMALS: u8 = 2;

//SOL market
pub const TEST_SOL_MARKET_SYMBOL: &str = "SOL";
pub const TEST_SOL_MINIMUM_POSITION_VALUE: u64 = 10_000;
pub const TEST_SOL_CHARGE_BORROW_FEE_INTERVAL: u64 = 3600;
pub const TEST_SOL_OPEN_FEE_RATE: u16 = 30; // 0.3% (30 / 10000)
pub const TEST_SOL_CLOSE_FEE_RATE: u16 = 50; // 0.5%   (50 /  10000)
pub const TEST_SOL_LIQUIDITY_FEE_RATE: u16 = 80; // 0.8%   (80 /  10000)
pub const TEST_SOL_MARKET_DECIMALS: u8 = 9;
pub const TEST_SOL_ORACLE_SOURCE: u8 = 0; // 0: mock,1: pyth
pub const TEST_SOL_ASSET_INDEX: u8 = 3; // 0:usdc, 1:btc, 2:eth, 3:sol
pub const TEST_SOL_SIGNIFICANT_DECIMALS: u8 = 2;