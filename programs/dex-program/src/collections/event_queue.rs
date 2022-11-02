use crate::errors::{DexError, DexResult};
use crate::utils::time::get_timestamp;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use core::slice;
use std::cell::RefMut;
use std::cmp::Ordering;
use std::io::{Read, Write};
use std::mem::ManuallyDrop;

#[repr(C)]
pub struct EventQueueHeader {
    pub magic: u32,
    pub head: u32,
    pub tail: u32,
    pub seq: u16,
    pub over_writable: bool,
    padding: u8,
}
const EVENT_QUEUE_MAGIC: u32 = 0x72047901;
const EVENT_QUEUE_HEADER_SIZE: u32 = 24;

#[repr(C)]
pub struct EventQueue<'a> {
    header: &'a mut EventQueueHeader,
    data_ptr: *mut u8,
    data_size: u32,
    read_cursor: u32,
    write_cursor: u32,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct EventHeader {
    pub magic: u32,
    pub seq: u16,
    pub discriminator: u8,
    pub len: u8,
    pub time: i64,
}

const EVENT_HEADER_SIZE: u32 = 16;
const EVENT_HEADER_MAGIC: u32 = 0x6785129f;

pub trait PackedEvent
where
    Self: Sized + AnchorSerialize + AnchorDeserialize,
{
    const DISCRIMINATOR: u8;
}

pub struct EventData {
    buf: Vec<u8>,
}
impl EventData {
    pub fn new(buf: Vec<u8>) -> Self {
        Self { buf }
    }
    pub fn to<PEvent>(&self) -> DexResult<PEvent>
    where
        PEvent: PackedEvent,
    {
        PEvent::deserialize(&mut &self.buf[..]).map_err(|_| error!(DexError::InvalidEvent))
    }
}

impl<'a> Write for EventQueue<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let header = self
            .header_ref()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "read queue header"))?;

        let mut write_cursor = self.write_cursor;
        let mut free_space = if write_cursor >= header.head {
            self.data_size - write_cursor + header.head
        } else {
            header.head - write_cursor
        };

        if header.over_writable {
            free_space = 0xffffffff;
        }

        if free_space < buf.len() as u32 {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "space low"));
        }

        let mut written = 0;
        while written < buf.len() {
            let space = if write_cursor >= header.head || header.over_writable {
                self.data_size - write_cursor
            } else {
                header.head - write_cursor
            } as usize;

            if space == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "space low"));
            }

            let to_write = if space > buf.len() - written {
                buf.len() - written
            } else {
                space
            };

            let slice = unsafe {
                slice::from_raw_parts_mut(
                    self.data_ptr
                        .add((EVENT_QUEUE_HEADER_SIZE + write_cursor) as usize),
                    to_write,
                )
            };

            slice.copy_from_slice(&buf[written..written + to_write]);
            written += to_write;

            write_cursor += to_write as u32;
            if write_cursor >= self.data_size {
                write_cursor = 0;
            }

            if !header.over_writable && write_cursor == header.head {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "space low"));
            }
        }

        self.write_cursor = write_cursor;

        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> Read for EventQueue<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read_cursor = self.read_cursor;

        let header = self
            .header()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "read queue header"))?;

        let readable = match read_cursor.cmp(&header.tail) {
            Ordering::Greater => self.data_size - read_cursor + header.tail,
            Ordering::Less => header.tail - read_cursor,
            Ordering::Equal => 0,
        };

        if readable < buf.len() as u32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "no more to read",
            ));
        }

        let mut read = 0;
        while read < buf.len() {
            let readable = if read_cursor > header.tail {
                self.data_size - read_cursor
            } else {
                header.tail - read_cursor
            } as usize;

            let to_read = if readable > buf.len() - read {
                buf.len() - read
            } else {
                readable
            };

            let slice = unsafe {
                slice::from_raw_parts(
                    self.data_ptr
                        .add((EVENT_QUEUE_HEADER_SIZE + read_cursor) as usize),
                    to_read,
                )
            };

            buf[read..read + to_read].copy_from_slice(slice);
            read += to_read;

            read_cursor += to_read as u32;
            if read_cursor >= self.data_size {
                read_cursor = 0;
            }
        }

        self.read_cursor = read_cursor;

        Ok(read)
    }
}

impl<'a> EventQueue<'a> {
    fn mount_internal(
        data_ptr: *mut u8,
        account_size: usize,
        should_initialized: bool,
    ) -> DexResult<Self> {
        let header = unsafe { data_ptr.cast::<EventQueueHeader>().as_mut() };
        let initialized = if let Some(h) = header {
            h.magic == EVENT_QUEUE_MAGIC
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
            data_size: (account_size - EVENT_QUEUE_HEADER_SIZE as usize) as u32,
            header: unsafe { data_ptr.cast::<EventQueueHeader>().as_mut() }.unwrap(),
            read_cursor: 0,
            write_cursor: 0,
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

    pub fn mount_buf(buf: Vec<u8>) -> DexResult<Self> {
        let (data_ptr, data_size) = {
            let mut me = ManuallyDrop::new(buf);
            (me.as_mut_ptr(), me.len())
        };

        Self::mount_internal(data_ptr, data_size, true)
    }

    pub fn initialize(&mut self, over_writable: bool) -> DexResult {
        let header = self.header()?;
        if header.magic == EVENT_QUEUE_MAGIC {
            return Err(error!(DexError::AlreadyInUse));
        }

        header.magic = EVENT_QUEUE_MAGIC;
        header.head = 0;
        header.tail = 0;
        header.seq = 0;
        header.over_writable = over_writable;

        Ok(())
    }

    pub fn append<PEvent>(&mut self, event: PEvent) -> DexResult<u16>
    where
        PEvent: PackedEvent,
    {
        let queue_header = self.header_ref()?;
        let seq = queue_header.seq;
        let tail = queue_header.tail;

        let mut event_slice: Vec<u8> = Vec::new();
        event
            .serialize(&mut event_slice)
            .map_err(|_| error!(DexError::FailedSerializeEvent))?;

        let event_header = EventHeader {
            magic: EVENT_HEADER_MAGIC,
            discriminator: PEvent::DISCRIMINATOR,
            len: event_slice.len() as u8,
            seq: seq.wrapping_add(1),
            time: get_timestamp()?,
        };

        self.set_write_cursor(tail);
        event_header
            .serialize(self)
            .map_err(|_| error!(DexError::FailedSendEventHeader))?;

        self.write_all(&event_slice[..])
            .map_err(|_| error!(DexError::FailedSendEvent))?;

        let queue_header = self.header()?;
        queue_header.tail = self.write_cursor;
        queue_header.seq = event_header.seq;

        Ok(queue_header.seq)
    }

    pub fn read(&mut self, offset: u32, commit: bool) -> DexResult<(EventHeader, EventData, u32)> {
        let mut event_header_buf: Vec<u8> = vec![0; EVENT_HEADER_SIZE as usize];
        self.set_read_cursor(offset);

        self.read_exact(&mut event_header_buf)?;
        let event_header = EventHeader::deserialize(&mut &event_header_buf[..])?;
        if event_header.magic != EVENT_HEADER_MAGIC {
            return Err(error!(DexError::InvalidEvent));
        }

        let mut event_buf: Vec<u8> = vec![0; event_header.len as usize];
        self.read_exact(&mut event_buf)?;

        if commit {
            self.header()?.head = self.read_cursor;
        }

        Ok((
            event_header,
            EventData::new(event_buf.to_vec()),
            self.read_cursor,
        ))
    }

    pub fn read_and_commit(&mut self) -> DexResult<(u8, EventData)> {
        let head = self.header_ref()?.head;
        let (event_header, data, _) = self.read(head, true)?;

        Ok((event_header.discriminator, data))
    }

    #[inline]
    fn set_read_cursor(&mut self, read_cursor: u32) {
        self.read_cursor = read_cursor;
    }

    #[inline]
    fn set_write_cursor(&mut self, write_cursor: u32) {
        self.write_cursor = write_cursor;
    }

    fn header(&self) -> DexResult<&mut EventQueueHeader> {
        match unsafe { self.data_ptr.cast::<EventQueueHeader>().as_mut() } {
            Some(v) => Ok(v),
            None => Err(error!(DexError::InvalidEventQueue)),
        }
    }

    pub fn header_ref(&self) -> DexResult<&EventQueueHeader> {
        match unsafe { self.data_ptr.cast::<EventQueueHeader>().as_ref() } {
            Some(v) => Ok(v),
            None => Err(error!(DexError::InvalidEventQueue)),
        }
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self.data_size
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::utils::unit_test::*;
    use bumpalo::Bump;

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct MockEvent {
        pub amount: u64,
        pub asset: u8,
        pub size: u32,
    }
    const MOCK_EVENT_SIZE: u32 = 13;

    impl PackedEvent for MockEvent {
        const DISCRIMINATOR: u8 = 0xff;
    }

    #[test]
    fn test_eq_mount_and_initialize() {
        let bump = Bump::new();
        let account = gen_account(1024, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        assert!(q.data_size == 1024 - EVENT_QUEUE_HEADER_SIZE as u32);

        q.initialize(false).assert_ok();
        EventQueue::mount(&account, false).assert_err();

        EventQueue::mount(&account, true).assert_ok();

        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.over_writable, false);
    }

    #[test]
    fn test_eq_serialize() {
        let bump = Bump::new();
        let account = gen_account(1024, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        q.initialize(false).assert_ok();

        q.append(MockEvent {
            amount: 10000,
            asset: 0x08,
            size: 999,
        })
        .assert_ok();

        let (_, event_data) = q.read_and_commit().assert_unwrap();
        let event = event_data.to::<MockEvent>().assert_unwrap();
        assert_eq!(event.amount, 10000);
        assert_eq!(event.asset, 0x08);
        assert_eq!(event.size, 999);

        q.read_and_commit().assert_err();

        q.append(MockEvent {
            amount: 2000,
            asset: 0xf,
            size: 500,
        })
        .assert_ok();

        let (_, event_data) = q.read_and_commit().assert_unwrap();
        let event = event_data.to::<MockEvent>().assert_unwrap();
        assert_eq!(event.amount, 2000);
        assert_eq!(event.asset, 0xf);
        assert_eq!(event.size, 500);
    }

    #[test]
    fn test_eq_write_max_event_data_len_aligned() {
        let bump = Bump::new();
        const MAX_EVENTS: u16 = 256;
        const EVENT_SIZE: u32 = EVENT_HEADER_SIZE + MOCK_EVENT_SIZE;
        let events_data_len = MAX_EVENTS as u32 * EVENT_SIZE;
        let account_data_len = EVENT_QUEUE_HEADER_SIZE + events_data_len;
        let account = gen_account(account_data_len as usize, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        q.initialize(false).assert_ok();

        let mut write = 0;
        loop {
            let result = q.append(MockEvent {
                amount: 10000,
                asset: 0x08,
                size: 999,
            });

            if result.is_err() {
                break;
            }

            write += 1;
        }

        assert_eq!(write, MAX_EVENTS - 1);

        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.head, 0);
        assert_eq!(header.seq, MAX_EVENTS - 1);
        assert_eq!(header.tail, (MAX_EVENTS as u32 - 1) * EVENT_SIZE);
    }

    #[test]
    fn test_eq_write_max_event_data_len_unaligned() {
        let bump = Bump::new();
        const MAX_EVENTS: u16 = 256;
        const EVENT_SIZE: u32 = EVENT_HEADER_SIZE + MOCK_EVENT_SIZE;
        let events_data_len = MAX_EVENTS as u32 * EVENT_SIZE;
        let account_data_len = EVENT_QUEUE_HEADER_SIZE + events_data_len + 1; // one extra byte
        let account = gen_account(account_data_len as usize, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        q.initialize(false).assert_ok();

        let mut write = 0;
        loop {
            let result = q.append(MockEvent {
                amount: 10000,
                asset: 0x08,
                size: 999,
            });

            if result.is_err() {
                break;
            }

            write += 1;
        }

        assert_eq!(write, MAX_EVENTS);

        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.head, 0);
        assert_eq!(header.tail, MAX_EVENTS as u32 * EVENT_SIZE);
    }

    #[test]
    fn test_eq_read_write_event_data_len_aligned() {
        let bump = Bump::new();
        const MAX_EVENTS: u16 = 256;
        const EVENT_SIZE: u32 = EVENT_HEADER_SIZE + MOCK_EVENT_SIZE;
        let events_data_len = MAX_EVENTS as u32 * EVENT_SIZE;
        let account_data_len = EVENT_QUEUE_HEADER_SIZE + events_data_len;
        let account = gen_account(account_data_len as usize, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        q.initialize(false).assert_ok();

        let mut read_write = 3000;
        while read_write > 0 {
            q.append(MockEvent {
                amount: 10010,
                asset: 0x09,
                size: 888,
            })
            .assert_ok();

            let (_, event_data) = q.read_and_commit().assert_unwrap();
            let event = event_data.to::<MockEvent>().assert_unwrap();
            assert_eq!(event.amount, 10010);
            assert_eq!(event.asset, 0x09);
            assert_eq!(event.size, 888);

            read_write -= 1;
        }

        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.head, header.tail);

        q.read_and_commit().assert_err();
    }

    #[test]
    fn test_eq_read_write_event_data_len_unaligned() {
        let bump = Bump::new();
        const MAX_EVENTS: u16 = 256;
        const EVENT_SIZE: u32 = EVENT_HEADER_SIZE + MOCK_EVENT_SIZE;
        let events_data_len = MAX_EVENTS as u32 * EVENT_SIZE;
        let account_data_len = EVENT_QUEUE_HEADER_SIZE + events_data_len + 1; // one extra byte
        let account = gen_account(account_data_len as usize, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        q.initialize(false).assert_ok();

        let mut read_write = 3000;
        while read_write > 0 {
            q.append(MockEvent {
                amount: 10010,
                asset: 0x09,
                size: 888,
            })
            .assert_ok();

            let (_, event_data) = q.read_and_commit().assert_unwrap();
            let event = event_data.to::<MockEvent>().assert_unwrap();
            assert_eq!(event.amount, 10010);
            assert_eq!(event.asset, 0x09);
            assert_eq!(event.size, 888);

            read_write -= 1;
        }

        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.head, header.tail);

        q.read_and_commit().assert_err();
    }

    #[test]
    fn test_eq_read_write_loop_event_data_len_aligned() {
        let bump = Bump::new();
        const MAX_EVENTS: u16 = 256;
        const EVENT_SIZE: u32 = EVENT_HEADER_SIZE + MOCK_EVENT_SIZE;
        let events_data_len = MAX_EVENTS as u32 * EVENT_SIZE;
        let account_data_len = EVENT_QUEUE_HEADER_SIZE + events_data_len;
        let account = gen_account(account_data_len as usize, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        q.initialize(false).assert_ok();

        let mut rw_loop = 0;
        while rw_loop < 512 {
            let mut write = 0;
            loop {
                let result = q.append(MockEvent {
                    amount: 10000,
                    asset: 0x08,
                    size: 999,
                });

                if result.is_err() {
                    break;
                }

                write += 1;
            }

            assert_eq!(write, MAX_EVENTS - 1);

            let mut read = 0;
            loop {
                let result = q.read_and_commit();
                if result.is_err() {
                    break;
                }

                let (_, event_data) = result.unwrap();
                let event = event_data.to::<MockEvent>().assert_unwrap();
                assert_eq!(event.amount, 10000);
                assert_eq!(event.asset, 0x08);
                assert_eq!(event.size, 999);

                read += 1;
            }

            assert_eq!(write, read);
            let header = q.header_ref().assert_unwrap();
            assert_eq!(header.head, header.tail);

            rw_loop += 1;
        }
    }

    #[test]
    fn test_eq_read_write_loop_event_data_len_unaligned() {
        let bump = Bump::new();
        const MAX_EVENTS: u16 = 256;
        const EVENT_SIZE: u32 = EVENT_HEADER_SIZE + MOCK_EVENT_SIZE;
        let events_data_len = MAX_EVENTS as u32 * EVENT_SIZE;
        let account_data_len = EVENT_QUEUE_HEADER_SIZE + events_data_len + 1; // one extra byte
        let account = gen_account(account_data_len as usize, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        q.initialize(false).assert_ok();

        let mut rw_loop = 0;
        while rw_loop < 512 {
            let mut write = 0;
            loop {
                let result = q.append(MockEvent {
                    amount: 10000,
                    asset: 0x08,
                    size: 999,
                });

                if result.is_err() {
                    break;
                }

                write += 1;
            }

            assert_eq!(write, MAX_EVENTS);

            let mut read = 0;
            loop {
                let result = q.read_and_commit();
                if result.is_err() {
                    break;
                }

                let (_, event_data) = result.unwrap();
                let event = event_data.to::<MockEvent>().assert_unwrap();
                assert_eq!(event.amount, 10000);
                assert_eq!(event.asset, 0x08);
                assert_eq!(event.size, 999);

                read += 1;
            }

            assert_eq!(write, read);
            let header = q.header_ref().assert_unwrap();
            assert_eq!(header.head, header.tail);

            rw_loop += 1;
        }
    }

    #[test]
    fn test_eq_write_over_writable_event_data_len_unaligned() {
        let bump = Bump::new();
        const MAX_EVENTS: u16 = 256;
        const EVENT_SIZE: u32 = EVENT_HEADER_SIZE + MOCK_EVENT_SIZE;
        let events_data_len = MAX_EVENTS as u32 * EVENT_SIZE;
        let account_data_len = EVENT_QUEUE_HEADER_SIZE + events_data_len + 1; // one extra byte
        let account = gen_account(account_data_len as usize, &bump);

        let mut q = EventQueue::mount(&account, false).assert_unwrap();
        q.initialize(true).assert_ok();

        let mut write = 0;
        loop {
            let result = q.append(MockEvent {
                amount: 10000,
                asset: 0x08,
                size: 999,
            });

            if result.is_err() {
                assert!(false);
            }

            write += 1;
            if write == 1000 {
                break;
            }
        }

        let header = q.header_ref().assert_unwrap();
        assert_eq!(header.seq, 1000);
    }
}
