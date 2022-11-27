use crate::errors::{DexError, DexResult};
use anchor_lang::prelude::*;
use std::cell::RefMut;
use std::marker::PhantomData;
use std::mem;
#[cfg(feature = "client-support")]
use std::mem::ManuallyDrop;

#[repr(C)]
pub struct SingleEventQueueHeader {
    pub magic: u32,
    pub total_raw: u32,
    pub head: u32,
    pub tail: u32,
}

const CRANK_QUEUE_MAGIC: u32 = 0x72047902;
const CRANK_QUEUE_HEADER_SIZE: usize = 16;

#[repr(C)]
pub struct SingleEvent<T> {
    pub data: T,
}

pub struct SingleEventQueue<'a, T> {
    data_ptr: *mut u8,
    data_len: usize,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> SingleEventQueue<'a, T> {
    fn mount_internal(
        data_ptr: *mut u8,
        account_size: usize,
        should_initialized: bool,
    ) -> DexResult<Self> {
        let header = unsafe { data_ptr.cast::<SingleEventQueueHeader>().as_mut() };
        let initialized = if let Some(h) = header {
            h.magic == CRANK_QUEUE_MAGIC
        } else {
            false
        };

        if should_initialized {
            if !initialized {
                return Err(error!(DexError::NotInitialized));
            }
        } else if initialized {
            return Err(error!(DexError::AlreadyInUse));
        }

        Ok(Self {
            data_ptr,
            data_len: account_size - CRANK_QUEUE_HEADER_SIZE as usize,
            phantom: PhantomData,
        })
    }

    pub fn mount(account: &AccountInfo, should_initialized: bool) -> DexResult<Self> {
        let account_size = account.data_len();

        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => return Err(error!(DexError::FailedMountAccount)),
        };

        Self::mount_internal(data_ptr, account_size, should_initialized)
    }

    #[cfg(feature = "client-support")]
    pub fn mount_buf(buf: Vec<u8>) -> DexResult<Self> {
        let (data_ptr, data_size) = {
            let mut me = ManuallyDrop::new(buf);
            (me.as_mut_ptr(), me.len())
        };

        Self::mount_internal(data_ptr, data_size, true)
    }

    pub fn initialize(&mut self) -> DexResult {
        let header = self.header()?;
        if header.magic == CRANK_QUEUE_MAGIC {
            return Err(error!(DexError::AlreadyInUse));
        }

        header.magic = CRANK_QUEUE_MAGIC;
        header.total_raw = (self.data_len / mem::size_of::<SingleEvent<T>>()) as u32;
        header.head = 0;
        header.tail = 0;

        Ok(())
    }

    pub fn new_tail(&mut self) -> DexResult<&'a mut SingleEvent<T>> {
        let header = self.header_ref()?;
        let curr_tail = header.tail;
        let next_tail = (curr_tail + 1) % header.total_raw;
        if next_tail == header.head {
            return Err(error!(DexError::EventQueueFull));
        }

        self.header()?.tail = next_tail;
        self.event_as_mut(self.offset(curr_tail as usize))
    }

    pub fn read_head(&self) -> DexResult<&'a SingleEvent<T>> {
        let header = self.header_ref()?;
        if header.head == header.tail {
            return Err(error!(DexError::EventQueueEmpty));
        }

        self.event_as_ref(self.offset(header.head as usize))
    }

    pub fn remove_head(&mut self) -> DexResult {
        let header = self.header()?;
        if header.head == header.tail {
            return Err(error!(DexError::EventQueueEmpty));
        }

        header.head = (header.head + 1) % header.total_raw;

        Ok(())
    }

    pub fn size(&self) -> DexResult<usize> {
        let header = self.header_ref()?;
        let size = if header.tail >= header.head {
            header.tail - header.head
        } else {
            header.tail + header.total_raw - header.head
        } as usize;

        Ok(size)
    }

    pub fn header(&self) -> DexResult<&mut SingleEventQueueHeader> {
        match unsafe { self.data_ptr.cast::<SingleEventQueueHeader>().as_mut() } {
            Some(v) => Ok(v),
            None => Err(error!(DexError::InvalidEventQueue)),
        }
    }

    pub fn header_ref(&self) -> DexResult<&SingleEventQueueHeader> {
        match unsafe { self.data_ptr.cast::<SingleEventQueueHeader>().as_ref() } {
            Some(v) => Ok(v),
            None => Err(error!(DexError::InvalidEventQueue)),
        }
    }

    fn offset(&self, slot: usize) -> usize {
        CRANK_QUEUE_HEADER_SIZE + slot * mem::size_of::<SingleEvent<T>>()
    }

    fn event_as_ref(&self, offset: usize) -> DexResult<&'a SingleEvent<T>> {
        match unsafe { self.data_ptr.add(offset).cast::<SingleEvent<T>>().as_ref() } {
            Some(v) => Ok(v),
            None => Err(error!(DexError::InvalidEvent)),
        }
    }

    fn event_as_mut(&mut self, offset: usize) -> DexResult<&'a mut SingleEvent<T>> {
        match unsafe { self.data_ptr.add(offset).cast::<SingleEvent<T>>().as_mut() } {
            Some(v) => Ok(v),
            None => Err(error!(DexError::InvalidEvent)),
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::utils::test::*;
    use bumpalo::Bump;

    #[repr(C)]
    pub struct MockEvent {
        pub amount: u64,
        pub size: u32,
        pub asset: u8,
        pub padding: [u8; 3],
    }

    #[test]
    fn test_crank_queue_mount_and_initialize() {
        let bump = Bump::new();
        let account = gen_account(1024, &bump);

        let mut q = SingleEventQueue::<MockEvent>::mount(&account, false).assert_unwrap();
        assert!(q.data_len == 1024usize - CRANK_QUEUE_HEADER_SIZE);

        q.initialize().assert_ok();
        SingleEventQueue::<MockEvent>::mount(&account, false).assert_err();

        let q = SingleEventQueue::<MockEvent>::mount(&account, true).assert_unwrap();
        assert_eq!(q.size().assert_unwrap(), 0);
        assert_eq!(
            q.header().assert_unwrap().total_raw as usize,
            (1024usize - CRANK_QUEUE_HEADER_SIZE) / std::mem::size_of::<SingleEvent<MockEvent>>()
        );
    }

    #[test]
    fn test_crank_queue_read_back() {
        let bump = Bump::new();
        let account_size = 100;
        let account = gen_account(account_size, &bump);

        let mut q = SingleEventQueue::<MockEvent>::mount(&account, false).assert_unwrap();
        q.initialize().assert_ok();

        let new_event = q.new_tail().assert_unwrap();
        new_event.data.amount = 10000;
        new_event.data.size = 1;
        new_event.data.asset = 0xff;

        assert_eq!(q.size().assert_unwrap(), 1);
        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.head, 0);
        assert_eq!(header.tail, 1);

        let read_event = q.read_head().assert_unwrap();
        assert_eq!(read_event.data.amount, 10000);
        assert_eq!(read_event.data.size, 1);
        assert_eq!(read_event.data.asset, 0xff);
    }

    #[test]
    fn test_crank_queue_read_write_batch() {
        let bump = Bump::new();
        let account_size = 64 * 1024;
        let account = gen_account(account_size, &bump);

        let mut q = SingleEventQueue::<MockEvent>::mount(&account, false).assert_unwrap();
        q.initialize().assert_ok();

        let max_events = (account_size - CRANK_QUEUE_HEADER_SIZE)
            / std::mem::size_of::<SingleEvent<MockEvent>>()
            - 1;

        for i in 0..max_events {
            let new_event = q.new_tail().assert_unwrap();
            new_event.data.amount = 10000 + i as u64;
            new_event.data.size = 1 + i as u32;
            new_event.data.asset = i as u8;
        }

        assert_eq!(q.size().assert_unwrap(), max_events);
        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.head, 0);
        assert_eq!(header.tail, max_events as u32);

        for i in 0..max_events {
            let read_event = q.read_head().assert_unwrap();
            assert_eq!(read_event.data.amount, 10000 + i as u64);
            assert_eq!(read_event.data.size, 1 + i as u32);
            assert_eq!(read_event.data.asset, i as u8);

            q.remove_head().assert_ok();
        }

        assert_eq!(q.size().assert_unwrap(), 0);
    }

    #[test]
    fn test_crank_queue_u_turn() {
        let bump = Bump::new();
        let account_size = 64 * 1024;
        let account = gen_account(account_size, &bump);

        let mut q = SingleEventQueue::<MockEvent>::mount(&account, false).assert_unwrap();
        q.initialize().assert_ok();

        let max_events = (account_size - CRANK_QUEUE_HEADER_SIZE)
            / std::mem::size_of::<SingleEvent<MockEvent>>()
            - 1;

        // 1. Make queue full
        for _ in 0..max_events {
            q.new_tail().assert_unwrap();
        }

        // 2. Can add not more
        q.new_tail().assert_err();

        assert_eq!(q.size().assert_unwrap(), max_events);
        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.head, 0);
        assert_eq!(header.tail, max_events as u32);

        // 3. Remove one from head
        q.remove_head().assert_ok();

        // 4. One event slot is available
        q.new_tail().assert_ok();
        q.new_tail().assert_err();
        assert_eq!(q.size().assert_unwrap(), max_events);
        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.head, 1);
        assert_eq!(header.tail, 0);
    }
}
