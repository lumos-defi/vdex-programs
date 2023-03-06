use std::{
    cell::{RefCell, RefMut},
    mem,
};

use anchor_lang::prelude::*;

use crate::{
    collections::{SmallList, SmallListSlot},
    errors::{DexError, DexResult},
    utils::{get_timestamp, DI_ACCOUNT_MAGIC_NUMBER},
};

#[repr(C)]
pub struct DIMeta {
    pub magic: u32,
    pub admin: Pubkey,
    pub fee_rate: u16,
    pub stopped: bool,
    pub option_slot_count: u8,
    reserved: [u8; 60],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DIOption {
    pub id: u64,
    pub expiry_date: i64,
    pub strike_price: u64,
    pub settle_price: u64,
    pub minimum_open_size: u64,
    pub maximum_open_size: u64,
    pub stop_before_expiry: u64,
    pub volume: u64,
    pub settle_size: u64,
    pub premium_rate: u16,
    pub is_call: bool,
    pub stopped: bool,
    pub settled: bool,
    pub base_asset_index: u8,
    pub quote_asset_index: u8,
    reserved: [u8; 2],
}

impl DIOption {
    pub fn init(
        &mut self,
        id: u64,
        is_call: bool,
        base_asset_index: u8,
        quote_asset_index: u8,
        premium_rate: u16,
        expiry_date: i64,
        strike_price: u64,
        minimum_open_size: u64,
        maximum_open_size: u64,
        stop_before_expiry: u64,
    ) {
        self.id = id;
        self.is_call = is_call;
        self.base_asset_index = base_asset_index;
        self.quote_asset_index = quote_asset_index;
        self.premium_rate = premium_rate;
        self.expiry_date = expiry_date;
        self.strike_price = strike_price;
        self.minimum_open_size = minimum_open_size;
        self.maximum_open_size = maximum_open_size;
        self.stop_before_expiry = stop_before_expiry;
        self.stopped = false;
        self.settled = false;
        self.settle_price = 0;
        self.settle_size = 0;
        self.volume = 0;
    }
}

pub struct DI<'a> {
    pub meta: &'a mut DIMeta,
    pub options: SmallList<'a, DIOption>,
}

impl<'a> DI<'a> {
    pub fn required_account_size(max_options: u8) -> usize {
        let mut size = 0;

        size += mem::size_of::<DIMeta>();
        size += SmallList::<DIOption>::required_data_len(max_options);

        size
    }

    pub fn initialize(
        account: &'a AccountInfo,
        max_options: u8,
        admin: Pubkey,
        fee_rate: u16,
    ) -> DexResult {
        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => return Err(error!(DexError::FailedMountAccount)),
        };

        let meta = unsafe { &mut *(data_ptr as *mut DIMeta) };
        meta.magic = DI_ACCOUNT_MAGIC_NUMBER;
        meta.admin = admin;
        meta.fee_rate = fee_rate;
        meta.stopped = false;
        meta.option_slot_count = max_options;

        let di = Self::mount(account, false)?;
        di.borrow_mut().options.initialize()?;

        Ok(())
    }

    fn mount_internal(
        data_ptr: *mut u8,
        data_size: usize,
        should_initialized: bool,
    ) -> DexResult<RefCell<Self>> {
        let mut offset = 0usize;

        let meta = unsafe { &mut *(data_ptr as *mut DIMeta) };
        offset += mem::size_of::<DIMeta>();

        let options_data_ptr = unsafe { data_ptr.add(offset) };
        let options = SmallList::<DIOption>::mount(
            options_data_ptr,
            meta.option_slot_count,
            should_initialized,
        )?;
        offset += options.data_len();

        require!(offset <= data_size, DexError::FailedMountUserState);

        Ok(RefCell::new(DI { meta, options }))
    }

    pub fn mount(account: &'a AccountInfo, should_initialized: bool) -> DexResult<RefCell<Self>> {
        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => return Err(error!(DexError::FailedMountAccount)),
        };

        DI::mount_internal(data_ptr, account.data_len(), should_initialized)
    }

    #[cfg(feature = "client-support")]
    pub fn mount_buf(buf: Vec<u8>) -> DexResult<RefCell<Self>> {
        use std::mem::ManuallyDrop;

        let (data_ptr, data_size) = {
            let mut me = ManuallyDrop::new(buf);
            (me.as_mut_ptr(), me.len())
        };

        DI::mount_internal(data_ptr, data_size, true)
    }

    pub fn set_admin(&mut self, admin: Pubkey) {
        self.meta.admin = admin;
    }

    pub fn set_fee_rate(&mut self, fee_rate: u16) {
        self.meta.fee_rate = fee_rate;
    }

    pub fn create(
        &mut self,
        id: u64,
        is_call: bool,
        base_asset_index: u8,
        quote_asset_index: u8,
        premium_rate: u16,
        expiry_date: i64,
        strike_price: u64,
        minimum_open_size: u64,
        maximum_open_size: u64,
        stop_before_expiry: u64,
    ) -> DexResult {
        // Check no dup id
        if let Ok(_) = self.find_option(id) {
            return Err(error!(DexError::DIOptionDupID));
        }

        // Check if any option has same attributes
        let lookup = self.options.into_iter().find(|x| {
            x.data.is_call == is_call
                && x.data.base_asset_index == base_asset_index
                && x.data.quote_asset_index == quote_asset_index
                && x.data.expiry_date == expiry_date
                && x.data.strike_price == strike_price
        });
        if let Some(_) = lookup {
            return Err(error!(DexError::DIOptionDup));
        }

        let option = self.options.new_slot()?;
        option.data.init(
            id,
            is_call,
            base_asset_index,
            quote_asset_index,
            premium_rate,
            expiry_date,
            strike_price,
            minimum_open_size,
            maximum_open_size,
            stop_before_expiry,
        );
        self.options.add_to_tail(option)
    }

    pub fn update(&mut self, id: u64, premium_rate: u16, stop: bool) -> DexResult {
        let option = self.find_option(id)?;

        let date = get_timestamp()?;
        if date >= option.data.expiry_date {
            return Err(error!(DexError::DIOptionExpired));
        }

        option.data.premium_rate = premium_rate;
        option.data.stopped = stop;

        Ok(())
    }

    pub fn set_settle_price(&mut self, id: u64, price: u64) -> DexResult {
        let option = self.find_option(id)?;

        let date = get_timestamp()?;
        if date < option.data.expiry_date {
            return Err(error!(DexError::DIOptionNotExpired));
        }

        option.data.stopped = true;
        option.data.settled = true;
        option.data.settle_price = price;

        Ok(())
    }

    pub fn add_settle_size(&mut self, id: u64, size: u64) -> DexResult {
        let option = self.find_option(id)?;

        let date = get_timestamp()?;
        require!(
            date >= option.data.expiry_date,
            DexError::DIOptionNotExpired
        );
        require!(option.data.settled, DexError::DIOptionNotSettled);

        option.data.settle_size += size;

        Ok(())
    }

    pub fn add_volume(&mut self, id: u64, size: u64) -> DexResult {
        let option = self.find_option(id)?;

        let date = get_timestamp()?;
        if date >= option.data.expiry_date - option.data.stop_before_expiry as i64 {
            return Err(error!(DexError::DIOptionExpired));
        }

        option.data.volume += size;

        Ok(())
    }

    pub fn remove(&mut self, id: u64, force: bool) -> DexResult {
        let option = self.find_option(id)?;

        let date = get_timestamp()?;

        if option.data.volume == 0 {
            self.options.remove(option.index)?;
            return Ok(());
        }

        require!(
            date >= option.data.expiry_date,
            DexError::DIOptionNotExpired
        );

        if !force {
            require!(
                option.data.settle_size == option.data.volume,
                DexError::DIOptionNotAllSettled
            );
        }

        self.options.remove(option.index)
    }

    pub fn find_option(&self, id: u64) -> DexResult<&mut SmallListSlot<DIOption>> {
        let lookup = self.options.into_iter().find(|x| x.data.id == id);
        if let Some(p) = lookup {
            return Ok(p);
        }

        return Err(error!(DexError::DIOptionNotFound));
    }

    pub fn get_option(&self, id: u64) -> DexResult<DIOption> {
        Ok(self.find_option(id)?.data)
    }

    #[cfg(feature = "client-support")]
    pub fn collect(&self) -> Vec<DIOption> {
        let mut options: Vec<DIOption> = vec![];

        for o in self.options.into_iter() {
            options.push(o.data);
        }

        options
    }
}
