pub const NIL8: u8 = u8::MAX;
pub const NIL16: u16 = u16::MAX;
pub const NIL32: u32 = u32::MAX;

pub const DEX_MAGIC_NUMBER: u64 = 0x6666;
pub const MOCK_ORACLE_MAGIC_NUMBER: u64 = 0x66667;
pub const USER_STATE_MAGIC_NUMBER: u32 = 0x6668;
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

pub const REWARD_SHARE_POW_DECIMALS: u64 = 10u64.pow(12 as u32);
