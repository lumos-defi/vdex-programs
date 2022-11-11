use anchor_lang::prelude::*;

use crate::{
    errors::{DexError, DexResult},
    utils::{
        time::get_timestamp, ISafeAddSub, ISafeMath, SafeMath, FEE_RATE_BASE, FEE_RATE_DECIMALS,
    },
};

use super::get_oracle_price;

#[account(zero_copy)]
pub struct Dex {
    pub magic: u64,
    pub assets: [AssetInfo; 16],
    pub markets: [MarketInfo; 16],
    pub authority: Pubkey,
    pub event_queue: Pubkey,
    pub match_queue: Pubkey,
    pub usdc_mint: Pubkey,
    pub vlp_mint: Pubkey,
    pub vlp_mint_authority: Pubkey,
    pub user_list_entry_page: Pubkey,
    pub user_list_remaining_pages: [Pubkey; 8],
    pub user_list_remaining_pages_number: u8,
    pub assets_number: u8,
    pub markets_number: u8,
    pub vlp_mint_nonce: u8,
    pub usdc_asset_index: u8,
    pub padding: [u8; 251],
}

impl Dex {
    pub fn borrow_fund(
        &mut self,
        market: usize,
        long: bool,
        collateral: u64,
        borrow: u64,
        open_fee: u64,
    ) -> DexResult {
        require!(
            market < self.markets_number as usize,
            DexError::InvalidMarketIndex
        );

        let mi = &mut self.markets[market];
        require!(mi.valid, DexError::InvalidMarketIndex);

        let asset_index = if long {
            mi.asset_index
        } else {
            self.usdc_asset_index
        } as usize;

        require!(
            asset_index < self.assets_number as usize,
            DexError::InvalidMarketIndex
        );
        let ai = &mut self.assets[asset_index];
        require!(ai.valid, DexError::InvalidMarketIndex);

        ai.fee_amount = ai.fee_amount.safe_add(open_fee)?;
        ai.liquidity_amount = ai
            .liquidity_amount
            .safe_sub(borrow)
            .map_err(|_| error!(DexError::InsufficientLiquidity))?;
        ai.collateral_amount = ai.collateral_amount.safe_add(collateral)?;
        ai.borrowed_amount = ai.borrowed_amount.safe_add(borrow)?;

        Ok(())
    }

    pub fn settle_pnl(
        &mut self,
        market: usize,
        long: bool,
        collateral: u64,
        borrow: u64,
        pnl: i64,
        close_fee: u64,
        borrow_fee: u64,
    ) -> DexResult<u64> {
        require!(
            market < self.markets_number as usize,
            DexError::InvalidMarketIndex
        );

        let mi = &mut self.markets[market];
        require!(mi.valid, DexError::InvalidMarketIndex);

        let asset_index = if long {
            mi.asset_index
        } else {
            self.usdc_asset_index
        } as usize;

        require!(
            asset_index < self.assets_number as usize,
            DexError::InvalidMarketIndex
        );

        let ai = &mut self.assets[asset_index];
        require!(ai.valid, DexError::InvalidMarketIndex);

        ai.liquidity_amount = ai.liquidity_amount.safe_add(borrow)?;
        ai.collateral_amount = ai.collateral_amount.safe_sub(collateral)?;
        ai.borrowed_amount = ai.borrowed_amount.safe_sub(borrow)?;
        ai.fee_amount = ai.fee_amount.safe_add(close_fee)?.safe_add(borrow_fee)?;

        let total_fee = borrow_fee.safe_add(close_fee)?;
        let abs_pnl = i64::abs(pnl) as u64;
        let user_withdrawable = if pnl >= 0 {
            // User take the profit
            ai.liquidity_amount = ai.liquidity_amount.safe_sub(abs_pnl)?;
            match collateral.safe_add(abs_pnl)?.safe_sub(total_fee) {
                Ok(v) => v,
                Err(_) => 0,
            }
        } else {
            // Pool take the profit
            let pnl_and_fee = total_fee.safe_add(abs_pnl)?;
            match collateral.safe_sub(pnl_and_fee) {
                Ok(remain) => {
                    ai.liquidity_amount = ai.liquidity_amount.safe_add(abs_pnl)?;
                    remain
                }
                Err(_) => {
                    ai.liquidity_amount = ai
                        .liquidity_amount
                        .safe_add(collateral)?
                        .safe_sub(total_fee)?;
                    0
                }
            }
        };

        Ok(user_withdrawable)
    }

    pub fn increase_global_position(
        &mut self,
        market: usize,
        long: bool,
        price: u64,
        size: u64,
        collateral: u64,
    ) -> DexResult {
        require!(
            market < self.markets_number as usize,
            DexError::InvalidMarketIndex
        );

        let pos = if long {
            &mut self.markets[market].global_long
        } else {
            &mut self.markets[market].global_short
        };

        let merged_size = pos.size.safe_add(size)?;

        pos.average_price = pos
            .average_price
            .safe_mul(pos.size)?
            .safe_add(price.safe_mul(size)?)?
            .safe_div(merged_size as u128)? as u64;

        pos.size = merged_size;
        pos.collateral = pos.collateral.safe_add(collateral)?;
        pos.last_fill_time = get_timestamp()?;

        Ok(())
    }

    pub fn decrease_global_position(
        &mut self,
        market: usize,
        long: bool,
        size: u64,
        collateral: u64,
    ) -> DexResult {
        require!(
            market < self.markets_number as usize,
            DexError::InvalidMarketIndex
        );

        let pos = if long {
            &mut self.markets[market].global_long
        } else {
            &mut self.markets[market].global_short
        };

        pos.collateral = pos.collateral.safe_sub(collateral)?;
        pos.size = pos.size.safe_sub(size)?;
        pos.last_fill_time = get_timestamp()?;

        if pos.size == 0 {
            pos.zero(long)?;
        }

        Ok(())
    }
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
    pub fee_amount: u64,
    pub add_liquidity_fee: u64,
    pub remove_liquidity_fee: u64,
    pub borrow_fee_rate: u16,
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

    pub minimum_position_value: u64,
    pub charge_borrow_fee_interval: u64,
    pub open_fee_rate: u16,
    pub close_fee_rate: u16,
    pub liquidate_fee_rate: u16,
    pub valid: bool,
    pub decimals: u8,
    pub oracle_source: u8,
    pub asset_index: u8,
    pub significant_decimals: u8,
    pub padding: [u8; 253],
}

pub struct MarketFeeRates {
    pub charge_borrow_fee_interval: u64,
    pub minimum_position_value: u64,
    pub borrow_fee_rate: u16,
    pub open_fee_rate: u16,
    pub close_fee_rate: u16,
    pub liquidate_fee_rate: u16,
    pub base_decimals: u8,
}

impl MarketInfo {
    pub fn get_fee_rates(&self, borrow_fee_rate: u16) -> MarketFeeRates {
        MarketFeeRates {
            charge_borrow_fee_interval: self.charge_borrow_fee_interval,
            minimum_position_value: self.minimum_position_value,
            borrow_fee_rate,
            open_fee_rate: self.open_fee_rate,
            close_fee_rate: self.close_fee_rate,
            liquidate_fee_rate: self.liquidate_fee_rate,
            base_decimals: self.decimals,
        }
    }

    pub fn un_pnl(&self, price: u64) -> DexResult<i64> {
        let short_pnl = (self.global_short.average_price as i128 - price as i128)
            .i_safe_mul(self.global_short.size as i128)?
            .i_safe_div(10i128.pow(self.decimals as u32))? as i64;

        let long_pnl = (price as i128 - self.global_long.average_price as i128)
            .i_safe_mul(self.global_long.size as i128)?
            .i_safe_div(10i128.pow(self.decimals as u32))? as i64;

        short_pnl.i_safe_add(long_pnl)
    }
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
        price: u64,
        amount: u64,
        leverage: u32,
        mfr: &MarketFeeRates,
    ) -> DexResult<(u64, u64, u64, u64)> {
        let (collateral, open_fee) =
            Position::calc_collateral_and_fee(amount, leverage, mfr.open_fee_rate)?;

        let size = if self.long {
            collateral.safe_mul(leverage as u64)
        } else {
            collateral
                .safe_mul(leverage as u64)?
                .safe_mul(10u128.pow(mfr.base_decimals.into()))?
                .safe_div(price as u128)
        }? as u64;

        // Update cumulative fund fee
        let now = get_timestamp()?;
        let cumulative_fund_fee = if self.borrowed_amount > 0 {
            require!(self.last_fill_time >= now, DexError::InvalidPositionTime);

            self.borrowed_amount
                .safe_mul(mfr.borrow_fee_rate as u64)?
                .safe_mul((now - self.last_fill_time) as u128)?
                .safe_div(FEE_RATE_BASE)?
                .safe_div(mfr.charge_borrow_fee_interval as u128)? as u64
                + self.cumulative_fund_fee
        } else {
            0
        };

        // Update borrowed amount
        let borrow = if self.long {
            Ok(size as u128)
        } else {
            collateral.safe_mul(leverage as u64)
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

        Ok((size, collateral, borrow, open_fee))
    }

    pub fn close(
        &mut self,
        size: u64,
        price: u64,
        mfr: &MarketFeeRates,
        liquidate: bool,
    ) -> DexResult<(u64, u64, i64, u64, u64)> {
        let mut collateral_unlocked = size
            .safe_mul(self.collateral)?
            .safe_div(self.size as u128)? as u64;

        let mut fund_returned = size
            .safe_mul(self.borrowed_amount)?
            .safe_div(self.size as u128)? as u64;

        // Update cumulative fund fee
        let now = get_timestamp()?;
        let borrow_fee = if self.borrowed_amount > 0 {
            require!(self.last_fill_time >= now, DexError::InvalidPositionTime);

            self.borrowed_amount
                .safe_mul(mfr.borrow_fee_rate as u64)?
                .safe_mul((now - self.last_fill_time) as u128)?
                .safe_div(FEE_RATE_BASE)?
                .safe_div(mfr.charge_borrow_fee_interval as u128)? as u64
                + self.cumulative_fund_fee
        } else {
            0
        };

        // Calculate close position fee
        let rate = if liquidate {
            mfr.liquidate_fee_rate
        } else {
            mfr.close_fee_rate
        } as u64;
        let close_fee = if self.long {
            size.safe_mul(rate)?.safe_div(FEE_RATE_BASE)? as u64
        } else {
            size.safe_mul(price)?
                .safe_mul(rate as u128)?
                .safe_div(10u64.pow(mfr.base_decimals as u32 + FEE_RATE_DECIMALS) as u128)?
                as u64
        };

        let total_fee = borrow_fee.safe_add(close_fee)?;
        let pnl = self.pnl(size, price, self.average_price, mfr.base_decimals)?;
        let pnl_with_fee = pnl.i_safe_sub(total_fee as i64)?;

        // Update the position
        self.borrowed_amount = self.borrowed_amount.safe_sub(fund_returned)?;
        self.collateral = self.collateral.safe_sub(collateral_unlocked)?;
        self.size = self.size.safe_sub(size)?;
        self.cumulative_fund_fee = 0;
        self.last_fill_time = now;

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

        if self.size > 0 {
            require!(
                self.size.safe_mul(price)? as u64 >= mfr.minimum_position_value,
                DexError::PositionTooSmall
            );
        }

        Ok((
            fund_returned,
            collateral_unlocked,
            pnl,
            close_fee,
            borrow_fee,
        ))
    }

    fn calc_collateral_and_fee(amount: u64, leverage: u32, rate: u16) -> DexResult<(u64, u64)> {
        let temp = (leverage as u64).safe_mul(rate as u64)? as u64;

        let dividend = amount.safe_mul(temp)?;
        let divisor = (FEE_RATE_BASE as u128).safe_add(temp as u128)?;

        let fee = dividend.safe_div(divisor)? as u64;
        let collateral = amount.safe_sub(fee)?;

        Ok((collateral, fee))
    }

    pub fn pnl(
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

pub trait GetOraclePrice {
    fn get_price(&self) -> Result<(u64, u8)>;
}

pub struct OracleInfo<'a, 'info> {
    pub base_decimals: u8,
    pub oracle_source: u8,
    pub oracle_account: &'a AccountInfo<'info>,
}

impl GetOraclePrice for OracleInfo<'_, '_> {
    fn get_price(&self) -> Result<(u64, u8)> {
        let price = get_oracle_price(self.oracle_source, self.oracle_account)?;

        Ok((price, self.base_decimals))
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::utils::unit_test::*;

    impl Default for Dex {
        fn default() -> Dex {
            unsafe { std::mem::zeroed() }
        }
    }
    impl Default for AssetInfo {
        fn default() -> AssetInfo {
            unsafe { std::mem::zeroed() }
        }
    }
    impl Default for MarketInfo {
        fn default() -> MarketInfo {
            unsafe { std::mem::zeroed() }
        }
    }

    #[test]
    fn test_market_un_pnl() {
        let mut market = MarketInfo::default();
        assert!(market.valid == false);
        market.decimals = BTC_DECIMALS;

        market.global_long = Position::new(true).assert_unwrap();
        market.global_short = Position::new(false).assert_unwrap();

        assert_eq!(market.un_pnl(usdc(20000.)).assert_unwrap(), 0i64);

        market.global_long.average_price = usdc(20000.);
        market.global_long.size = btc(1.0);
        market.global_long.collateral = btc(0.1);

        assert_eq!(
            market.un_pnl(usdc(22000.)).assert_unwrap(),
            usdc(2000.) as i64
        );

        market.global_short.average_price = usdc(23000.);
        market.global_short.size = btc(1.0);
        market.global_short.collateral = usdc(2300.);

        assert_eq!(
            market.un_pnl(usdc(22000.)).assert_unwrap(),
            usdc(3000.) as i64
        );

        assert_eq!(
            market.un_pnl(usdc(25000.)).assert_unwrap(),
            usdc(3000.) as i64
        );
    }

    impl Dex {
        pub fn mock_dex(&mut self) {
            // BTC
            self.assets[0] = AssetInfo {
                valid: true,
                decimals: 9,
                borrow_fee_rate: 10,
                ..AssetInfo::default()
            };
            self.assets_number += 1;

            // USDC
            self.assets[1] = AssetInfo {
                valid: true,
                decimals: 6,
                borrow_fee_rate: 10,
                ..AssetInfo::default()
            };
            self.assets_number += 1;

            self.markets[0] = MarketInfo {
                valid: true,
                decimals: 9,
                asset_index: 0,
                open_fee_rate: 20,
                close_fee_rate: 20,
                liquidate_fee_rate: 50,
                charge_borrow_fee_interval: 3600,
                global_long: Position::new(true).assert_unwrap(),
                global_short: Position::new(false).assert_unwrap(),
                ..MarketInfo::default()
            };
            self.markets_number += 1;
            self.usdc_asset_index = 1;
        }

        pub fn mock_btc_liquidity(&mut self, amount: u64) {
            self.assets[0].liquidity_amount = amount;
        }

        pub fn mock_usdc_liquidity(&mut self, amount: u64) {
            self.assets[1].liquidity_amount = amount;
        }

        // Asset BTC properties
        pub fn assert_btc_liquidity(&self, amount: u64) {
            assert_eq!(self.assets[0].liquidity_amount, amount)
        }

        pub fn assert_btc_collateral(&self, amount: u64) {
            assert_eq!(self.assets[0].collateral_amount, amount)
        }

        pub fn assert_btc_borrowed(&self, amount: u64) {
            assert_eq!(self.assets[0].borrowed_amount, amount)
        }

        pub fn assert_btc_fee_amount(&self, amount: u64) {
            assert_eq!(self.assets[0].fee_amount, amount)
        }

        // Asset USDC properties
        pub fn assert_usdc_liquidity(&self, amount: u64) {
            assert_eq!(self.assets[1].liquidity_amount, amount)
        }

        pub fn assert_usdc_collateral(&self, amount: u64) {
            assert_eq!(self.assets[1].collateral_amount, amount)
        }

        pub fn assert_usdc_borrowed(&self, amount: u64) {
            assert_eq!(self.assets[1].borrowed_amount, amount)
        }

        pub fn assert_usdc_fee_amount(&self, amount: u64) {
            assert_eq!(self.assets[1].fee_amount, amount)
        }
    }

    impl Position {
        pub fn mock_after_hours(&mut self, hours: u64) {
            self.last_fill_time = self
                .last_fill_time
                .i_safe_sub((hours as i64) * 3600)
                .unwrap();
        }
    }

    #[test]
    fn test_global_pos_invalid_market() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(1, true, usdc(20000.), btc(1.0), btc(0.1))
            .assert_err();

        dex.increase_global_position(0xff, true, usdc(20000.), btc(1.0), btc(0.1))
            .assert_err();

        dex.increase_global_position(0, true, usdc(20000.), btc(1.0), btc(0.1))
            .assert_ok();

        dex.decrease_global_position(1, true, btc(0.5), btc(0.05))
            .assert_err();
    }

    #[test]
    fn test_increase_global_long() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(0, true, usdc(20000.), btc(1.0), btc(0.1))
            .assert_ok();

        let long = &dex.markets[0].global_long;
        assert_eq!(long.size, btc(1.0));
        assert_eq!(long.collateral, btc(0.1));
        assert_eq!(long.average_price, usdc(20000.));

        dex.increase_global_position(0, true, usdc(26000.), btc(0.5), btc(0.05))
            .assert_ok();

        let long = &dex.markets[0].global_long;
        assert_eq!(long.size, btc(1.5));
        assert_eq!(long.collateral, btc(0.15));
        assert_eq!(long.average_price, usdc(22000.));
    }

    #[test]
    fn test_decrease_global_long() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(0, true, usdc(20000.), btc(1.0), btc(0.1))
            .assert_ok();

        dex.decrease_global_position(0, true, btc(0.5), btc(0.05))
            .assert_ok();

        let long = &dex.markets[0].global_long;
        assert_eq!(long.size, btc(0.5));
        assert_eq!(long.collateral, btc(0.05));
        assert_eq!(long.average_price, usdc(20000.));

        dex.decrease_global_position(0, true, btc(0.5), btc(0.05))
            .assert_ok();

        let long = &dex.markets[0].global_long;
        assert_eq!(long.size, btc(0.));
        assert_eq!(long.collateral, btc(0.));
        assert_eq!(long.average_price, usdc(0.));
    }

    #[test]
    fn test_increase_global_short() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(0, false, usdc(20000.), btc(1.0), usdc(2000.))
            .assert_ok();

        let long = &dex.markets[0].global_short;
        assert_eq!(long.size, btc(1.0));
        assert_eq!(long.collateral, usdc(2000.));
        assert_eq!(long.average_price, usdc(20000.));

        dex.increase_global_position(0, false, usdc(18000.), btc(1.0), usdc(1800.))
            .assert_ok();

        let long = &dex.markets[0].global_short;
        assert_eq!(long.size, btc(2.0));
        assert_eq!(long.collateral, usdc(3800.));
        assert_eq!(long.average_price, usdc(19000.));
    }

    #[test]
    fn test_decrease_global_short() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(0, false, usdc(20000.), btc(1.0), usdc(2000.))
            .assert_ok();

        dex.decrease_global_position(0, false, btc(0.5), usdc(1000.))
            .assert_ok();

        let long = &dex.markets[0].global_short;
        assert_eq!(long.size, btc(0.5));
        assert_eq!(long.collateral, usdc(1000.));
        assert_eq!(long.average_price, usdc(20000.));

        dex.decrease_global_position(0, false, btc(0.5), usdc(1000.))
            .assert_ok();

        let long = &dex.markets[0].global_short;
        assert_eq!(long.size, btc(0.));
        assert_eq!(long.collateral, usdc(0.));
        assert_eq!(long.average_price, usdc(0.));
    }

    #[test]
    fn test_decrease_global_long_collateral_overflow() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(0, true, usdc(20000.), btc(1.0), btc(0.1))
            .assert_ok();

        dex.decrease_global_position(0, true, btc(0.5), btc(0.11))
            .assert_err();
    }

    #[test]
    fn test_decrease_global_long_size_overflow() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(0, true, usdc(20000.), btc(1.0), btc(0.1))
            .assert_ok();

        dex.decrease_global_position(0, true, btc(1.1), btc(0.05))
            .assert_err();
    }

    #[test]
    fn test_decrease_global_short_collateral_overflow() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(0, false, usdc(20000.), btc(1.0), usdc(2000.))
            .assert_ok();

        dex.decrease_global_position(0, false, btc(0.5), usdc(2100.))
            .assert_err();
    }

    #[test]
    fn test_decrease_global_short_size_overflow() {
        let mut dex = Dex::default();
        dex.mock_dex();

        dex.increase_global_position(0, false, usdc(20000.), btc(1.0), usdc(2000.))
            .assert_ok();

        dex.decrease_global_position(0, false, btc(1.1), usdc(2000.))
            .assert_err();
    }

    #[test]
    fn test_new_position() {
        let long = Position::new(true).assert_unwrap();
        assert!(long.long);
        assert_eq!(long.size, 0);
        assert_eq!(long.average_price, 0);
        assert_eq!(long.collateral, 0);
        assert_eq!(long.borrowed_amount, 0);
        assert_eq!(long.closing_size, 0);
        assert_eq!(long.cumulative_fund_fee, 0);

        let short = Position::new(false).assert_unwrap();
        assert!(!short.long);
        assert_eq!(short.size, 0);
        assert_eq!(short.average_price, 0);
        assert_eq!(short.collateral, 0);
        assert_eq!(short.borrowed_amount, 0);
        assert_eq!(short.closing_size, 0);
        assert_eq!(short.cumulative_fund_fee, 0);
    }

    #[test]
    fn test_open_long_position() {
        let mut dex = Dex::default();
        dex.mock_dex();
        let mfr = dex.markets[0].get_fee_rates(20);

        let mut long = Position::new(true).assert_unwrap();
        let (size, collateral, borrow, open_fee) =
            long.open(usdc(20000.), btc(1.0), 20, &mfr).assert_unwrap();

        let expected_open_fee = btc(0.038461538);
        let expected_collateral = btc(1.0) - expected_open_fee;

        assert_eq!(open_fee, expected_open_fee);
        assert_eq!(collateral, expected_collateral);
        assert_eq!(size, expected_collateral * 20);
        assert_eq!(borrow, expected_collateral * 20);

        assert_eq!(long.size, expected_collateral * 20);
        assert_eq!(long.average_price, usdc(20000.));
        assert_eq!(long.collateral, expected_collateral);
        assert_eq!(long.borrowed_amount, expected_collateral * 20);
        assert_eq!(long.closing_size, 0);
        assert_eq!(long.cumulative_fund_fee, 0);

        const HOURS_2: u64 = 2;
        long.mock_after_hours(HOURS_2);

        // Long more
        long.open(usdc(26000.), btc(1.0), 20, &mfr).assert_unwrap();
        assert_eq!(open_fee, expected_open_fee);
        assert_eq!(collateral, expected_collateral);
        assert_eq!(size, expected_collateral * 20);
        assert_eq!(borrow, expected_collateral * 20);

        assert_eq!(long.size, expected_collateral * 20 * 2);
        assert_eq!(long.average_price, usdc(23000.));
        assert_eq!(long.collateral, expected_collateral * 2);
        assert_eq!(long.borrowed_amount, expected_collateral * 20 * 2);
        assert_eq!(long.closing_size, 0);

        let expected_fund_fee = expected_collateral * 20 * (mfr.borrow_fee_rate as u64) * HOURS_2
            / FEE_RATE_BASE as u64;
        assert_eq!(long.cumulative_fund_fee, expected_fund_fee);
    }

    #[test]
    fn test_close_long_position_with_profit() {
        let mut dex = Dex::default();
        dex.mock_dex();
        let mfr = dex.markets[0].get_fee_rates(20);

        let mut long = Position::new(true).assert_unwrap();
        let leverage = 20u64;
        let (size, collateral, borrow, _) = long
            .open(usdc(20000.), btc(1.0), leverage as u32, &mfr)
            .assert_unwrap();

        const HOURS_2: u64 = 2;
        long.mock_after_hours(HOURS_2);

        let (returned, collateral_unlocked, pnl, close_fee, borrow_fee) =
            long.close(size, usdc(25000.), &mfr, false).assert_unwrap();

        let expected_borrow_fee =
            collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2 / FEE_RATE_BASE as u64;

        let expected_close_fee = size * (mfr.close_fee_rate as u64) / FEE_RATE_BASE as u64;

        let expected_pnl =
            (size as u128) * (usdc(25000.) - usdc(20000.)) as u128 / usdc(20000.) as u128;

        assert_eq!(returned, borrow);
        assert_eq!(collateral_unlocked, collateral);
        assert_eq!(borrow_fee, expected_borrow_fee);
        assert_eq!(close_fee, expected_close_fee);
        assert_eq!(pnl, expected_pnl as i64);
    }

    #[test]
    fn test_close_long_position_with_loss() {
        let mut dex = Dex::default();
        dex.mock_dex();
        let mfr = dex.markets[0].get_fee_rates(20);

        let mut long = Position::new(true).assert_unwrap();
        let leverage = 5u64;
        let (size, collateral, borrow, _) = long
            .open(usdc(20000.), btc(1.0), leverage as u32, &mfr)
            .assert_unwrap();

        const HOURS_2: u64 = 2;
        long.mock_after_hours(HOURS_2);

        let (returned, collateral_unlocked, pnl, close_fee, borrow_fee) =
            long.close(size, usdc(18000.), &mfr, false).assert_unwrap();

        let expected_borrow_fee =
            collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2 / FEE_RATE_BASE as u64;

        let expected_close_fee = size * (mfr.close_fee_rate as u64) / FEE_RATE_BASE as u64;

        let expected_pnl = size * (usdc(20000.) - usdc(18000.)) / usdc(20000.);

        assert_eq!(returned, borrow);
        assert_eq!(collateral_unlocked, collateral);
        assert_eq!(borrow_fee, expected_borrow_fee);
        assert_eq!(close_fee, expected_close_fee);
        assert_eq!(pnl, -(expected_pnl as i64));
    }

    #[test]
    fn test_open_short_position() {
        let mut dex = Dex::default();
        dex.mock_dex();
        let mfr = dex.markets[0].get_fee_rates(20);

        let mut short = Position::new(false).assert_unwrap();
        let leverage = 10u64;
        let (size, collateral, borrow, open_fee) = short
            .open(usdc(20000.), usdc(2000.), leverage as u32, &mfr)
            .assert_unwrap();

        let expected_open_fee = usdc(39.215686);
        let expected_collateral = usdc(2000.0) - expected_open_fee;
        let expected_size = ((expected_collateral as u128)
            * (leverage as u128)
            * 10u128.pow(mfr.base_decimals.into())
            / usdc(20000.) as u128) as u64;

        assert_eq!(open_fee, expected_open_fee);
        assert_eq!(collateral, expected_collateral);
        assert_eq!(size, expected_size);
        assert_eq!(borrow, expected_collateral * leverage);

        assert_eq!(short.size, expected_size);
        assert_eq!(short.average_price, usdc(20000.));
        assert_eq!(short.collateral, expected_collateral);
        assert_eq!(short.borrowed_amount, expected_collateral * leverage);
        assert_eq!(short.closing_size, 0);
        assert_eq!(short.cumulative_fund_fee, 0);

        const HOURS_2: u64 = 2;
        short.mock_after_hours(HOURS_2);

        // Short more
        short
            .open(usdc(20000.), usdc(2000.0), leverage as u32, &mfr)
            .assert_unwrap();
        assert_eq!(open_fee, expected_open_fee);
        assert_eq!(collateral, expected_collateral);
        assert_eq!(size, expected_size);
        assert_eq!(borrow, expected_collateral * leverage);

        assert_eq!(short.size, expected_size * 2);
        assert_eq!(short.average_price, usdc(20000.));
        assert_eq!(short.collateral, expected_collateral * 2);
        assert_eq!(short.borrowed_amount, expected_collateral * leverage * 2);
        assert_eq!(short.closing_size, 0);

        let expected_fund_fee =
            expected_collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2
                / FEE_RATE_BASE as u64;
        assert_eq!(short.cumulative_fund_fee, expected_fund_fee);
    }

    #[test]
    fn test_close_short_position_with_profit() {
        let mut dex = Dex::default();
        dex.mock_dex();
        let mfr = dex.markets[0].get_fee_rates(20);

        let mut short = Position::new(false).assert_unwrap();
        let leverage = 10u64;
        let (size, collateral, borrow, _) = short
            .open(usdc(20000.), usdc(2000.), leverage as u32, &mfr)
            .assert_unwrap();

        const HOURS_2: u64 = 2;
        short.mock_after_hours(HOURS_2);

        let (returned, collateral_unlocked, pnl, close_fee, borrow_fee) =
            short.close(size, usdc(18000.), &mfr, false).assert_unwrap();

        let expected_borrow_fee =
            collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2 / FEE_RATE_BASE as u64;

        let expected_close_fee =
            (size as u128) * (mfr.close_fee_rate as u128) * (usdc(18000.) as u128)
                / FEE_RATE_BASE as u128
                / 10u128.pow(mfr.base_decimals.into());

        let expected_pnl =
            size * (usdc(20000.) - usdc(18000.)) / 10u64.pow(mfr.base_decimals.into());

        assert_eq!(returned, borrow);
        assert_eq!(collateral_unlocked, collateral);
        assert_eq!(borrow_fee, expected_borrow_fee);
        assert_eq!(close_fee, expected_close_fee as u64);
        assert_eq!(pnl, expected_pnl as i64);
    }

    #[test]
    fn test_close_short_position_with_loss() {
        let mut dex = Dex::default();
        dex.mock_dex();
        let mfr = dex.markets[0].get_fee_rates(20);

        let mut short = Position::new(false).assert_unwrap();
        let leverage = 10u64;
        let (size, collateral, borrow, _) = short
            .open(usdc(20000.), usdc(2000.), leverage as u32, &mfr)
            .assert_unwrap();

        const HOURS_2: u64 = 2;
        short.mock_after_hours(HOURS_2);

        let (returned, collateral_unlocked, pnl, close_fee, borrow_fee) =
            short.close(size, usdc(22000.), &mfr, false).assert_unwrap();

        let expected_borrow_fee =
            collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2 / FEE_RATE_BASE as u64;

        let expected_close_fee =
            (size as u128) * (mfr.close_fee_rate as u128) * (usdc(22000.) as u128)
                / FEE_RATE_BASE as u128
                / 10u128.pow(mfr.base_decimals.into());

        let expected_pnl =
            size * (usdc(22000.) - usdc(20000.)) / 10u64.pow(mfr.base_decimals.into());

        assert_eq!(returned, borrow);
        assert_eq!(collateral_unlocked, collateral);
        assert_eq!(borrow_fee, expected_borrow_fee);
        assert_eq!(close_fee, expected_close_fee as u64);
        assert_eq!(pnl, -(expected_pnl as i64));
    }

    #[test]
    fn test_borrow_fund_insufficient_liquidity() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_btc_liquidity(btc(1.0));

        dex.assert_btc_liquidity(btc(1.0));

        dex.borrow_fund(0, true, btc(0.1), btc(1.1), btc(0.04))
            .assert_err();
    }

    #[test]
    fn test_borrow_fund_for_long_position() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_btc_liquidity(btc(1.0));

        dex.assert_btc_liquidity(btc(1.0));

        dex.borrow_fund(0, true, btc(0.1), btc(1.), btc(0.04))
            .assert_ok();

        dex.assert_btc_liquidity(0);
        dex.assert_btc_borrowed(btc(1.));
        dex.assert_btc_collateral(btc(0.1));
        dex.assert_btc_fee_amount(btc(0.04));
    }

    #[test]
    fn test_borrow_fund_for_short_position() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_usdc_liquidity(usdc(10000.0));

        dex.assert_usdc_liquidity(usdc(10000.0));

        dex.borrow_fund(0, false, usdc(1000.), usdc(10000.), usdc(20.))
            .assert_ok();

        dex.assert_usdc_liquidity(0);
        dex.assert_usdc_borrowed(usdc(10000.));
        dex.assert_usdc_collateral(usdc(1000.));
        dex.assert_usdc_fee_amount(usdc(20.));
    }

    #[test]
    fn test_settle_pnl_long_with_profit() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_btc_liquidity(btc(1.0));
        dex.borrow_fund(0, true, btc(0.1), btc(1.), btc(0.004))
            .assert_ok();

        let withdrawable = dex
            .settle_pnl(
                0,
                true,
                btc(0.1),
                btc(1.),
                btc_i(0.02),
                btc(0.002),
                btc(0.003),
            )
            .assert_unwrap();

        assert_eq!(withdrawable, btc(0.1 + 0.02 - 0.002 - 0.003));
        dex.assert_btc_borrowed(0);
        dex.assert_btc_collateral(0);
        dex.assert_btc_fee_amount(btc(0.004 + 0.002 + 0.003));
        dex.assert_btc_liquidity(btc(1.0 - 0.02));
    }

    #[test]
    fn test_settle_pnl_long_with_loss() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_btc_liquidity(btc(1.0));
        dex.borrow_fund(0, true, btc(0.1), btc(1.), btc(0.004))
            .assert_ok();

        let withdrawable = dex
            .settle_pnl(
                0,
                true,
                btc(0.1),
                btc(1.),
                btc_i(-0.02),
                btc(0.002),
                btc(0.003),
            )
            .assert_unwrap();

        assert_eq!(withdrawable, btc(0.1 - 0.02 - 0.002 - 0.003));
        dex.assert_btc_borrowed(0);
        dex.assert_btc_collateral(0);
        dex.assert_btc_fee_amount(btc(0.004 + 0.002 + 0.003));
        dex.assert_btc_liquidity(btc(1.0 + 0.02));
    }

    #[test]
    fn test_settle_pnl_long_with_liquidation() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_btc_liquidity(btc(1.0));
        dex.borrow_fund(0, true, btc(0.1), btc(1.), btc(0.004))
            .assert_ok();

        let withdrawable = dex
            .settle_pnl(
                0,
                true,
                btc(0.1),
                btc(1.),
                btc_i(-0.098),
                btc(0.002),
                btc(0.003),
            )
            .assert_unwrap();

        assert_eq!(withdrawable, 0);
        dex.assert_btc_borrowed(0);
        dex.assert_btc_collateral(0);
        dex.assert_btc_fee_amount(btc(0.004 + 0.002 + 0.003));

        let _user_paid_fee = 0.002;
        let actual_pool_pnl = 0.098 - 0.003;
        dex.assert_btc_liquidity(btc(1.0 + actual_pool_pnl));
    }

    #[test]
    fn test_settle_pnl_short_with_profit() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_usdc_liquidity(usdc(10000.0));
        dex.borrow_fund(0, false, usdc(1000.), usdc(10000.), usdc(20.))
            .assert_ok();

        let withdrawable = dex
            .settle_pnl(
                0,
                false,
                usdc(1000.),
                usdc(10000.),
                usdc_i(500.),
                usdc(25.),
                usdc(35.),
            )
            .assert_unwrap();

        assert_eq!(withdrawable, usdc(1000. + 500. - 25. - 35.));
        dex.assert_usdc_borrowed(0);
        dex.assert_usdc_collateral(0);
        dex.assert_usdc_fee_amount(usdc(20. + 25. + 35.));
        dex.assert_usdc_liquidity(usdc(10000. - 500.));
    }

    #[test]
    fn test_settle_pnl_short_with_loss() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_usdc_liquidity(usdc(10000.0));
        dex.borrow_fund(0, false, usdc(1000.), usdc(10000.), usdc(20.))
            .assert_ok();

        let withdrawable = dex
            .settle_pnl(
                0,
                false,
                usdc(1000.),
                usdc(10000.),
                usdc_i(-500.),
                usdc(25.),
                usdc(35.),
            )
            .assert_unwrap();

        assert_eq!(withdrawable, usdc(1000. - 500. - 25. - 35.));
        dex.assert_usdc_borrowed(0);
        dex.assert_usdc_collateral(0);
        dex.assert_usdc_fee_amount(usdc(20. + 25. + 35.));
        dex.assert_usdc_liquidity(usdc(10000. + 500.));
    }

    #[test]
    fn test_settle_pnl_short_with_liquidation() {
        let mut dex = Dex::default();
        dex.mock_dex();
        dex.mock_usdc_liquidity(usdc(10000.0));
        dex.borrow_fund(0, false, usdc(1000.), usdc(10000.), usdc(20.))
            .assert_ok();

        let withdrawable = dex
            .settle_pnl(
                0,
                false,
                usdc(1000.),
                usdc(10000.),
                usdc_i(-980.),
                usdc(25.),
                usdc(35.),
            )
            .assert_unwrap();

        assert_eq!(withdrawable, 0);
        dex.assert_usdc_borrowed(0);
        dex.assert_usdc_collateral(0);
        dex.assert_usdc_fee_amount(usdc(20. + 25. + 35.));

        let _user_paid_fee = usdc(20.);
        let actual_pool_pnl = 980. - 25. - 35. + 20.;
        dex.assert_usdc_liquidity(usdc(10000. + actual_pool_pnl));
    }
}
