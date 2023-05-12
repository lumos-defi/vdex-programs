pub const NIL8: u8 = u8::MAX;
pub const NIL16: u16 = u16::MAX;
pub const NIL32: u32 = u32::MAX;

pub const SECONDS_PER_DAY: i64 = 3600 * 24;
pub const MAX_ASSET_COUNT: usize = 16;
pub const MAX_MARKET_COUNT: usize = 16;
pub const MAX_USER_LIST_REMAINING_PAGES_COUNT: usize = 15;
pub const MAX_PRICE_COUNT: usize = 16;

pub const DEX_MAGIC_NUMBER: u64 = 0x6666;
pub const MOCK_ORACLE_MAGIC_NUMBER: u64 = 0x66667;
pub const USER_STATE_MAGIC_NUMBER: u32 = 0x6668;
pub const DI_ACCOUNT_MAGIC_NUMBER: u32 = 0x6669;
pub const PRICE_FEED_MAGIC_NUMBER: u64 = 0x666A;
pub const ORDER_POOL_MAGIC_BYTE: u8 = 0x30;
pub const USER_LIST_MAGIC_BYTE: u8 = 0x31;

pub const LEVERAGE_POW_DECIMALS: u32 = 1000;
pub const FEE_RATE_DECIMALS: u32 = 4;
pub const FEE_RATE_BASE: u128 = 10000;
pub const BORROW_FEE_RATE_BASE: u128 = 100_0000;

pub const USDC_DECIMALS: u8 = 6;
pub const USD_POW_DECIMALS: u64 = 10u64.pow(USDC_DECIMALS as u32);
pub const USDC_POW_DECIMALS: u64 = 10u64.pow(USDC_DECIMALS as u32);

pub const MAX_FILLED_PER_INSTRUCTION: u32 = 20;

pub const REWARD_SHARE_POW_DECIMALS: u64 = 10u64.pow(9 as u32);

pub const VLP_DECIMALS: u8 = 6;
pub const VDX_DECIMALS: u8 = 6;
pub const VDX_TOTAL_SUPPLY: u64 = 20000000 * 10u64.pow(VDX_DECIMALS as u32);
pub const ES_VDX_PER_SECOND: u64 = 32000; // Carrying decimals

pub const ES_VDX_PERCENTAGE_FOR_VDX_POOL: u32 = 50;
pub const REWARD_PERCENTAGE_FOR_VDX_POOL: u32 = 30;

pub const VESTING_PERIOD: u16 = 360;
pub const UPDATE_REWARDS_PERIOD: i64 = 1800;

pub const ASSET_VDX: u8 = u8::MAX;
pub const ASSET_REWARDS: u8 = u8::MAX - 1;

pub const MASK_PERP_POSITION: u8 = 0x1;
pub const MASK_DI_OPTION: u8 = 0x2;
