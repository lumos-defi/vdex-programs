use std::cell::{RefCell, RefMut};
use std::mem::{self, ManuallyDrop};

use crate::collections::small_list::*;
use crate::dex::state::*;
use crate::utils::{NIL32, USER_STATE_MAGIC_NUMBER};
use anchor_lang::prelude::*;

use crate::errors::{DexError, DexResult};

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum FeeType {
    MakerTaker = 0,
    Funding = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MetaInfo {
    pub magic: u32,
    pub user_list_index: u32,
    pub serial_number: u32,
    pub order_slot_count: u8,
    pub position_slot_count: u8,
    reserved: [u8; 130],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserOrder {
    pub common: Order,
    pub order_slot: u32,
    pub position_slot: u8,
    padding: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserPosition {
    pub long: Position,
    pub short: Position,
    pub market: u8,
    padding: [u8; 3],
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
    ) -> DexResult<(u64, u64, i64, u64)> {
        if long {
            self.long.close(size, price, mfr)
        } else {
            self.short.close(size, price, mfr)
        }
    }
}

pub struct UserState<'a> {
    pub meta: &'a mut MetaInfo,
    pub order_pool: SmallList<'a, UserOrder>,
    pub position_pool: SmallList<'a, UserPosition>,
}

impl<'a> UserState<'a> {
    pub fn required_account_size(max_order_count: u8, max_position_count: u8) -> usize {
        let mut size = 0;

        size += mem::size_of::<MetaInfo>();
        size += SmallList::<UserOrder>::required_data_len(max_order_count);
        size += SmallList::<UserPosition>::required_data_len(max_position_count);

        size
    }

    pub fn initialize(
        account: &'a AccountInfo,
        max_order_count: u8,
        max_position_count: u8,
    ) -> DexResult {
        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => return Err(error!(DexError::FailedMountAccount)),
        };

        let basic = unsafe { &mut *(data_ptr as *mut MetaInfo) };
        basic.magic = USER_STATE_MAGIC_NUMBER;
        basic.order_slot_count = max_order_count;
        basic.position_slot_count = max_position_count;

        basic.user_list_index = NIL32;
        basic.serial_number = 0;

        let user_state = Self::mount(account, false)?;

        user_state.borrow().order_pool.initialize()?;
        user_state.borrow().position_pool.initialize()?;

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

        require!(offset <= data_size, DexError::FailedMountUserState);

        Ok(RefCell::new(UserState {
            meta,
            order_pool,
            position_pool,
        }))
    }

    pub fn mount(account: &'a AccountInfo, should_initialized: bool) -> DexResult<RefCell<Self>> {
        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => return Err(error!(DexError::FailedMountAccount)),
        };

        UserState::mount_internal(data_ptr, account.data_len(), should_initialized)
    }

    pub fn mount_buf(buf: Vec<u8>) -> DexResult<RefCell<Self>> {
        let (data_ptr, data_size) = {
            let mut me = ManuallyDrop::new(buf);
            (me.as_mut_ptr(), me.len())
        };

        UserState::mount_internal(data_ptr, data_size, true)
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
        let position = self.find_position(market)?;
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
        let position = self.find_or_new_position(market)?;
        position.data.open(price, amount, long, leverage, mfr)
    }

    pub fn close_position(
        &mut self,
        market: u8,
        size: u64,
        price: u64,
        long: bool,
        mfr: &MarketFeeRates,
    ) -> DexResult<(u64, u64, i64, u64)> {
        let position = self.find_or_new_position(market)?;
        position.data.close(size, price, long, mfr)
    }

    pub fn new_order(
        &mut self,
        market: u8,
        size: u64,
        price: u64,
        long: bool,
        close: bool,
    ) -> DexResult {
        Ok(())
    }

    pub fn cancel_order(&mut self, order_slot: u8) -> DexResult {
        Ok(())
    }

    pub fn fill_order(&mut self, order_slot: u8, price: u64) -> DexResult {
        Ok(())
    }

    fn find_position(&self, market: u8) -> DexResult<&mut SmallListSlot<UserPosition>> {
        let lookup = self
            .position_pool
            .into_iter()
            .find(|x| x.data.market == market);

        if let Some(p) = lookup {
            return Ok(p);
        }

        return Err(error!(DexError::FoundNoPosition));
    }

    fn find_or_new_position(&self, market: u8) -> DexResult<&mut SmallListSlot<UserPosition>> {
        let lookup = self
            .position_pool
            .into_iter()
            .find(|x| x.data.market == market);

        if let Some(p) = lookup {
            return Ok(p);
        }

        let position = self.position_pool.new_slot()?;
        self.position_pool.add_to_tail(position)?;
        position.data.init(market)?;

        Ok(position)
    }
}
