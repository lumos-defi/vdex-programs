pub mod errors;
mod tests;

use std::marker::Copy;
use std::{cell::RefMut, marker::PhantomData, mem};

use anchor_lang::prelude::{AccountInfo, Pubkey};
use std::convert::TryFrom;

use self::errors::Error;
use super::MountMode;

#[cfg(feature = "client-support")]
use std::mem::ManuallyDrop;

const MAGIC_HEADER: u32 = 0xd1c34400;
const NIL_LIST_INDEX: PagedListIndex = PagedListIndex {
    page_no: 0xffff,
    offset: 0xffff,
};
const PAGE_NIL: Pubkey = Pubkey::new_from_array([0; 32]);

/*
memory shape:
list_header slot0 slot1 slot2 slot3 ... | page_header slot4 slot5 slot6 ... | page_header slot7 slot8 slot9 ...

*/

struct PageHeaderObj(u32, u16, u16, Pubkey);
trait PageHeader {
    fn set(&mut self, magic: u32, page_no: u16, total_raw: u16, next_page: Pubkey);
    fn get(&self) -> PageHeaderObj;
}
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct RemainingPageHeader {
    magic: u32,
    page_no: u16,
    total_raw: u16,
    next_page: Pubkey,
}
impl PageHeader for RemainingPageHeader {
    fn set(&mut self, magic: u32, page_no: u16, total_raw: u16, next_page: Pubkey) {
        self.magic = magic;
        self.page_no = page_no;
        self.total_raw = total_raw;
        self.next_page = next_page;
    }
    fn get(&self) -> PageHeaderObj {
        PageHeaderObj(self.magic, self.page_no, self.total_raw, self.next_page)
    }
}

#[derive(Copy, Clone, Debug)]
struct Page<'a, TSlot> {
    account_ptr: *mut u8,
    page_no: u16,
    data_ptr: *mut u8,
    count: u16,
    phantom: PhantomData<&'a TSlot>,
}

impl<'a, TSlot> Page<'a, TSlot> {
    #[cfg(feature = "client-support")]
    fn mount_buf<THeader>(
        buf: Vec<u8>,
        expected_magic: u32,
        expected_page_no: u16,
        expected_next_page: Pubkey,
    ) -> Result<Self, Error>
    where
        THeader: PageHeader,
    {
        let account_raw_size = buf.len();
        let account_ptr = ManuallyDrop::new(buf).as_mut_ptr();

        Page::internal_mount::<THeader>(
            account_ptr,
            account_raw_size,
            MountMode::ReadWrite,
            expected_magic,
            expected_page_no,
            expected_next_page,
        )
    }

    fn internal_mount<THeader>(
        account_ptr: *mut u8,
        account_raw_size: usize,
        mode: MountMode,
        expected_magic: u32,
        expected_page_no: u16,
        expected_next_page: Pubkey,
    ) -> Result<Self, Error>
    where
        THeader: PageHeader,
    {
        let header_size = mem::size_of::<THeader>();
        let data_size = account_raw_size - header_size;
        let data_ptr = get_offset_cast::<u8>(header_size)(account_ptr)?;
        let PageHeaderObj(existing_magic, page_no, raw_number, next_page) =
            get_offset_cast::<THeader>(0)(account_ptr)?.get();
        let initialized = existing_magic == expected_magic;
        let count = u16::try_from(data_size / mem::size_of::<PagedListSlot<TSlot>>())
            .map_err(|_| Error::TooManyItemsInOnePage)?;

        match mode {
            MountMode::Initialize => {
                if initialized {
                    return Err(Error::AlreadyInUse);
                }
                get_offset_cast::<THeader>(0)(account_ptr)?.set(
                    expected_magic,
                    expected_page_no,
                    count as u16,
                    expected_next_page,
                )
            }
            MountMode::ReadWrite => {
                if !initialized || count != raw_number || page_no != expected_page_no {
                    return Err(Error::PageNotInitialized);
                }
                if expected_next_page != PAGE_NIL && expected_next_page != next_page {
                    return Err(Error::PageNotChained);
                }
            }
        }
        Ok(Self {
            account_ptr,
            data_ptr,
            page_no: expected_page_no,
            count,
            phantom: PhantomData,
        })
    }

    fn mount<THeader>(
        account: &AccountInfo,
        mode: MountMode,
        expected_magic: u32,
        expected_page_no: u16,
        expected_next_page: Pubkey,
    ) -> Result<Self, Error>
    where
        THeader: PageHeader,
    {
        let account_raw_size = account.data_len();

        let account_ptr = try_borrow_mut_data_from_account(account)?;
        Page::internal_mount::<THeader>(
            account_ptr,
            account_raw_size,
            mode,
            expected_magic,
            expected_page_no,
            expected_next_page,
        )
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct PagedListIndex {
    pub page_no: u16,
    pub offset: u16,
}
impl PagedListIndex {
    pub fn new(index: u32) -> Self {
        Self {
            page_no: ((index & 0xffff0000) >> 16) as u16,
            offset: (index & 0x0000ffff) as u16,
        }
    }
    pub fn to_u32(&self) -> u32 {
        ((self.page_no as u32) << 16) | (self.offset as u32)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct PagedListHeader {
    magic: u32,
    last_slot: PagedListIndex,
    next_raw: PagedListIndex,
    top_free: PagedListIndex,
    head: PagedListIndex,
    tail: PagedListIndex,

    // the below is shared between entry page and following pages
    first_page_total_raw: u16,
    next_page: Pubkey,
}
impl PageHeader for PagedListHeader {
    fn set(&mut self, magic: u32, _page_no: u16, total_raw: u16, next_page: Pubkey) {
        self.magic = magic;
        self.first_page_total_raw = total_raw;
        self.next_page = next_page;
    }
    fn get(&self) -> PageHeaderObj {
        PageHeaderObj(self.magic, 0, self.first_page_total_raw, self.next_page)
    }
}

pub struct PagedList<'a, TSlot> {
    pages: Vec<Page<'a, TSlot>>,
    header: &'a mut PagedListHeader,
    phantom: PhantomData<&'a TSlot>,
}

#[derive(Debug, PartialEq)]
pub struct PagedListSlot<TSlot> {
    pub data: TSlot,
    index: PagedListIndex,
    next: PagedListIndex,
    prev: PagedListIndex,
    is_in_use: bool,
    pub padding: [u8; 3],
}

impl<TSlot> PagedListSlot<TSlot> {
    #[inline]
    pub fn index(&self) -> u32 {
        self.index.to_u32()
    }
    #[inline]
    pub fn next(&self) -> u32 {
        self.next.to_u32()
    }
    #[inline]
    pub fn prev(&self) -> u32 {
        self.prev.to_u32()
    }

    #[inline]
    pub fn in_use(&self) -> bool {
        self.is_in_use
    }
}

pub struct PagedListIter<'a, TSlot> {
    list: &'a PagedList<'a, TSlot>,
    curr: PagedListIndex,
}

impl<'a, TSlot> Iterator for PagedListIter<'a, TSlot> {
    type Item = &'a mut PagedListSlot<TSlot>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.list.from_index(self.curr.to_u32()) {
            Ok(item) => {
                self.curr = item.next;
                Some(item)
            }
            Err(_) => None,
        }
    }
}

impl<'a, TSlot> IntoIterator for &'a PagedList<'_, TSlot> {
    type Item = &'a mut PagedListSlot<TSlot>;
    type IntoIter = PagedListIter<'a, TSlot>;

    fn into_iter(self) -> Self::IntoIter {
        PagedListIter {
            list: self,
            curr: self.header.head,
        }
    }
}

impl<'a, TSlot> PagedList<'a, TSlot> {
    #[cfg(feature = "client-support")]
    pub fn mount_buf(
        first_buf: Vec<u8>,
        remaining_bufs: Vec<Vec<u8>>,
        magic_byte: u8,
    ) -> Result<Self, Error> {
        let magic = PagedList::<TSlot>::get_magic(magic_byte);
        let list_header = get_offset_cast::<PagedListHeader>(0)(
            ManuallyDrop::new(first_buf.clone()).as_mut_ptr(),
        )?;
        let initialized = list_header.magic == magic;
        let pages = PagedList::<TSlot>::get_pages_from_bufs(first_buf, remaining_bufs, magic_byte)?;
        if !initialized {
            return Err(Error::NotInitialized);
        }
        Ok(Self {
            pages,
            header: list_header,
            phantom: PhantomData,
        })
    }

    pub fn mount(
        first_account: &AccountInfo,
        remaining_accounts: &[AccountInfo],
        magic_byte: u8,
        mount_mode: MountMode,
    ) -> Result<Self, Error> {
        let magic = PagedList::<TSlot>::get_magic(magic_byte);
        let list_header = try_borrow_mut_data_from_account(first_account)
            .and_then(get_offset_cast::<PagedListHeader>(0))?;
        let initialized = list_header.magic == magic;
        let pages = PagedList::<TSlot>::get_pages(
            first_account,
            remaining_accounts,
            magic_byte,
            mount_mode,
        )?;
        match mount_mode {
            MountMode::Initialize => {
                if initialized {
                    return Err(Error::AlreadyInUse);
                }

                let list = Self {
                    pages,
                    header: list_header,
                    phantom: PhantomData,
                };
                list.initialize(magic_byte)?;
                Ok(list)
            }
            MountMode::ReadWrite => {
                if !initialized {
                    return Err(Error::NotInitialized);
                }
                Ok(Self {
                    pages,
                    header: list_header,
                    phantom: PhantomData,
                })
            }
        }
    }
    pub fn append_pages(
        first_account: &AccountInfo,
        remaining_accounts: &[AccountInfo],
        new_accounts: &[AccountInfo],
        magic_byte: u8,
    ) -> Result<Self, Error> {
        if new_accounts.is_empty() {
            return Err(Error::NoPagesToAppend);
        }
        let mut list = PagedList::<TSlot>::mount(
            first_account,
            remaining_accounts,
            magic_byte,
            MountMode::ReadWrite,
        )?;

        if list.pages.len() == 1 {
            get_offset_cast::<PagedListHeader>(0)(
                list.pages.last().ok_or(Error::InvalidIndex)?.account_ptr,
            )?
            .next_page = *new_accounts.first().ok_or(Error::InvalidIndex)?.key;
        } else {
            get_offset_cast::<RemainingPageHeader>(0)(
                list.pages.last().ok_or(Error::InvalidIndex)?.account_ptr,
            )?
            .next_page = *new_accounts.first().ok_or(Error::InvalidIndex)?.key;
        }

        for i in 0..new_accounts.len() {
            let expected_next_page = if i == new_accounts.len() - 1 {
                PAGE_NIL
            } else {
                *new_accounts[i + 1].key
            };
            {
                list.pages.push(Page::<TSlot>::mount::<RemainingPageHeader>(
                    &new_accounts[i],
                    MountMode::Initialize,
                    PagedList::<TSlot>::get_magic(magic_byte),
                    (i + 1 + remaining_accounts.len()) as u16,
                    expected_next_page,
                )?);
            }
        }

        let list_header = try_borrow_mut_data_from_account(first_account)
            .and_then(get_offset_cast::<PagedListHeader>(0))?;

        list_header.last_slot = PagedListIndex {
            page_no: list.pages.len() as u16,
            offset: list.pages.iter().last().ok_or(Error::InvalidIndex)?.count,
        };

        Ok(list)
    }

    pub fn new_slot(&self) -> Result<&'a mut PagedListSlot<TSlot>, Error> {
        let header = get_offset_cast::<PagedListHeader>(0)(self.pages[0].account_ptr)?;
        let last_page = self.pages.iter().last().ok_or(Error::InvalidIndex)?;
        let slot = if header.next_raw
            < (PagedListIndex {
                page_no: last_page.page_no,
                offset: last_page.count,
            }) {
            let slot = self.get_mut_slot_by_index(header.next_raw)?;
            slot.index = header.next_raw;
            let current_page = header.next_raw.page_no;
            let current_offset = header.next_raw.offset;
            if current_offset + 1 < self.pages[current_page as usize].count {
                header.next_raw.offset += 1
            } else {
                header.next_raw = PagedListIndex {
                    page_no: header.next_raw.page_no + 1,
                    offset: 0,
                };
            }
            slot
        } else {
            if header.top_free == NIL_LIST_INDEX {
                return Err(Error::NoFreeOrRawSlot);
            }
            let slot = self.get_mut_slot_by_index(header.top_free)?;
            header.top_free = slot.next;
            slot
        };
        slot.next = NIL_LIST_INDEX;
        slot.is_in_use = true;

        if header.head == NIL_LIST_INDEX {
            header.head = slot.index;
            header.tail = slot.index;
            slot.prev = NIL_LIST_INDEX;
        } else {
            self.get_mut_slot_by_index(header.tail)?.next = slot.index;
            slot.prev = header.tail;
            header.tail = slot.index
        }

        Ok(slot)
    }

    pub fn release_slot(&self, index: u32) -> Result<(), Error> {
        let list_index = PagedListIndex::new(index);
        if list_index == NIL_LIST_INDEX {
            return Ok(());
        }
        let header = get_offset_cast::<PagedListHeader>(0)(self.pages[0].account_ptr)?;
        if list_index >= header.last_slot {
            return Err(Error::InvalidIndex);
        }

        let slot = self.get_mut_slot_by_index(list_index)?;
        if !slot.is_in_use {
            return Err(Error::SlotNotInUse);
        }
        if slot.prev != NIL_LIST_INDEX {
            self.get_mut_slot_by_index(slot.prev)?.next = slot.next;
        }

        if slot.next != NIL_LIST_INDEX {
            self.get_mut_slot_by_index(slot.next)?.prev = slot.prev;
        }
        if header.head == slot.index {
            header.head = slot.next
        }
        if header.tail == slot.index {
            header.tail = slot.prev
        }
        slot.is_in_use = false;

        slot.prev = NIL_LIST_INDEX;
        if header.top_free == NIL_LIST_INDEX {
            slot.next = NIL_LIST_INDEX;
            header.top_free = list_index;
        } else {
            slot.next = header.top_free;
            self.get_mut_slot_by_index(header.top_free)?.prev = list_index;
            header.top_free = slot.index;
        }

        Ok(())
    }

    pub fn from_index(&self, index: u32) -> Result<&mut PagedListSlot<TSlot>, Error> {
        let list_index = PagedListIndex::new(index);
        if list_index >= self.header.last_slot {
            return Err(Error::InvalidIndex);
        }
        self.get_mut_slot_by_index(list_index)
    }

    //todo: when mount bufs, we don't check pubkeys
    #[cfg(feature = "client-support")]
    fn get_pages_from_bufs(
        first_buf: Vec<u8>,
        remaining_bufs: Vec<Vec<u8>>,
        magic_byte: u8,
    ) -> Result<Vec<Page<'a, TSlot>>, Error> {
        let magic = PagedList::<'a, TSlot>::get_magic(magic_byte);
        let first_page_data =
            Page::<'a, TSlot>::mount_buf::<PagedListHeader>(first_buf, magic, 0, PAGE_NIL)?;
        let mut ret = vec![first_page_data];
        for (i, item) in remaining_bufs.iter().enumerate() {
            ret.push(Page::<'a, TSlot>::mount_buf::<RemainingPageHeader>(
                item.clone(),
                magic,
                i as u16 + 1,
                PAGE_NIL,
            )?);
        }

        Ok(ret)
    }

    fn get_pages(
        first_account: &AccountInfo,
        remaining_accounts: &[AccountInfo],
        magic_byte: u8,
        mount_mode: MountMode,
    ) -> Result<Vec<Page<'a, TSlot>>, Error> {
        let magic = PagedList::<'a, TSlot>::get_magic(magic_byte);
        let expected_next_page_for_first_page = if remaining_accounts.is_empty() {
            PAGE_NIL
        } else {
            *remaining_accounts[0].key
        };

        let first_page_data = Page::<'a, TSlot>::mount::<PagedListHeader>(
            first_account,
            mount_mode,
            magic,
            0,
            expected_next_page_for_first_page,
        )?;

        let mut ret = vec![first_page_data];
        for i in 0..remaining_accounts.len() {
            let expected_next_page = if i == remaining_accounts.len() - 1 {
                PAGE_NIL
            } else {
                *remaining_accounts[i + 1].key
            };
            ret.push(Page::<'a, TSlot>::mount::<RemainingPageHeader>(
                &remaining_accounts[i],
                mount_mode,
                magic,
                i as u16 + 1,
                expected_next_page,
            )?);
        }
        Ok(ret)
    }

    fn get_mut_slot_by_index(
        &self,
        index: PagedListIndex,
    ) -> Result<&'a mut PagedListSlot<TSlot>, Error> {
        let p = self
            .pages
            .get(index.page_no as usize)
            .ok_or(Error::InvalidIndex)?;
        get_offset_cast::<PagedListSlot<TSlot>>(
            (index.offset as usize) * mem::size_of::<PagedListSlot<TSlot>>(),
        )(p.data_ptr)
    }

    #[inline]
    fn initialize(&self, magic_byte: u8) -> Result<(), Error> {
        let header = get_offset_cast::<PagedListHeader>(0)(self.pages[0].account_ptr)?;
        header.magic = PagedList::<TSlot>::get_magic(magic_byte);
        header.last_slot = PagedListIndex {
            page_no: (self.pages.len() - 1) as u16,
            offset: self.pages.iter().last().ok_or(Error::InvalidIndex)?.count,
        };

        header.next_raw = PagedListIndex::new(0);
        header.top_free = NIL_LIST_INDEX;
        header.head = NIL_LIST_INDEX;
        header.tail = NIL_LIST_INDEX;

        Ok(())
    }

    #[inline]
    fn get_magic(magic_byte: u8) -> u32 {
        MAGIC_HEADER | magic_byte as u32
    }
}

#[inline]
fn try_borrow_mut_data_from_account(account: &AccountInfo) -> Result<*mut u8, Error> {
    account
        .try_borrow_mut_data()
        .map_or(Err(Error::CannotBorrowFromAccount), |p| {
            Ok(RefMut::map(p, |data| *data).as_mut_ptr())
        })
}

#[inline]
fn get_offset_cast<'a, T>(offset: usize) -> impl Fn(*mut u8) -> Result<&'a mut T, Error> {
    move |ptr| unsafe {
        ptr.add(offset as usize)
            .cast::<T>()
            .as_mut()
            .ok_or(Error::InvalidListHeader)
    }
}
