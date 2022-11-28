use anchor_lang::prelude::*;

use crate::{
    errors::{DexError, DexResult},
    utils::{
        swap, time::get_timestamp, value, ISafeAddSub, ISafeMath, SafeMath, FEE_RATE_BASE,
        FEE_RATE_DECIMALS, USD_POW_DECIMALS,
    },
};

use super::{get_oracle_price, StakingPool};

#[account(zero_copy)]
pub struct Dex {
    pub magic: u64,
    pub assets: [AssetInfo; 16],
    pub markets: [MarketInfo; 16],
    pub vlp_pool: StakingPool,
    pub authority: Pubkey,
    pub event_queue: Pubkey,
    pub match_queue: Pubkey,
    pub usdc_mint: Pubkey,
    pub user_list_entry_page: Pubkey,
    pub user_list_remaining_pages: [Pubkey; 8],
    pub user_list_remaining_pages_number: u8,
    pub assets_number: u8,
    pub markets_number: u8,
    pub usdc_asset_index: u8,
    pub padding: [u8; 252],
}

impl Dex {
    pub fn asset_as_ref(&self, index: u8) -> DexResult<&AssetInfo> {
        require!(
            index < self.assets_number && self.assets[index as usize].valid,
            DexError::InvalidAssetIndex
        );

        Ok(&self.assets[index as usize])
    }

    fn asset_as_mut(&mut self, index: u8) -> DexResult<&mut AssetInfo> {
        require!(
            index < self.assets_number && self.assets[index as usize].valid,
            DexError::InvalidAssetIndex
        );

        Ok(&mut self.assets[index as usize])
    }

    fn position_as_mut(&mut self, market: u8, long: bool) -> DexResult<&mut Position> {
        require!(market < self.markets_number, DexError::InvalidMarketIndex);

        Ok(if long {
            &mut self.markets[market as usize].global_long
        } else {
            &mut self.markets[market as usize].global_short
        })
    }

    fn vlp_info(&self) -> DexResult<(u64, u8)> {
        // If VLP is dummy token (not minted), then total supply can be read from vlp_pool.staked_total.
        // Otherwise should read the token's total supply

        Ok((self.vlp_pool.staked_total, self.vlp_pool.decimals))
    }

    pub fn market_asset(&mut self, market: u8, long: bool) -> DexResult<&mut AssetInfo> {
        require!(market < self.markets_number, DexError::InvalidMarketIndex);

        let mi = &mut self.markets[market as usize];
        require!(mi.valid, DexError::InvalidMarketIndex);

        let index = if long {
            mi.asset_index
        } else {
            self.usdc_asset_index
        };

        self.asset_as_mut(index)
    }

    #[cfg(feature = "client-support")]
    pub fn market_asset_as_ref(&self, market: u8, long: bool) -> DexResult<&AssetInfo> {
        require!(market < self.markets_number, DexError::InvalidMarketIndex);

        let mi = &self.markets[market as usize];
        require!(mi.valid, DexError::InvalidMarketIndex);

        let index = if long {
            mi.asset_index
        } else {
            self.usdc_asset_index
        };

        self.asset_as_ref(index)
    }

    pub fn find_asset_by_mint(&self, mint: Pubkey) -> DexResult<(u8, &AssetInfo)> {
        let index = self
            .assets
            .iter()
            .position(|x| x.mint == mint)
            .ok_or(DexError::InvalidMint)? as u8;

        Ok((index, &self.assets[index as usize]))
    }

    fn aum(&self, oracles: &[AccountInfo]) -> DexResult<i64> {
        let mut aum = 0u64;

        let mut oracle_offset = 0;
        for i in 0..self.assets_number as usize {
            let ai = &self.assets[i];
            if !ai.valid {
                continue;
            }

            require!(oracle_offset < oracles.len(), DexError::InvalidOracle);

            require_eq!(
                ai.oracle,
                oracles[oracle_offset].key(),
                DexError::InvalidOracle
            );

            let price = get_oracle_price(ai.oracle_source, &oracles[oracle_offset])?;

            let amount = ai
                .liquidity_amount
                .safe_add(ai.collateral_amount)?
                .safe_add(ai.borrowed_amount)?;

            aum = aum.safe_add(
                amount
                    .safe_mul(price.into())?
                    .safe_div(10u128.pow(ai.decimals.into()))? as u64,
            )?;

            oracle_offset += 1;
        }

        let mut pnl = 0i64;
        for index in 0..self.markets_number as usize {
            let mi = &self.markets[index];
            if !mi.valid {
                continue;
            }

            require!(oracle_offset < oracles.len(), DexError::InvalidOracle);

            require_eq!(
                mi.oracle,
                oracles[oracle_offset].key(),
                DexError::InvalidOracle
            );

            let price = get_oracle_price(mi.oracle_source, &oracles[oracle_offset])?;
            pnl = pnl.i_safe_add(mi.un_pnl(price)?)?;

            oracle_offset += 1;
        }

        (aum as i64).i_safe_sub(pnl)
    }

    fn to_oracle_index(&self, index: u8) -> DexResult<usize> {
        require!(index < self.assets_number, DexError::InvalidAssetIndex);

        let mut oracle_index = 0usize;
        for i in 0..index as usize {
            if self.assets[i].valid {
                oracle_index += 1;
            }
        }

        Ok(oracle_index)
    }

    pub fn add_liquidity(
        &mut self,
        index: u8,
        amount: u64,
        oracles: &[AccountInfo],
    ) -> DexResult<(u64, u64)> {
        require!(amount > 0, DexError::InvalidAmount);

        let aum = self.aum(oracles)?;
        require!(aum >= 0, DexError::AUMBelowZero);

        let (vlp_supply, vlp_decimals) = self.vlp_info()?;
        let oracle_index = self.to_oracle_index(index)?;

        let ai = self.asset_as_mut(index)?;

        let fee = amount
            .safe_mul(ai.add_liquidity_fee_rate as u64)?
            .safe_div(FEE_RATE_BASE)? as u64;

        let added = amount.safe_sub(fee)?;
        ai.liquidity_amount = ai.liquidity_amount.safe_add(added)?;
        ai.fee_amount = ai.fee_amount.safe_add(fee)?;

        require!(
            ai.oracle == oracles[oracle_index].key(),
            DexError::InvalidOracle
        );

        // vlp_amount = asset_value * vlp_supply / aum
        let price = get_oracle_price(ai.oracle_source, &oracles[oracle_index])?;
        let asset_value = value(added, price, ai.decimals)?;

        let vlp_amount = if aum == 0 {
            asset_value
                .safe_mul(10u64.pow(vlp_decimals.into()))?
                .safe_div(USD_POW_DECIMALS as u128)? as u64
        } else {
            asset_value.safe_mul(vlp_supply)?.safe_div(aum as u128)? as u64
        };

        Ok((vlp_amount, fee))
    }

    pub fn remove_liquidity(
        &mut self,
        index: u8,
        amount: u64,
        oracles: &[AccountInfo],
    ) -> DexResult<(u64, u64)> {
        require!(amount > 0, DexError::InvalidAmount);

        let aum = self.aum(oracles)?;
        require!(aum >= 0, DexError::AUMBelowZero);

        let (vlp_supply, vlp_decimals) = self.vlp_info()?;
        require!(vlp_supply > 0, DexError::VLPSupplyZero);
        let oracle_index = self.to_oracle_index(index)?;

        let ai = self.asset_as_mut(index)?;
        require!(
            ai.oracle == oracles[oracle_index].key(),
            DexError::InvalidOracle
        );

        let asset_price = get_oracle_price(ai.oracle_source, &oracles[oracle_index])?;
        // vlp_price= aum / vlp_supply
        let vlp_price = (aum as u64)
            .safe_mul(10u64.pow(vlp_decimals.into()))?
            .safe_div(vlp_supply as u128)? as u64;

        let out_amount = swap(amount, vlp_price, vlp_decimals, asset_price, ai.decimals)?;

        let fee = out_amount
            .safe_mul(ai.remove_liquidity_fee_rate as u64)?
            .safe_div(FEE_RATE_BASE)? as u64;

        ai.liquidity_amount = ai
            .liquidity_amount
            .safe_sub(out_amount)
            .map_err(|_| DexError::InsufficientLiquidity)?;
        ai.fee_amount = ai.fee_amount.safe_add(fee)?;

        Ok((out_amount.safe_sub(fee)?, fee))
    }

    pub fn has_sufficient_fund(&mut self, market: u8, long: bool, borrow: u64) -> DexResult {
        let ai = self.market_asset(market, long)?;
        if ai.liquidity_amount > borrow {
            Ok(())
        } else {
            Err(error!(DexError::InsufficientLiquidity))
        }
    }

    pub fn borrow_fund(
        &mut self,
        market: u8,
        long: bool,
        collateral: u64,
        borrow: u64,
        open_fee: u64,
    ) -> DexResult {
        let ai = self.market_asset(market, long)?;

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
        market: u8,
        long: bool,
        collateral: u64,
        borrow: u64,
        pnl: i64,
        close_fee: u64,
        borrow_fee: u64,
    ) -> DexResult<u64> {
        let ai = self.market_asset(market, long)?;

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
        market: u8,
        long: bool,
        price: u64,
        size: u64,
        collateral: u64,
    ) -> DexResult {
        let pos = self.position_as_mut(market, long)?;

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
        market: u8,
        long: bool,
        size: u64,
        collateral: u64,
    ) -> DexResult {
        let pos = self.position_as_mut(market, long)?;

        pos.collateral = pos.collateral.safe_sub(collateral)?;
        pos.size = pos.size.safe_sub(size)?;
        pos.last_fill_time = get_timestamp()?;

        if pos.size == 0 {
            pos.zero(long)?;
        }

        Ok(())
    }

    pub fn swap(
        &self,
        ain: u8,
        aout: u8,
        amount: u64,
        charge: bool,
        oracles: &[&AccountInfo],
    ) -> DexResult<(u64, u64)> {
        // TODO: check minimum amount?

        require!(ain != aout, DexError::InvalidAssetIndex);
        require!(amount > 0, DexError::InvalidAmount);

        let aii = self.asset_as_ref(ain)?;
        let aoi = self.asset_as_ref(aout)?;

        require!(
            aii.oracle == oracles[0].key() && aoi.oracle == oracles[1].key(),
            DexError::InvalidOracle
        );

        let in_price = get_oracle_price(aii.oracle_source, &oracles[0])?;
        let out_price = get_oracle_price(aoi.oracle_source, &oracles[1])?;

        let fee = if charge {
            amount
                .safe_mul(aii.swap_fee_rate.into())?
                .safe_div(FEE_RATE_BASE)? as u64
        } else {
            0
        };
        let in_amount = amount.safe_sub(fee)?;
        let out = swap(in_amount, in_price, aii.decimals, out_price, aoi.decimals)?;

        Ok((out, fee))
    }

    fn collect_fees(&mut self, reward_asset: usize, oracles: &[AccountInfo]) -> DexResult<u64> {
        let rai = self.asset_as_ref(reward_asset as u8)?;
        let reward_oracle_index = self.to_oracle_index(reward_asset as u8)?;

        require!(reward_oracle_index < oracles.len(), DexError::InvalidOracle);

        require_eq!(
            rai.oracle,
            oracles[reward_oracle_index].key(),
            DexError::InvalidOracle
        );

        require_eq!(
            oracles.len(),
            self.assets_number as usize,
            DexError::InvalidOracle
        );

        let mut oracle_offset = 0usize;
        for i in 0..self.assets_number as usize {
            let ai = self.asset_as_ref(i as u8)?;
            if !ai.valid {
                continue;
            }

            if i == reward_asset as usize || ai.fee_amount == 0 {
                oracle_offset += 1;
                continue;
            }

            require!(oracle_offset < oracles.len(), DexError::InvalidOracle);

            require_eq!(
                ai.oracle,
                oracles[oracle_offset].key(),
                DexError::InvalidOracle
            );

            let swap_oracles: Vec<&AccountInfo> =
                vec![&oracles[oracle_offset], &oracles[reward_oracle_index]];

            // TODO: define DUST fee amount which should be ignored?
            let (collected, _) = self.swap(
                i as u8,
                reward_asset as u8,
                ai.fee_amount,
                false,
                &swap_oracles,
            )?;

            self.assets[i].liquidity_amount = self.assets[i]
                .liquidity_amount
                .safe_add(self.assets[i].fee_amount)?;
            self.assets[i].fee_amount = 0;

            self.assets[reward_asset].liquidity_amount = self.assets[reward_asset]
                .liquidity_amount
                .safe_sub(collected)?;
            self.assets[reward_asset].fee_amount =
                self.assets[reward_asset].fee_amount.safe_add(collected)?;

            oracle_offset += 1;
        }

        Ok(self.assets[reward_asset as usize].fee_amount)
    }

    pub fn collect_rewards(&mut self, oracles: &[AccountInfo]) -> DexResult {
        let index = self.vlp_pool.reward_asset_index;
        require!(
            index < self.assets_number && self.assets[index as usize].valid,
            DexError::InvalidRewardAsset
        );

        let rewards = self.collect_fees(index as usize, oracles)?;
        if rewards == 0 {
            return Ok(());
        }

        self.vlp_pool.add_reward(rewards)?;

        let ai = self.asset_as_mut(index)?;
        ai.fee_amount = 0;

        Ok(())
    }

    pub fn swap_in(&mut self, index: u8, amount: u64, fee: u64) -> DexResult {
        let ai = self.asset_as_mut(index)?;

        ai.liquidity_amount = ai.liquidity_amount.safe_add(amount)?;
        ai.fee_amount = ai.fee_amount.safe_add(fee)?;

        Ok(())
    }

    pub fn swap_out(&mut self, index: u8, amount: u64) -> DexResult {
        let ai = self.asset_as_mut(index)?;

        ai.liquidity_amount = ai.liquidity_amount.safe_sub(amount)?;

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
    pub swap_fee_rate: u16,
    pub borrow_fee_rate: u16,
    pub add_liquidity_fee_rate: u16,
    pub remove_liquidity_fee_rate: u16,
    pub target_weight: u16,
    pub valid: bool,
    pub decimals: u8,
    pub nonce: u8,
    pub oracle_source: u8,
    pub padding: [u8; 250],
}

#[zero_copy]
pub struct MarketInfo {
    pub symbol: [u8; 16],
    pub oracle: Pubkey,

    pub order_book: Pubkey,

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
    pub order_pool_remaining_pages_number: u8,
    pub padding: [u8; 252],
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

    pub fn size(
        long: bool,
        price: u64,
        amount: u64,
        leverage: u32,
        mfr: &MarketFeeRates,
    ) -> DexResult<u64> {
        let (collateral, _) =
            Position::calc_collateral_and_fee(amount, leverage, mfr.open_fee_rate)?;

        let size = if long {
            collateral.safe_mul(leverage as u64)
        } else {
            collateral
                .safe_mul(leverage as u64)?
                .safe_mul(10u128.pow(mfr.base_decimals.into()))?
                .safe_div(price as u128)
        }? as u64;

        Ok(size)
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
            require!(self.last_fill_time <= now, DexError::InvalidPositionTime);

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
        let unclosing_size = self.size.safe_sub(self.closing_size)?;
        require!(unclosing_size >= size, DexError::CloseSizeTooLarge);

        let mut collateral_unlocked = size
            .safe_mul(self.collateral)?
            .safe_div(self.size as u128)? as u64;

        let mut fund_returned = size
            .safe_mul(self.borrowed_amount)?
            .safe_div(self.size as u128)? as u64;

        // Update cumulative fund fee
        let now = get_timestamp()?;
        let borrow_fee = if self.borrowed_amount > 0 {
            require!(self.last_fill_time <= now, DexError::InvalidPositionTime);

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
        self.size = unclosing_size.safe_sub(size)?.safe_add(self.closing_size)?;

        self.borrowed_amount = self.borrowed_amount.safe_sub(fund_returned)?;
        self.collateral = self.collateral.safe_sub(collateral_unlocked)?;
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

    pub fn sub_closing(&mut self, closing_size: u64) -> DexResult {
        self.closing_size = self.closing_size.safe_sub(closing_size)?;
        Ok(())
    }

    pub fn add_closing(&mut self, closing_size: u64) -> DexResult {
        self.closing_size = self.closing_size.safe_add(closing_size)?;
        require!(self.closing_size <= self.size, DexError::AskSizeTooLarge);

        Ok(())
    }

    pub fn unclosing_size(&self) -> DexResult<u64> {
        self.size.safe_sub(self.closing_size)
    }

    pub fn calc_collateral_and_fee(amount: u64, leverage: u32, rate: u16) -> DexResult<(u64, u64)> {
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

#[account]
#[repr(C)]
pub struct MockOracle {
    pub magic: u64,
    pub price: u64,
    pub expo: u8,
    pub padding: [u8; 7],
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
    use crate::{
        dex::{set_mock_price, OracleSource},
        utils::test::*,
    };
    use bumpalo::Bump;

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

        pub fn add_asset(&mut self, decimals: u8, oracle: Pubkey) {
            self.assets[self.assets_number as usize] = AssetInfo {
                valid: true,
                decimals,
                borrow_fee_rate: 10,
                oracle_source: OracleSource::Mock as u8,
                oracle,
                add_liquidity_fee_rate: 10,
                remove_liquidity_fee_rate: 10,
                ..AssetInfo::default()
            };
            self.assets_number += 1;
        }

        pub fn mock_invalid_asset(&mut self, decimals: u8, oracle: Pubkey) {
            self.assets[self.assets_number as usize] = AssetInfo {
                valid: false,
                decimals,
                borrow_fee_rate: 10,
                oracle_source: OracleSource::Mock as u8,
                oracle,
                ..AssetInfo::default()
            };
            self.assets_number += 1;
        }

        pub fn add_market(&mut self, decimals: u8, asset_index: u8, oracle: Pubkey) {
            self.markets[self.markets_number as usize] = MarketInfo {
                valid: true,
                decimals,
                oracle,
                asset_index,
                open_fee_rate: 20,
                close_fee_rate: 20,
                liquidate_fee_rate: 50,
                charge_borrow_fee_interval: 3600,
                global_long: Position::new(true).assert_unwrap(),
                global_short: Position::new(false).assert_unwrap(),
                ..MarketInfo::default()
            };

            self.markets_number += 1;
        }
        pub fn mock_vlp_pool(&mut self, decimals: u8, reward_asset_index: u8) {
            self.vlp_pool = StakingPool {
                decimals,
                reward_asset_index,
                ..StakingPool::default()
            };
        }

        pub fn mock_asset_liquidity(&mut self, index: u8, amount: u64) {
            self.assets[index as usize].liquidity_amount = amount;
        }

        pub fn add_asset_collateral(&mut self, index: u8, amount: u64) {
            self.assets[index as usize].collateral_amount += amount;
        }

        pub fn mock_asset_borrowed(&mut self, index: u8, amount: u64) {
            self.assets[index as usize].borrowed_amount = amount;
        }

        pub fn mock_asset_fee(&mut self, index: u8, amount: u64) {
            self.assets[index as usize].fee_amount = amount;
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

        pub fn assert_btc_fee(&self, amount: u64) {
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

        pub fn assert_usdc_fee(&self, amount: u64) {
            assert_eq!(self.assets[1].fee_amount, amount)
        }

        pub fn assert_asset_fee(&self, index: u8, amount: u64) {
            assert_eq!(self.assets[index as usize].fee_amount, amount)
        }

        pub fn assert_asset_liquidity(&self, index: u8, amount: u64) {
            assert_eq!(self.assets[index as usize].liquidity_amount, amount)
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
        dex.assert_btc_fee(btc(0.04));
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
        dex.assert_usdc_fee(usdc(20.));
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
        dex.assert_btc_fee(btc(0.004 + 0.002 + 0.003));
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
        dex.assert_btc_fee(btc(0.004 + 0.002 + 0.003));
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
        dex.assert_btc_fee(btc(0.004 + 0.002 + 0.003));

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
        dex.assert_usdc_fee(usdc(20. + 25. + 35.));
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
        dex.assert_usdc_fee(usdc(20. + 25. + 35.));
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
        dex.assert_usdc_fee(usdc(20. + 25. + 35.));

        let _user_paid_fee = usdc(20.);
        let actual_pool_pnl = 980. - 25. - 35. + 20.;
        dex.assert_usdc_liquidity(usdc(10000. + actual_pool_pnl));
    }

    #[test]
    fn test_swap_with_invalid_input() {
        let bump = Bump::new();
        let btc_oracle = gen_account(1024, &bump);
        let usdc_oracle = gen_account(1024, &bump);
        let dummy_oracle = gen_account(1024, &bump);

        let mut dex = Dex::default();
        dex.add_asset(BTC_DECIMALS, btc_oracle.key());
        dex.add_asset(USDC_DECIMALS, usdc_oracle.key());

        let oracles: Vec<&AccountInfo> = vec![&usdc_oracle, &btc_oracle];

        set_mock_price(&btc_oracle, usdc(20000.)).assert_ok();
        set_mock_price(&usdc_oracle, usdc(1.)).assert_ok();

        // Invalid asset index
        dex.swap(1, 2, usdc(0.1), false, &oracles).assert_err();
        dex.swap(1, 1, usdc(0.1), false, &oracles).assert_err();

        // Invalid amount
        dex.swap(1, 0, usdc(0.), false, &oracles).assert_err();

        // Invalid oracle
        dex.swap(1, 0, usdc(0.1), false, &oracles).assert_ok();
        let wrong_oracles: Vec<&AccountInfo> = vec![&usdc_oracle, &dummy_oracle];
        dex.swap(1, 0, usdc(0.1), false, &wrong_oracles)
            .assert_err();
    }

    #[test]
    fn test_swap_btc_with_usdc_no_fee() {
        let bump = Bump::new();
        let btc_oracle = gen_account(1024, &bump);
        let usdc_oracle = gen_account(1024, &bump);

        let mut dex = Dex::default();
        dex.add_asset(BTC_DECIMALS, btc_oracle.key());
        dex.add_asset(USDC_DECIMALS, usdc_oracle.key());

        let oracles: Vec<&AccountInfo> = vec![&btc_oracle, &usdc_oracle];

        set_mock_price(&btc_oracle, usdc(20000.)).assert_ok();
        set_mock_price(&usdc_oracle, usdc(1.)).assert_ok();

        let (out, fee) = dex.swap(0, 1, btc(1.0), false, &oracles).assert_unwrap();

        assert_eq!(out, usdc(20000.));
        assert_eq!(fee, 0);

        let oracles: Vec<&AccountInfo> = vec![&usdc_oracle, &btc_oracle];
        let (out, fee) = dex.swap(1, 0, usdc(0.1), false, &oracles).assert_unwrap();

        assert_eq!(out, btc(0.000005));
        assert_eq!(fee, 0);
    }

    #[test]
    fn test_collect_fees() {
        let bump = Bump::new();
        let btc_oracle = gen_account(1024, &bump);
        let usdc_oracle = gen_account(1024, &bump);
        let sol_oracle = gen_account(1024, &bump);

        let mut dex = Dex::default();
        dex.add_asset(BTC_DECIMALS, btc_oracle.key());
        dex.add_asset(USDC_DECIMALS, usdc_oracle.key());
        dex.add_asset(SOL_DECIMALS, sol_oracle.key());

        set_mock_price(&btc_oracle, usdc(20000.)).assert_ok();
        set_mock_price(&usdc_oracle, usdc(1.)).assert_ok();
        set_mock_price(&sol_oracle, usdc(20.)).assert_ok();

        let oracles: Vec<AccountInfo> = vec![btc_oracle, usdc_oracle, sol_oracle];

        dex.collect_fees(2, &oracles).assert_ok();

        dex.assert_asset_fee(0, 0);
        dex.assert_asset_fee(1, 0);
        dex.assert_asset_fee(2, 0);

        // Mock fee for each asset

        dex.mock_asset_liquidity(0, 0);
        dex.mock_asset_liquidity(1, 0);
        dex.mock_asset_liquidity(2, sol(30.)); // Fee of btc&usdc will be converted to sol

        dex.mock_asset_fee(0, btc(0.01));
        dex.mock_asset_fee(1, usdc(100.));
        dex.mock_asset_fee(2, sol(3.));

        dex.collect_fees(2, &oracles).assert_ok();
        dex.assert_asset_fee(0, 0);
        dex.assert_asset_fee(1, 0);
        dex.assert_asset_fee(2, sol(10.0 + 5.0 + 3.0));

        // Check liquidity
        dex.assert_asset_liquidity(0, btc(0.01));
        dex.assert_asset_liquidity(1, usdc(100.));
        dex.assert_asset_liquidity(2, sol(15.));
    }

    #[test]
    fn test_aum() {
        let bump = Bump::new();
        let btc_oracle = gen_account(1024, &bump);
        let btc_oracle2 = gen_account(1024, &bump);
        let usdc_oracle = gen_account(1024, &bump);
        let sol_oracle = gen_account(1024, &bump);
        let dummy_oracle = gen_account(1024, &bump);

        // Mock dex with 4 assets and 1 market
        let mut dex = Dex::default();
        dex.mock_invalid_asset(6, dummy_oracle.key()); // Mock invalid asset
        dex.add_asset(BTC_DECIMALS, btc_oracle.key());
        dex.add_asset(USDC_DECIMALS, usdc_oracle.key());
        dex.add_asset(SOL_DECIMALS, sol_oracle.key());

        dex.add_market(BTC_DECIMALS, 1, btc_oracle2.key());

        set_mock_price(&btc_oracle, usdc(20000.)).assert_ok();
        set_mock_price(&btc_oracle2, usdc(20000.)).assert_ok();
        set_mock_price(&usdc_oracle, usdc(1.)).assert_ok();
        set_mock_price(&sol_oracle, usdc(20.)).assert_ok();

        let mut oracles: Vec<AccountInfo> = vec![btc_oracle, usdc_oracle, sol_oracle, btc_oracle2];

        let aum = dex.aum(&oracles).assert_unwrap();
        assert_eq!(aum, 0);

        // Add liquidity
        dex.mock_asset_liquidity(1, btc(0.04));
        dex.mock_asset_liquidity(2, usdc(500.));
        dex.mock_asset_liquidity(3, sol(30.));

        let aum = dex.aum(&oracles).assert_unwrap();
        let mut liquidity = usdc_i(800.0 + 500. + 600.0);
        assert_eq!(aum, liquidity);

        // Mock borrow amount
        dex.mock_asset_borrowed(1, btc(0.005));
        dex.mock_asset_borrowed(2, usdc(110.));
        dex.mock_asset_borrowed(3, sol(3.));

        let aum = dex.aum(&oracles).assert_unwrap();
        let mut borrowed = usdc_i(100.0 + 110.0 + 60.0);
        assert_eq!(aum, liquidity + borrowed);

        let mut collateral = 0i64;

        // Mock global long
        dex.increase_global_position(0, true, usdc(18000.), btc(0.01), btc(0.001))
            .assert_ok();
        dex.add_asset_collateral(1, btc(0.001));

        liquidity -= usdc_i(200.0);
        collateral += usdc_i(20.0);
        borrowed += usdc_i(200.0);
        let long_un_pnl = usdc_i(20.0);

        let aum = dex.aum(&oracles).assert_unwrap();
        assert_eq!(aum, liquidity + collateral + borrowed - long_un_pnl);

        // Mock global short
        dex.increase_global_position(0, false, usdc(19000.), btc(0.01), btc(0.001))
            .assert_ok();
        dex.add_asset_collateral(1, btc(0.001));

        liquidity -= usdc_i(200.0);
        collateral += usdc_i(20.0);
        borrowed += usdc_i(200.0);
        let short_un_pnl = -usdc_i(10.0);

        let aum = dex.aum(&oracles).assert_unwrap();
        assert_eq!(
            aum,
            liquidity + collateral + borrowed - long_un_pnl - short_un_pnl
        );

        // Test invalid oracles
        dex.aum(&vec![]).assert_err();
        oracles.pop();
        dex.aum(&oracles).assert_err()
    }

    const VLP_DECIMALS: u8 = 8;
    fn vlp(size: f64) -> u64 {
        (size * (10u64.pow(VLP_DECIMALS as u32) as f64)) as u64
    }

    #[test]
    fn test_add_and_remove_liquidity() {
        let bump = Bump::new();
        let btc_oracle = gen_account(1024, &bump);
        let btc_oracle2 = gen_account(1024, &bump);
        let usdc_oracle = gen_account(1024, &bump);
        let sol_oracle = gen_account(1024, &bump);
        let dummy_oracle = gen_account(1024, &bump);

        // Mock dex with 4 assets and 1 market
        let mut dex = Dex::default();
        dex.mock_invalid_asset(6, dummy_oracle.key()); // Mock invalid asset
        dex.add_asset(BTC_DECIMALS, btc_oracle.key());
        dex.add_asset(USDC_DECIMALS, usdc_oracle.key());
        dex.add_asset(SOL_DECIMALS, sol_oracle.key());

        dex.mock_vlp_pool(VLP_DECIMALS, 3);

        dex.add_market(BTC_DECIMALS, 1, btc_oracle2.key());

        set_mock_price(&btc_oracle, usdc(20000.)).assert_ok();
        set_mock_price(&btc_oracle2, usdc(20000.)).assert_ok();
        set_mock_price(&usdc_oracle, usdc(1.)).assert_ok();
        set_mock_price(&sol_oracle, usdc(20.)).assert_ok();

        let oracles: Vec<AccountInfo> = vec![btc_oracle, usdc_oracle, sol_oracle, btc_oracle2];

        // Add 1000 SOL, add liquidity fee rate = 0.1%
        let (vlp_amount, fee) = dex.add_liquidity(3, sol(1000.), &oracles).assert_unwrap();

        // vlp_amount = 19980_00000000
        assert_eq!(vlp_amount, vlp((1000.0 - 1.0) * 20.0));
        assert_eq!(fee, sol(1.));
        dex.assert_asset_fee(3, sol(1.));
        dex.vlp_pool.increase_staking(vlp_amount).assert_ok();

        // Add another 1000 SOL
        let (vlp_amount, fee) = dex.add_liquidity(3, sol(1000.), &oracles).assert_unwrap();

        // vlp_amount = 19980_00000000
        assert_eq!(vlp_amount, vlp((1000.0 - 1.0) * 20.0));

        assert_eq!(fee, sol(1.));
        dex.assert_asset_fee(3, sol(2.));
        dex.vlp_pool.increase_staking(vlp_amount).assert_ok();

        assert_eq!(dex.vlp_pool.staked_total, 2 * vlp((1000.0 - 1.0) * 20.0));

        // Remove liquidity, remove liquidity fee rate = 0.1%
        let (withdraw, fee) = dex
            .remove_liquidity(3, vlp(1000.), &oracles)
            .assert_unwrap();

        assert_eq!(fee, sol(0.05));
        assert_eq!(withdraw, sol(49.95));

        dex.vlp_pool.decrease_staking(vlp(1000.)).assert_ok();
        assert_eq!(
            dex.vlp_pool.staked_total,
            2 * vlp((1000.0 - 1.0) * 20.0) - vlp(1000.)
        );

        // Add 1 BTC
        let (vlp_amount, fee) = dex.add_liquidity(1, btc(1.), &oracles).assert_unwrap();
        assert_eq!(vlp_amount, vlp((1.0 - 0.001) * 20000.0));
        assert_eq!(fee, btc(0.001));
        dex.vlp_pool.increase_staking(vlp_amount).assert_ok();

        // Add 10000 usdc
        let (vlp_amount, fee) = dex.add_liquidity(2, usdc(10000.), &oracles).assert_unwrap();
        assert_eq!(vlp_amount, vlp((10000.0 - 10.) * 1.0));
        assert_eq!(fee, usdc(10.));
        dex.vlp_pool.increase_staking(vlp_amount).assert_ok();
    }
}
