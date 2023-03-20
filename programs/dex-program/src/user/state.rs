use std::cell::{RefCell, RefMut};

use crate::collections::small_list::*;
use crate::dex::{state::*, StakingPool, UserStake};
use crate::dual_invest::DIOption;
use crate::errors::{DexError, DexResult};

use crate::utils::{time::get_timestamp, SafeMath, NIL32, USER_STATE_MAGIC_NUMBER};
use anchor_lang::prelude::*;
use std::mem;

#[cfg(feature = "client-support")]
use std::mem::ManuallyDrop;

#[repr(C)]
// #[derive(Clone, Copy)]
pub struct MetaInfo {
    pub magic: u32,
    pub serial_number: u32,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub vlp: UserStake,
    pub user_list_index: u32,
    pub order_slot_count: u8,
    pub position_slot_count: u8,
    pub di_option_slot_count: u8,
    pub asset_slot_count: u8,
    reserved: [u8; 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserOrder {
    pub list_time: i64,
    pub size: u64, // Refer to collateral size for opening position
    pub price: u64,
    pub loss_stop_price: u64,
    pub profit_stop_price: u64,
    pub order_slot: u32,
    pub leverage: u32,
    pub long: bool,
    pub open: bool,
    pub asset: u8,
    pub market: u8,
    padding: [u8; 20],
}

impl UserOrder {
    pub fn init_as_bid(
        &mut self,
        order_slot: u32,
        size: u64,
        price: u64,
        leverage: u32,
        long: bool,
        market: u8,
        asset: u8,
    ) -> DexResult {
        self.order_slot = order_slot;
        self.size = size;
        self.price = price;
        self.leverage = leverage;
        self.long = long;
        self.market = market;
        self.asset = asset;
        self.open = true;

        self.list_time = get_timestamp()?;
        Ok(())
    }

    pub fn init_as_ask(&mut self, size: u64, price: u64, long: bool, market: u8) -> DexResult {
        self.size = size;
        self.price = price;
        self.long = long;
        self.market = market;
        self.open = false;

        self.list_time = get_timestamp()?;
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserPosition {
    pub long: Position,
    pub short: Position,
    pub market: u8,
    padding: [u8; 23],
}

impl UserPosition {
    pub fn init(&mut self, market: u8) -> DexResult {
        self.market = market;

        self.long.zero(true)?;
        self.short.zero(false)?;

        Ok(())
    }

    pub fn open(
        &mut self,
        price: u64,
        amount: u64,
        long: bool,
        leverage: u32,
        mfr: &MarketFeeRates,
    ) -> DexResult<(u64, u64, u64, u64)> {
        if long {
            self.long.open(price, amount, leverage, mfr)
        } else {
            self.short.open(price, amount, leverage, mfr)
        }
    }

    pub fn close(
        &mut self,
        size: u64,
        price: u64,
        long: bool,
        mfr: &MarketFeeRates,
        liquidate: bool,
        limit_order: bool,
    ) -> DexResult<(u64, u64, i64, u64, u64, u64)> {
        if long {
            self.long.close(size, price, mfr, liquidate, limit_order)
        } else {
            self.short.close(size, price, mfr, liquidate, limit_order)
        }
    }

    pub fn sub_closing(&mut self, long: bool, closing_size: u64) -> DexResult {
        if long {
            self.long.sub_closing(closing_size)
        } else {
            self.short.sub_closing(closing_size)
        }
    }

    pub fn add_closing(&mut self, long: bool, size: u64) -> DexResult<u64> {
        if long {
            self.long.add_closing(size)
        } else {
            self.short.add_closing(size)
        }
    }

    pub fn unclosing_size(&mut self, long: bool) -> DexResult<u64> {
        if long {
            self.long.unclosing_size()
        } else {
            self.short.unclosing_size()
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserDIOption {
    pub id: u64,
    pub created: u64,
    pub expiry_date: i64,
    pub strike_price: u64,
    pub settle_price: u64,
    pub size: u64,
    pub borrowed_base_funds: u64,
    pub borrowed_quote_funds: u64,
    pub premium_rate: u16,
    pub base_asset_index: u8,
    pub quote_asset_index: u8,
    pub is_call: bool,
    pub exercised: bool,
    pub settled: bool,
    padding: [u8; 17],
}

impl UserDIOption {
    pub fn init(
        &mut self,
        option: &DIOption,
        created: u64,
        size: u64,
        borrow_base_funds: u64,
        borrow_quote_funds: u64,
    ) -> DexResult {
        self.id = option.id;
        self.created = created;
        self.expiry_date = option.expiry_date;
        self.strike_price = option.strike_price;
        self.size = size;
        self.borrowed_base_funds = borrow_base_funds;
        self.borrowed_quote_funds = borrow_quote_funds;
        self.premium_rate = option.premium_rate;
        self.base_asset_index = option.base_asset_index;
        self.quote_asset_index = option.quote_asset_index;
        self.is_call = option.is_call;
        self.size = size;

        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserAsset {
    pub amount: u64,
    pub asset: u8, // asset index
    padding: [u8; 23],
}

pub struct UserState<'a> {
    pub meta: &'a mut MetaInfo,
    pub order_pool: SmallList<'a, UserOrder>,
    pub position_pool: SmallList<'a, UserPosition>,
    pub di_option_pool: SmallList<'a, UserDIOption>,
    pub asset_pool: SmallList<'a, UserAsset>,
}

impl<'a> UserState<'a> {
    pub fn required_account_size(
        max_order_count: u8,
        max_position_count: u8,
        max_option_count: u8,
        max_asset_count: u8,
    ) -> usize {
        let mut size = 0;

        size += mem::size_of::<MetaInfo>();
        size += SmallList::<UserOrder>::required_data_len(max_order_count);
        size += SmallList::<UserPosition>::required_data_len(max_position_count);
        size += SmallList::<UserDIOption>::required_data_len(max_option_count);
        size += SmallList::<UserAsset>::required_data_len(max_asset_count);

        size
    }

    pub fn initialize(
        account: &'a AccountInfo,
        max_order_count: u8,
        max_position_count: u8,
        max_di_option_count: u8,
        max_asset_count: u8,
        owner: Pubkey,
    ) -> DexResult {
        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => return Err(error!(DexError::FailedMountAccount)),
        };

        let basic = unsafe { &mut *(data_ptr as *mut MetaInfo) };
        basic.magic = USER_STATE_MAGIC_NUMBER;
        basic.order_slot_count = max_order_count;
        basic.position_slot_count = max_position_count;
        basic.di_option_slot_count = max_di_option_count;
        basic.asset_slot_count = max_asset_count;

        basic.user_list_index = NIL32;
        basic.serial_number = 0;
        basic.owner = owner;

        let user_state = Self::mount(account, false)?;

        user_state.borrow().order_pool.initialize()?;
        user_state.borrow().position_pool.initialize()?;
        user_state.borrow().di_option_pool.initialize()?;
        user_state.borrow().asset_pool.initialize()?;

        Ok(())
    }

    fn mount_internal(
        data_ptr: *mut u8,
        data_size: usize,
        should_initialized: bool,
    ) -> DexResult<RefCell<Self>> {
        let mut offset = 0usize;

        let meta = unsafe { &mut *(data_ptr as *mut MetaInfo) };
        offset += mem::size_of::<MetaInfo>();

        let order_data_ptr = unsafe { data_ptr.add(offset) };
        let order_pool = SmallList::<UserOrder>::mount(
            order_data_ptr,
            meta.order_slot_count,
            should_initialized,
        )?;
        offset += order_pool.data_len();

        let position_data_ptr = unsafe { data_ptr.add(offset) };
        let position_pool = SmallList::<UserPosition>::mount(
            position_data_ptr,
            meta.position_slot_count,
            should_initialized,
        )?;
        offset += position_pool.data_len();

        let di_option_data_ptr = unsafe { data_ptr.add(offset) };
        let di_option_pool = SmallList::<UserDIOption>::mount(
            di_option_data_ptr,
            meta.di_option_slot_count,
            should_initialized,
        )?;
        offset += di_option_pool.data_len();

        let asset_data_ptr = unsafe { data_ptr.add(offset) };
        let asset_pool = SmallList::<UserAsset>::mount(
            asset_data_ptr,
            meta.asset_slot_count,
            should_initialized,
        )?;
        offset += asset_pool.data_len();

        require!(offset <= data_size, DexError::FailedMountUserState);

        Ok(RefCell::new(UserState {
            meta,
            order_pool,
            position_pool,
            di_option_pool,
            asset_pool,
        }))
    }

    pub fn mount(account: &'a AccountInfo, should_initialized: bool) -> DexResult<RefCell<Self>> {
        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => return Err(error!(DexError::FailedMountAccount)),
        };

        UserState::mount_internal(data_ptr, account.data_len(), should_initialized)
    }

    #[cfg(feature = "client-support")]
    pub fn mount_buf(buf: Vec<u8>) -> DexResult<RefCell<Self>> {
        let (data_ptr, data_size) = {
            let mut me = ManuallyDrop::new(buf);
            (me.as_mut_ptr(), me.len())
        };

        UserState::mount_internal(data_ptr, data_size, true)
    }

    #[cfg(feature = "client-support")]
    pub fn require_liquidate(
        &self,
        market: u8,
        long: bool,
        market_price: u64,
        mfr: &MarketFeeRates,
    ) -> DexResult {
        let position = self.find_or_new_position(market, false)?;
        let (_, collateral, pnl, _, close_fee, borrow_fee) =
            position
                .data
                .close(u64::MAX, market_price, long, mfr, true, false)?;

        if pnl < 0 {
            let loss = (pnl.abs() as u64) + close_fee + borrow_fee;
            if loss
                >= collateral
                    .safe_mul((100 - mfr.liquidate_threshold) as u64)?
                    .safe_div(100u128)? as u64
            {
                return Ok(());
            }
        }
        Err(error!(DexError::RequireNoLiquidation))
    }

    #[inline]
    pub fn serial_number(&self) -> u32 {
        self.meta.serial_number
    }

    #[inline]
    pub fn user_list_index(&self) -> u32 {
        self.meta.user_list_index
    }

    #[inline]
    pub fn set_user_list_index(&mut self, index: u32) {
        self.meta.user_list_index = index;
    }

    #[inline]
    pub fn inc_serial_number(&mut self) {
        self.meta.serial_number += 1;
    }

    pub fn get_position_size(&self, market: u8, long: bool) -> DexResult<u64> {
        let position = self.find_or_new_position(market, false)?;
        let size = if long {
            position.data.long.size
        } else {
            position.data.short.size
        };

        Ok(size)
    }

    pub fn open_position(
        &mut self,
        market: u8,
        price: u64,
        amount: u64,
        long: bool,
        leverage: u32,
        mfr: &MarketFeeRates,
    ) -> DexResult<(u64, u64, u64, u64)> {
        let position = self.find_or_new_position(market, true)?;
        position.data.open(price, amount, long, leverage, mfr)
    }

    pub fn close_position(
        &mut self,
        market: u8,
        size: u64,
        price: u64,
        long: bool,
        mfr: &MarketFeeRates,
        liquidate: bool,
        limit_order: bool,
    ) -> DexResult<(u64, u64, i64, u64, u64, u64)> {
        let position = self.find_or_new_position(market, false)?;
        position
            .data
            .close(size, price, long, mfr, liquidate, limit_order)
    }

    pub fn new_bid_order(
        &mut self,
        order_slot: u32,
        size: u64,
        price: u64,
        leverage: u32,
        long: bool,
        market: u8,
        asset: u8,
    ) -> DexResult<u8> {
        let order = self.order_pool.new_slot()?;
        order
            .data
            .init_as_bid(order_slot, size, price, leverage, long, market, asset)?;

        self.order_pool.add_to_tail(order)?;

        Ok(order.index)
    }

    pub fn new_ask_order(
        &mut self,
        size: u64,
        price: u64,
        long: bool,
        market: u8,
    ) -> DexResult<(u8, u64)> {
        let position = self.find_or_new_position(market, false)?;
        let added_closing_size = position.data.add_closing(long, size)?;
        require!(added_closing_size > 0, DexError::NoSizeForAskOrder);

        let order = self.order_pool.new_slot()?;
        order
            .data
            .init_as_ask(added_closing_size, price, long, market)?;

        self.order_pool.add_to_tail(order)?;

        Ok((order.index, added_closing_size))
    }

    pub fn set_ask_order_slot(&mut self, user_order_slot: u8, order_slot: u32) -> DexResult {
        let order = self.order_pool.from_index(user_order_slot)?;
        require!(order.in_use(), DexError::InvalidIndex);
        order.data.order_slot = order_slot;

        Ok(())
    }

    pub fn get_order(&self, user_order_slot: u8) -> DexResult<UserOrder> {
        let order = self.order_pool.from_index(user_order_slot)?;
        require!(order.in_use(), DexError::InvalidIndex);

        Ok(order.data)
    }

    pub fn get_order_info(&self, user_order_slot: u8) -> DexResult<u32> {
        let order = self.order_pool.from_index(user_order_slot)?;
        require!(order.in_use(), DexError::InvalidIndex);

        Ok(order.data.order_slot)
    }

    pub fn unlink_order(
        &mut self,
        user_order_slot: u8,
        cancel: bool,
    ) -> DexResult<(u8, bool, bool, u8, u64)> {
        let order = self.order_pool.from_index(user_order_slot)?;
        require!(order.in_use(), DexError::InvalidIndex);

        if !order.data.open && cancel {
            let position = self.find_or_new_position(order.data.market, false)?;
            position
                .data
                .sub_closing(order.data.long, order.data.size)?;
        }

        let UserOrder {
            market,
            open,
            long,
            asset,
            size,
            ..
        } = order.data;
        self.order_pool.remove(user_order_slot)?;

        Ok((market, open, long, asset, size))
    }

    pub fn collect_market_orders(&self, market: u8) -> Vec<u8> {
        let mut orders: Vec<u8> = vec![];

        for order in self.order_pool.into_iter() {
            if order.data.market == market {
                orders.push(order.index);
            }
        }

        orders
    }

    pub fn collect_orders(&self, market: usize, open: bool) -> Vec<u8> {
        let mut orders: Vec<u8> = vec![];

        for order in self.order_pool.into_iter() {
            if order.data.open == open && order.data.market == market as u8 {
                orders.push(order.index);
            }
        }

        orders
    }

    pub fn collect_ask_orders(&self, market: u8, long: bool) -> Vec<u8> {
        let mut orders: Vec<u8> = vec![];

        for order in self.order_pool.into_iter() {
            if !order.data.open && (order.data.market == market) && (order.data.long == long) {
                orders.push(order.index);
            }
        }

        orders
    }

    pub fn find_or_new_position(
        &self,
        market: u8,
        create: bool,
    ) -> DexResult<&mut SmallListSlot<UserPosition>> {
        let lookup = self
            .position_pool
            .into_iter()
            .find(|x| x.data.market == market);

        if let Some(p) = lookup {
            return Ok(p);
        }

        if !create {
            return Err(error!(DexError::PositionNotExist));
        }

        let position = self.position_pool.new_slot()?;
        self.position_pool.add_to_tail(position)?;
        position.data.init(market)?;

        Ok(position)
    }

    pub fn enter_staking_vlp(&mut self, pool: &mut StakingPool, amount: u64) -> DexResult {
        self.meta.vlp.enter_staking(pool, amount)
    }

    pub fn leave_staking_vlp(&mut self, pool: &mut StakingPool, amount: u64) -> DexResult {
        self.meta.vlp.leave_staking(pool, amount)
    }

    pub fn withdrawable_vlp_amount(&self, amount: u64) -> u64 {
        self.meta.vlp.staked.min(amount)
    }

    pub fn di_new_option(
        &mut self,
        raw: &DIOption,
        size: u64,
        borrow_base_funds: u64,
        borrow_quote_funds: u64,
    ) -> DexResult {
        let created = get_timestamp()? as u64;
        let dup = self
            .di_option_pool
            .into_iter()
            .find(|x| x.data.created == created);

        if dup.is_some() {
            return Err(error!(DexError::DIOptionDupID));
        }

        let option = self.di_option_pool.new_slot()?;
        option
            .data
            .init(raw, created, size, borrow_base_funds, borrow_quote_funds)?;

        self.di_option_pool.add_to_tail(option)?;

        Ok(())
    }

    pub fn di_get_option(&self, created: u64, settled: bool) -> DexResult<(u8, UserDIOption)> {
        let lookup = self
            .di_option_pool
            .into_iter()
            .find(|x| x.data.created == created && x.data.settled == settled);
        if let Some(p) = lookup {
            return Ok((p.index, p.data));
        }

        return Err(error!(DexError::DIOptionNotFound));
    }

    pub fn di_remove_option(&mut self, slot: u8) -> DexResult {
        self.di_option_pool.remove(slot)
    }

    pub fn di_settle_option(
        &mut self,
        slot: u8,
        settle_price: u64,
        exercised: bool,
        withdrawable: u64,
    ) -> DexResult {
        let option = self.di_option_pool.from_index(slot)?;
        if option.data.is_call {
            if exercised {
                option.data.borrowed_quote_funds = withdrawable;
                option.data.borrowed_base_funds = 0;
            } else {
                option.data.borrowed_base_funds = withdrawable;
                option.data.borrowed_quote_funds = 0;
            }
        } else {
            if exercised {
                option.data.borrowed_base_funds = withdrawable;
                option.data.borrowed_quote_funds = 0;
            } else {
                option.data.borrowed_quote_funds = withdrawable;
                option.data.borrowed_base_funds = 0;
            }
        }

        option.data.settle_price = settle_price;
        option.data.exercised = exercised;
        option.data.settled = true;

        Ok(())
    }

    pub fn di_withdraw_from_settled_option(&mut self, created: u64) -> DexResult<(u8, u64)> {
        let lookup = self
            .di_option_pool
            .into_iter()
            .find(|x| x.data.created == created);
        if let Some(p) = lookup {
            require!(p.data.settled, DexError::DIOptionNotSettled);

            let (asset_index, withdrawable) = if p.data.is_call {
                if p.data.exercised {
                    (p.data.quote_asset_index, p.data.borrowed_quote_funds)
                } else {
                    (p.data.base_asset_index, p.data.borrowed_base_funds)
                }
            } else {
                if p.data.exercised {
                    (p.data.base_asset_index, p.data.borrowed_base_funds)
                } else {
                    (p.data.quote_asset_index, p.data.borrowed_quote_funds)
                }
            };

            self.di_option_pool.remove(p.index)?;

            return Ok((asset_index, withdrawable));
        }

        return Err(error!(DexError::DIOptionNotFound));
    }

    #[cfg(feature = "client-support")]
    pub fn collect_di_option(&self, id: u64) -> Vec<UserDIOption> {
        let mut options: Vec<UserDIOption> = vec![];

        for o in self.di_option_pool.into_iter() {
            if o.data.id == id {
                options.push(o.data);
            }
        }

        options
    }

    #[cfg(feature = "client-support")]
    pub fn collect_unsettled_di_options(&self) -> Vec<UserDIOption> {
        let mut options: Vec<UserDIOption> = vec![];

        for o in self.di_option_pool.into_iter() {
            if !o.data.settled {
                options.push(o.data);
            }
        }

        options
    }

    #[cfg(feature = "client-support")]
    pub fn collect_all_di_options(&self) -> Vec<UserDIOption> {
        let mut options: Vec<UserDIOption> = vec![];

        for o in self.di_option_pool.into_iter() {
            options.push(o.data);
        }

        options
    }

    #[cfg(feature = "client-support")]
    pub fn di_read_created_option(&self, created: u64) -> DexResult<(u8, u64)> {
        let lookup = self
            .di_option_pool
            .into_iter()
            .find(|x| x.data.created == created);
        if let Some(p) = lookup {
            require!(p.data.settled, DexError::DIOptionNotSettled);

            let (asset_index, withdrawable) = if p.data.is_call {
                if p.data.exercised {
                    (p.data.quote_asset_index, p.data.borrowed_quote_funds)
                } else {
                    (p.data.base_asset_index, p.data.borrowed_base_funds)
                }
            } else {
                if p.data.exercised {
                    (p.data.base_asset_index, p.data.borrowed_base_funds)
                } else {
                    (p.data.quote_asset_index, p.data.borrowed_quote_funds)
                }
            };

            return Ok((asset_index, withdrawable));
        }

        return Err(error!(DexError::DIOptionNotFound));
    }

    pub fn find_or_new_asset(
        &self,
        asset: u8,
        create: bool,
    ) -> DexResult<&mut SmallListSlot<UserAsset>> {
        let lookup = self.asset_pool.into_iter().find(|x| x.data.asset == asset);

        if let Some(p) = lookup {
            return Ok(p);
        }

        if !create {
            return Err(error!(DexError::AssetNotExist));
        }

        let asset_slot = self.asset_pool.new_slot()?;
        self.asset_pool.add_to_tail(asset_slot)?;
        asset_slot.data.asset = asset;
        asset_slot.data.amount = 0;

        Ok(asset_slot)
    }

    pub fn deposit_asset(&mut self, asset: u8, amount: u64) -> DexResult {
        let asset_slot = self.find_or_new_asset(asset, true)?;
        asset_slot.data.amount = asset_slot.data.amount.safe_add(amount)?;

        Ok(())
    }

    pub fn withdraw_asset(&mut self, asset: u8) -> DexResult<u64> {
        let asset_slot = self.find_or_new_asset(asset, false)?;
        let amount = asset_slot.data.amount;

        asset_slot.data.amount = 0;

        Ok(amount)
    }

    #[cfg(feature = "client-support")]
    pub fn find_asset(&self, asset: u8) -> DexResult<&SmallListSlot<UserAsset>> {
        let lookup = self.asset_pool.into_iter().find(|x| x.data.asset == asset);

        if let Some(p) = lookup {
            return Ok(p);
        }

        return Err(error!(DexError::AssetNotExist));
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::utils::{test::*, BORROW_FEE_RATE_BASE, FEE_RATE_BASE};
    use bumpalo::Bump;

    impl<'a> UserState<'a> {
        fn get_position(&self, market: u8, long: bool) -> DexResult<Position> {
            let position = self.find_or_new_position(market, false)?;
            let p = if long {
                position.data.long
            } else {
                position.data.short
            };

            Ok(p)
        }
    }

    #[test]
    fn test_user_state_init() {
        let bump = Bump::new();
        let order_slot_count = 8u8;
        let position_slot_count = 4u8;
        let di_option_slot_count = 8u8;
        let asset_slot_count = 8u8;

        let required_size = UserState::required_account_size(
            order_slot_count,
            position_slot_count,
            di_option_slot_count,
            asset_slot_count,
        );

        println!("required account size {}", required_size);

        let account = gen_account(required_size, &bump);
        UserState::initialize(
            &account,
            order_slot_count,
            position_slot_count,
            di_option_slot_count,
            asset_slot_count,
            Pubkey::default(),
        )
        .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();

        assert_eq!(us.borrow().meta.order_slot_count, order_slot_count);
        assert_eq!(us.borrow().meta.position_slot_count, position_slot_count);
        assert_eq!(us.borrow().meta.di_option_slot_count, di_option_slot_count);
        assert_eq!(us.borrow().meta.asset_slot_count, asset_slot_count);

        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => panic!("Failed to get data ptr"),
        };

        let buf = unsafe { std::slice::from_raw_parts(data_ptr, account.data_len()) };
        assert_eq!(buf.len(), account.data_len());

        let us_on_buf = UserState::mount_buf(buf.to_vec()).assert_unwrap();
        assert_eq!(us_on_buf.borrow().meta.order_slot_count, order_slot_count);
        assert_eq!(
            us_on_buf.borrow().meta.position_slot_count,
            position_slot_count
        );
        assert_eq!(
            us_on_buf.borrow().meta.di_option_slot_count,
            di_option_slot_count
        );
        assert_eq!(us_on_buf.borrow().meta.asset_slot_count, asset_slot_count);
    }

    fn mock_mfr() -> MarketFeeRates {
        MarketFeeRates {
            charge_borrow_fee_interval: 3600,
            minimum_collateral: 200_000_000u64,
            borrow_fee_rate: 10,
            open_fee_rate: 20,
            close_fee_rate: 20,
            liquidate_fee_rate: 50,
            liquidate_threshold: 10,
            base_decimals: 9,
        }
    }

    #[test]
    fn test_open_long() {
        let bump = Bump::new();
        let required_size = UserState::required_account_size(8u8, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, 8u8, 8u8, 8u8, 8u8, Pubkey::default()).assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();

        let mfr = mock_mfr();
        let (size, collateral, borrow, open_fee) = us
            .borrow_mut()
            .open_position(0, usdc(20000.), btc(1.0), true, 20 * 1000, &mfr)
            .assert_unwrap();

        let expected_open_fee = btc(0.038461538);
        let expected_collateral = btc(1.0) - expected_open_fee;

        assert_eq!(open_fee, expected_open_fee);
        assert_eq!(collateral, expected_collateral);
        assert_eq!(size, expected_collateral * 20);
        assert_eq!(borrow, expected_collateral * 20);

        let long = us
            .borrow()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .long;

        assert_eq!(long.size, expected_collateral * 20);
        assert_eq!(long.average_price, usdc(20000.));
        assert_eq!(long.collateral, expected_collateral);
        assert_eq!(long.borrowed_amount, expected_collateral * 20);
        assert_eq!(long.closing_size, 0);
        assert_eq!(long.cumulative_fund_fee, 0);

        const HOURS_2: u64 = 2;
        us.borrow_mut()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .long
            .mock_after_hours(HOURS_2);

        let (size, collateral, borrow, open_fee) = us
            .borrow_mut()
            .open_position(0, usdc(26000.), btc(1.0), true, 20 * 1000, &mfr)
            .assert_unwrap();

        assert_eq!(open_fee, expected_open_fee);
        assert_eq!(collateral, expected_collateral);
        assert_eq!(size, expected_collateral * 20);
        assert_eq!(borrow, expected_collateral * 20);

        let long = us
            .borrow()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .long;

        assert_eq!(long.size, expected_collateral * 20 * 2);
        assert_eq!(long.average_price, usdc(23000.));
        assert_eq!(long.collateral, expected_collateral * 2);
        assert_eq!(long.borrowed_amount, expected_collateral * 20 * 2);
        assert_eq!(long.closing_size, 0);

        let expected_fund_fee = expected_collateral * 20 * (mfr.borrow_fee_rate as u64) * HOURS_2
            / BORROW_FEE_RATE_BASE as u64;
        assert_eq!(long.cumulative_fund_fee, expected_fund_fee);
    }

    #[test]
    fn test_open_short() {
        let bump = Bump::new();
        let required_size = UserState::required_account_size(8u8, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, 8u8, 8u8, 8u8, 8u8, Pubkey::default()).assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        let mfr = mock_mfr();
        let leverage = 10u64;
        let (size, collateral, borrow, open_fee) = us
            .borrow_mut()
            .open_position(
                0,
                usdc(20000.),
                usdc(2000.0),
                false,
                leverage as u32 * 1000,
                &mfr,
            )
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

        let short = us
            .borrow()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .short;

        assert_eq!(short.size, expected_size);
        assert_eq!(short.average_price, usdc(20000.));
        assert_eq!(short.collateral, expected_collateral);
        assert_eq!(short.borrowed_amount, expected_collateral * leverage);
        assert_eq!(short.closing_size, 0);
        assert_eq!(short.cumulative_fund_fee, 0);

        const HOURS_2: u64 = 2;
        us.borrow_mut()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .short
            .mock_after_hours(HOURS_2);

        let (size, collateral, borrow, open_fee) = us
            .borrow_mut()
            .open_position(
                0,
                usdc(20000.),
                usdc(2000.0),
                false,
                leverage as u32 * 1000,
                &mfr,
            )
            .assert_unwrap();

        assert_eq!(open_fee, expected_open_fee);
        assert_eq!(collateral, expected_collateral);
        assert_eq!(size, expected_size);
        assert_eq!(borrow, expected_collateral * leverage);

        let short = us
            .borrow()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .short;
        assert_eq!(short.size, expected_size * 2);
        assert_eq!(short.average_price, usdc(20000.));
        assert_eq!(short.collateral, expected_collateral * 2);
        assert_eq!(short.borrowed_amount, expected_collateral * leverage * 2);
        assert_eq!(short.closing_size, 0);

        let expected_fund_fee =
            expected_collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2
                / BORROW_FEE_RATE_BASE as u64;
        assert_eq!(short.cumulative_fund_fee, expected_fund_fee);
    }

    #[test]
    fn test_open_two_positions() {
        let bump = Bump::new();
        let required_size = UserState::required_account_size(8u8, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, 8u8, 8u8, 8u8, 8u8, Pubkey::default()).assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        let mfr = mock_mfr();
        let leverage = 10u64;

        us.borrow_mut()
            .open_position(
                0,
                usdc(20000.),
                usdc(2000.0),
                false,
                leverage as u32 * 1000,
                &mfr,
            )
            .assert_ok();

        us.borrow_mut()
            .open_position(
                1,
                usdc(2000.),
                usdc(2000.0),
                false,
                leverage as u32 * 1000,
                &mfr,
            )
            .assert_ok();

        let binding = us.borrow();
        let btc = binding.position_pool.from_index(0).assert_unwrap();
        assert_eq!(btc.index, 0);
        assert_eq!(btc.next, 1);
        assert_eq!(btc.prev, 255);

        let eth = binding.position_pool.from_index(1).assert_unwrap();
        assert_eq!(eth.index, 1);
        assert_eq!(eth.next, 255);
        assert_eq!(eth.prev, 0);
    }

    #[test]
    fn test_close_long_with_profit() {
        let bump = Bump::new();
        let required_size = UserState::required_account_size(8u8, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, 8u8, 8u8, 8u8, 8u8, Pubkey::default()).assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        let mfr = mock_mfr();
        let leverage = 20u64;
        let (size, collateral, borrow, _) = us
            .borrow_mut()
            .open_position(
                0,
                usdc(20000.),
                btc(1.0),
                true,
                leverage as u32 * 1000,
                &mfr,
            )
            .assert_unwrap();

        const HOURS_2: u64 = 2;
        us.borrow_mut()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .long
            .mock_after_hours(HOURS_2);

        let (returned, collateral_unlocked, pnl, _closed_size, close_fee, borrow_fee) = us
            .borrow_mut()
            .close_position(0, size, usdc(25000.), true, &mfr, false, false)
            .assert_unwrap();

        let expected_borrow_fee = collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2
            / BORROW_FEE_RATE_BASE as u64;

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
    fn test_close_long_with_loss() {
        let bump = Bump::new();
        let required_size = UserState::required_account_size(8u8, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, 8u8, 8u8, 8u8, 8u8, Pubkey::default()).assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        let mfr = mock_mfr();
        let leverage = 5u64;
        let (size, collateral, borrow, _) = us
            .borrow_mut()
            .open_position(
                0,
                usdc(20000.),
                btc(1.0),
                true,
                leverage as u32 * 1000,
                &mfr,
            )
            .assert_unwrap();

        const HOURS_2: u64 = 2;
        us.borrow_mut()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .long
            .mock_after_hours(HOURS_2);

        let (returned, collateral_unlocked, pnl, _closed_size, close_fee, borrow_fee) = us
            .borrow_mut()
            .close_position(0, size, usdc(18000.), true, &mfr, false, false)
            .assert_unwrap();

        let expected_borrow_fee = collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2
            / BORROW_FEE_RATE_BASE as u64;

        let expected_close_fee = size * (mfr.close_fee_rate as u64) / FEE_RATE_BASE as u64;

        let expected_pnl =
            (size as u128) * (usdc(20000.) - usdc(18000.)) as u128 / usdc(20000.) as u128;

        assert_eq!(returned, borrow);
        assert_eq!(collateral_unlocked, collateral);
        assert_eq!(borrow_fee, expected_borrow_fee);
        assert_eq!(close_fee, expected_close_fee);
        assert_eq!(pnl, -(expected_pnl as i64));
    }

    #[test]
    fn test_close_short_with_profit() {
        let bump = Bump::new();
        let required_size = UserState::required_account_size(8u8, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, 8u8, 8u8, 8u8, 8u8, Pubkey::default()).assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        let mfr = mock_mfr();
        let leverage = 10u64;
        let (size, collateral, borrow, _) = us
            .borrow_mut()
            .open_position(
                0,
                usdc(20000.),
                usdc(2000.),
                false,
                leverage as u32 * 1000,
                &mfr,
            )
            .assert_unwrap();

        const HOURS_2: u64 = 2;
        us.borrow_mut()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .short
            .mock_after_hours(HOURS_2);

        let (returned, collateral_unlocked, pnl, _closed_size, close_fee, borrow_fee) = us
            .borrow_mut()
            .close_position(0, size, usdc(18000.), false, &mfr, false, false)
            .assert_unwrap();

        let expected_borrow_fee = collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2
            / BORROW_FEE_RATE_BASE as u64;

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
    fn test_close_short_with_loss() {
        let bump = Bump::new();
        let required_size = UserState::required_account_size(8u8, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, 8u8, 8u8, 8u8, 8u8, Pubkey::default()).assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        let mfr = mock_mfr();
        let leverage = 10u64;
        let (size, collateral, borrow, _) = us
            .borrow_mut()
            .open_position(
                0,
                usdc(20000.),
                usdc(2000.),
                false,
                leverage as u32 * 1000,
                &mfr,
            )
            .assert_unwrap();

        const HOURS_2: u64 = 2;
        us.borrow_mut()
            .find_or_new_position(0, false)
            .assert_unwrap()
            .data
            .short
            .mock_after_hours(HOURS_2);

        let (returned, collateral_unlocked, pnl, _closed_size, close_fee, borrow_fee) = us
            .borrow_mut()
            .close_position(0, size, usdc(22000.), false, &mfr, false, false)
            .assert_unwrap();

        let expected_borrow_fee = collateral * leverage * (mfr.borrow_fee_rate as u64) * HOURS_2
            / BORROW_FEE_RATE_BASE as u64;

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
    fn test_new_bid_order() {
        let bump = Bump::new();
        let max_order_count = 8u8;
        let required_size = UserState::required_account_size(max_order_count, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, max_order_count, 8u8, 8u8, 8u8, Pubkey::default())
            .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        for i in 0..max_order_count {
            let user_order_slot = us
                .borrow_mut()
                .new_bid_order(
                    0xff + i as u32,
                    btc(0.1 + i as f64),
                    usdc(20000. + i as f64),
                    20 + i as u32,
                    true,
                    i,
                    9,
                )
                .assert_unwrap();
            assert_eq!(user_order_slot, i);
        }

        for i in 0..max_order_count {
            let order = us.borrow().get_order(i).assert_unwrap();
            assert_eq!(order.order_slot, 0xff + i as u32);
            assert_eq!(order.size, btc(0.1 + i as f64));
            assert_eq!(order.price, usdc(20000. + i as f64));
            assert_eq!(order.leverage, 20 + i as u32);
            assert_eq!(order.long, true);
            assert_eq!(order.market, i);
        }
    }

    #[test]
    fn test_max_order_count() {
        let bump = Bump::new();
        let max_order_count = 8u8;
        let required_size = UserState::required_account_size(max_order_count, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, max_order_count, 8u8, 8u8, 8u8, Pubkey::default())
            .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();

        // Create bid orders
        for _ in 0..max_order_count {
            us.borrow_mut()
                .new_bid_order(0xff, btc(0.1), usdc(20000.), 20, true, 0x0, 9)
                .assert_unwrap();
        }

        us.borrow_mut()
            .new_bid_order(0xff, btc(0.1), usdc(20000.), 20, true, 0x0, 9)
            .assert_err();

        // Release all bid orders
        for i in 0..max_order_count {
            us.borrow_mut().unlink_order(i, true).assert_unwrap();
        }

        // Mock position
        let mfr = mock_mfr();
        us.borrow_mut()
            .open_position(0, usdc(20000.), usdc(2000.), false, 10 * 1000, &mfr)
            .assert_unwrap();

        // Create ask orders
        for _ in 0..max_order_count {
            us.borrow_mut()
                .new_ask_order(btc(0.1), usdc(19000.), false, 0)
                .assert_unwrap();
        }
        us.borrow_mut()
            .new_ask_order(btc(0.1), usdc(19000.), false, 0)
            .assert_err();
    }

    #[test]
    fn test_new_ask_order() {
        let bump = Bump::new();
        let max_order_count = 8u8;
        let required_size = UserState::required_account_size(max_order_count, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, max_order_count, 8u8, 8u8, 8u8, Pubkey::default())
            .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();

        // Mock position
        let mfr = mock_mfr();
        let (size, _, _, _) = us
            .borrow_mut()
            .open_position(0, usdc(20000.), usdc(2000.), false, 10 * 1000, &mfr)
            .assert_unwrap();

        let (user_order_slot, _) = us
            .borrow_mut()
            .new_ask_order(size / 2, usdc(19000.), false, 0)
            .assert_unwrap();

        let order = us.borrow().get_order(user_order_slot).assert_unwrap();
        assert_eq!(order.size, size / 2);
        assert_eq!(order.price, usdc(19000.));
        assert_eq!(order.leverage, 0);
        assert_eq!(order.long, false);
        assert_eq!(order.market, 0);

        let position = us.borrow().get_position(0, false).assert_unwrap();
        assert_eq!(position.size, size);
        assert_eq!(position.closing_size, size / 2);
    }

    #[test]
    fn test_new_ask_order_size_error() {
        let bump = Bump::new();
        let max_order_count = 8u8;
        let required_size = UserState::required_account_size(max_order_count, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, max_order_count, 8u8, 8u8, 8u8, Pubkey::default())
            .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();

        // Mock position
        let mfr = mock_mfr();
        let (size, _, _, _) = us
            .borrow_mut()
            .open_position(0, usdc(20000.), usdc(2000.), false, 10 * 1000, &mfr)
            .assert_unwrap();

        us.borrow_mut()
            .new_ask_order(size / 2, usdc(19000.), false, 0)
            .assert_ok();

        us.borrow_mut()
            .new_ask_order(size, usdc(19000.), false, 0)
            .assert_ok();

        // Can not place ask order with larger size
        us.borrow_mut()
            .new_ask_order(size / 2, usdc(19000.), false, 0)
            .assert_err();
    }

    #[test]
    fn test_collect_orders() {
        let bump = Bump::new();
        let max_order_count = 8u8;
        let required_size = UserState::required_account_size(max_order_count, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, max_order_count, 8u8, 8u8, 8u8, Pubkey::default())
            .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        us.borrow_mut()
            .new_bid_order(0xff, btc(0.1), usdc(20000.), 20, true, 0x0, 9)
            .assert_ok();

        us.borrow_mut()
            .new_bid_order(0xff, btc(0.01), usdc(22000.), 20, true, 0x0, 9)
            .assert_unwrap();

        let mfr = mock_mfr();
        let (size, _, _, _) = us
            .borrow_mut()
            .open_position(0, usdc(20000.), usdc(2000.), false, 10 * 1000, &mfr)
            .assert_unwrap();

        us.borrow_mut()
            .new_ask_order(size / 2, usdc(19000.), false, 0)
            .assert_ok();

        us.borrow_mut()
            .new_ask_order(size / 2, usdc(18000.), false, 0)
            .assert_ok();

        let orders = us.borrow().collect_market_orders(0);
        assert_eq!(orders.len(), 4);
        assert_eq!(orders[0], 0);
        assert_eq!(orders[1], 1);
        assert_eq!(orders[2], 2);
        assert_eq!(orders[3], 3);
    }

    #[test]
    fn test_unlink_bid_order() {
        let bump = Bump::new();
        let max_order_count = 8u8;
        let required_size = UserState::required_account_size(max_order_count, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, max_order_count, 8u8, 8u8, 8u8, Pubkey::default())
            .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();
        us.borrow_mut()
            .new_bid_order(0xff, btc(0.1), usdc(20000.), 20, true, 0x0, 9)
            .assert_ok();

        us.borrow_mut()
            .new_bid_order(0xff, btc(0.01), usdc(22000.), 20, true, 0x0, 9)
            .assert_unwrap();

        let mfr = mock_mfr();
        let (size, _, _, _) = us
            .borrow_mut()
            .open_position(0, usdc(20000.), usdc(2000.), false, 10 * 1000, &mfr)
            .assert_unwrap();

        us.borrow_mut()
            .new_ask_order(size / 2, usdc(19000.), false, 0)
            .assert_ok();

        us.borrow_mut()
            .new_ask_order(size / 2, usdc(18000.), false, 0)
            .assert_ok();

        let orders = us.borrow().collect_market_orders(0);
        assert_eq!(orders.len(), 4);

        us.borrow_mut().unlink_order(0, true).assert_ok();
        us.borrow_mut().unlink_order(1, true).assert_ok();
        us.borrow_mut().unlink_order(2, true).assert_ok();
        us.borrow_mut().unlink_order(3, true).assert_ok();
        let orders = us.borrow().collect_market_orders(0);
        assert_eq!(orders.len(), 0);

        us.borrow_mut().unlink_order(0, true).assert_err();
        us.borrow_mut().unlink_order(1, true).assert_err();
        us.borrow_mut().unlink_order(2, true).assert_err();
        us.borrow_mut().unlink_order(3, true).assert_err();
    }

    #[test]
    fn test_close_position_with_ask_order() {
        let bump = Bump::new();
        let max_order_count = 8u8;
        let required_size = UserState::required_account_size(max_order_count, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, max_order_count, 8u8, 8u8, 8u8, Pubkey::default())
            .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();

        // Mock position
        let mfr = mock_mfr();
        let (size, _, _, _) = us
            .borrow_mut()
            .open_position(0, usdc(20000.), usdc(2010.), false, 10 * 1000, &mfr)
            .assert_unwrap();

        us.borrow_mut()
            .new_ask_order(size / 2, usdc(19000.), false, 0)
            .assert_ok();

        // It should be ok to close the other half size.
        us.borrow_mut()
            .close_position(0, size / 2, usdc(19500.), false, &mfr, false, false)
            .assert_ok();

        let position = us.borrow().get_position(0, false).assert_unwrap();
        assert_eq!(position.size, size / 2);
        assert_eq!(position.closing_size, size / 2);

        // Fail to close the remained size.
        us.borrow_mut()
            .close_position(0, size / 2, usdc(19500.), false, &mfr, false, false)
            .assert_err();

        // Unlink the ask order
        us.borrow_mut().unlink_order(0, true).assert_ok();

        // Success to close
        us.borrow_mut()
            .close_position(0, size / 2, usdc(19500.), false, &mfr, false, false)
            .assert_ok();
    }

    #[test]
    fn test_require_liquidate() {
        let bump = Bump::new();
        let max_order_count = 8u8;
        let required_size = UserState::required_account_size(max_order_count, 8u8, 8u8, 8u8);
        let account = gen_account(required_size, &bump);
        UserState::initialize(&account, max_order_count, 8u8, 8u8, 8u8, Pubkey::default())
            .assert_ok();

        let us = UserState::mount(&account, true).assert_unwrap();

        // Mock position
        let mfr = mock_mfr();
        us.borrow_mut()
            .open_position(0, usdc(20000.), usdc(2000.), false, 10 * 1000, &mfr)
            .assert_ok();

        us.borrow()
            .require_liquidate(0, false, usdc(22000.), &mfr)
            .assert_ok();
        us.borrow()
            .require_liquidate(0, false, usdc(15000.), &mfr)
            .assert_err();
    }
}
