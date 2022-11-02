use crate::errors::{DexError, DexResult};
use crate::utils::constant::NIL8;
use anchor_lang::prelude::*;
use std::marker::PhantomData;
use std::mem;

const SMALL_LIST_MAGIC: u8 = 0x3F;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct SmallListHeader {
    magic: u8,
    total_raw: u8,
    next_raw: u8,
    top_free: u8,
    head: u8,
    tail: u8,
    padding: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SmallListSlot<T> {
    pub data: T,
    pub index: u8,
    pub next: u8,
    pub prev: u8,
    in_use: u8,
    padding: [u8; 4],
}

impl<T> SmallListSlot<T> {
    #[inline]
    fn set_next(&mut self, next: u8) {
        self.next = next;
    }

    #[inline]
    fn set_prev(&mut self, prev: u8) {
        self.prev = prev;
    }

    #[inline]
    fn set_index(&mut self, index: u8) {
        self.index = index;
    }

    #[inline]
    fn set_in_use(&mut self, in_use: bool) {
        if in_use {
            self.in_use = 1;
        } else {
            self.in_use = 0;
        }
    }

    #[inline]
    pub fn in_use(&self) -> bool {
        self.in_use == 1
    }
}

#[derive(Debug)]
pub struct SmallList<'a, T> {
    data_ptr: *mut u8,
    data_len: usize,
    header: &'a mut SmallListHeader,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> IntoIterator for &'a SmallList<'_, T>
where
    T: Copy,
{
    type Item = &'a mut SmallListSlot<T>;
    type IntoIter = SmallListIntoIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        SmallListIntoIterator {
            list: self,
            curr: self.head(),
        }
    }
}

pub struct SmallListIntoIterator<'a, T> {
    list: &'a SmallList<'a, T>,
    curr: u8,
}

impl<'a, T> SmallListIntoIterator<'a, T> {
    pub fn set_curr(&mut self, curr: u8) {
        self.curr = curr;
    }
}

impl<'a, T> Iterator for SmallListIntoIterator<'a, T>
where
    T: Copy,
{
    type Item = &'a mut SmallListSlot<T>;
    fn next(&mut self) -> Option<&'a mut SmallListSlot<T>> {
        match self.list.from_index(self.curr) {
            Ok(n) => {
                self.curr = n.next;
                Some(n)
            }
            Err(_) => None,
        }
    }
}

impl<'a, T> SmallList<'a, T> {
    pub fn mount(data_ptr: *mut u8, slot_count: u8, should_initialized: bool) -> DexResult<Self> {
        let header = unsafe { data_ptr.cast::<SmallListHeader>().as_mut() };
        let initialized = if let Some(h) = header {
            // require!(h.total_raw == slot_count, DexError::NotInitialized);
            h.magic == SMALL_LIST_MAGIC
        } else {
            false
        };

        if should_initialized {
            require!(initialized, DexError::NotInitialized);
        } else {
            require!(!initialized, DexError::AlreadyInUse);
        }

        Ok(Self {
            data_ptr,
            data_len: Self::required_data_len(slot_count),
            header: unsafe { data_ptr.cast::<SmallListHeader>().as_mut() }
                .ok_or(DexError::AlreadyInUse)?,
            phantom: PhantomData,
        })
    }

    pub fn required_data_len(slot_count: u8) -> usize {
        slot_count as usize * mem::size_of::<SmallListSlot<T>>() + mem::size_of::<SmallListHeader>()
    }

    fn header(&self) -> DexResult<&mut SmallListHeader> {
        match unsafe { self.data_ptr.cast::<SmallListHeader>().as_mut() } {
            Some(v) => Ok(v),
            None => Err(error!(DexError::InvalidListHeader)),
        }
    }

    pub fn initialize(&self) -> DexResult {
        let header = self.header()?;
        header.magic = SMALL_LIST_MAGIC;
        header.total_raw = ((self.data_len - mem::size_of::<SmallListHeader>())
            / mem::size_of::<SmallListSlot<T>>()) as u8;

        header.next_raw = 0;
        header.top_free = NIL8;

        header.head = NIL8;
        header.tail = NIL8;

        Ok(())
    }

    pub fn new_slot(&self) -> DexResult<&'a mut SmallListSlot<T>> {
        let header = self.header()?;

        let slot = if header.next_raw < header.total_raw {
            let offset = mem::size_of::<SmallListHeader>()
                + (header.next_raw as usize) * mem::size_of::<SmallListSlot<T>>();

            let raw_slot = unsafe {
                self.data_ptr
                    .add(offset)
                    .cast::<SmallListSlot<T>>()
                    .as_mut()
                    .unwrap()
            };

            raw_slot.set_index(self.header.next_raw);

            header.next_raw += 1;
            raw_slot
        } else {
            if header.top_free >= header.total_raw {
                return Err(error!(DexError::InvalidIndex));
            }

            let offset = mem::size_of::<SmallListHeader>()
                + (header.top_free as usize) * mem::size_of::<SmallListSlot<T>>();
            let free_slot = unsafe {
                self.data_ptr
                    .add(offset)
                    .cast::<SmallListSlot<T>>()
                    .as_mut()
                    .unwrap()
            };

            header.top_free = free_slot.next;
            free_slot
        };

        slot.next = NIL8;
        slot.prev = NIL8;
        slot.set_in_use(true);

        Ok(slot)
    }

    pub fn new_slot_by_index(&self, index: u8) -> DexResult<&'a mut SmallListSlot<T>> {
        let slot = self.new_slot()?;
        if index != NIL8 && slot.index != index {
            self.release_slot(slot.index)?;
            return Err(error!(DexError::SmallListSlotInUse));
        }

        Ok(slot)
    }

    pub fn release_slot(&self, index: u8) -> DexResult {
        let header = self.header()?;

        if index >= header.total_raw {
            return Err(error!(DexError::InvalidIndex));
        }

        let offset = mem::size_of::<SmallListHeader>()
            + (index as usize) * mem::size_of::<SmallListSlot<T>>();
        let node = unsafe {
            self.data_ptr
                .add(offset)
                .cast::<SmallListSlot<T>>()
                .as_mut()
                .unwrap()
        };

        node.set_next(header.top_free);
        node.set_prev(NIL8);
        node.set_in_use(false);

        header.top_free = index;

        Ok(())
    }

    #[cfg(test)]
    pub fn get_next_free_slot(&self) -> DexResult<u8> {
        let header = self.header()?;

        let next_free = if header.next_raw < header.total_raw {
            header.next_raw
        } else {
            if header.top_free >= header.total_raw {
                return Err(error!(DexError::InvalidIndex));
            }

            header.top_free
        };

        Ok(next_free)
    }

    pub fn from_index(&self, index: u8) -> DexResult<&mut SmallListSlot<T>> {
        if index >= self.header.total_raw {
            return Err(error!(DexError::InvalidIndex));
        }

        let offset = mem::size_of::<SmallListHeader>()
            + (index as usize) * mem::size_of::<SmallListSlot<T>>();
        let node = unsafe {
            self.data_ptr
                .add(offset)
                .cast::<SmallListSlot<T>>()
                .as_mut()
                .unwrap()
        };

        Ok(node)
    }

    pub fn from_index_as_ref(&self, index: u8) -> DexResult<&SmallListSlot<T>> {
        if index >= self.header.total_raw {
            return Err(error!(DexError::InvalidIndex));
        }

        let offset = mem::size_of::<SmallListHeader>()
            + (index as usize) * mem::size_of::<SmallListSlot<T>>();
        let node = unsafe {
            self.data_ptr
                .add(offset)
                .cast::<SmallListSlot<T>>()
                .as_ref()
                .unwrap()
        };

        Ok(node)
    }

    #[inline]
    pub fn head(&self) -> u8 {
        self.header.head
    }

    #[inline]
    pub fn tail(&self) -> u8 {
        self.header.tail
    }

    #[inline]
    pub fn set_head(&self, head: u8) -> DexResult {
        let header = self.header()?;
        header.head = head;

        Ok(())
    }

    #[inline]
    pub fn set_tail(&self, tail: u8) -> DexResult {
        let header = self.header()?;
        header.tail = tail;

        Ok(())
    }

    #[inline]
    pub fn data_len(&self) -> usize {
        self.data_len
    }

    pub fn add_to_list_tail(
        &self,
        head: u8,
        tail: u8,
        slot: &mut SmallListSlot<T>,
    ) -> DexResult<(u8, u8)> {
        if tail >= self.header.total_raw {
            slot.set_next(NIL8);
            return Ok((slot.index, slot.index));
        }

        let tail_node = self.from_index(tail)?;
        tail_node.set_next(slot.index);

        slot.set_prev(tail);
        slot.set_next(NIL8);

        Ok((head, slot.index))
    }

    pub fn add_to_tail(&self, slot: &mut SmallListSlot<T>) -> DexResult {
        if self.tail() >= self.header.total_raw {
            slot.set_next(NIL8);

            self.set_head(slot.index)?;
            self.set_tail(slot.index)?;

            return Ok(());
        }

        let tail = self.from_index(self.tail())?;
        tail.set_next(slot.index);

        slot.set_prev(tail.index);
        slot.set_next(NIL8);

        self.set_tail(slot.index)
    }

    pub fn remove_from_list(&self, head: u8, tail: u8, index: u8) -> DexResult<(u8, u8)> {
        let slot = self.from_index(index)?;
        if slot.prev == NIL8 {
            // Remove head
            if slot.next == NIL8 {
                // The only one slot
                if head == slot.index && tail == slot.index {
                    self.release_slot(slot.index)?;
                    return Ok((NIL8, NIL8));
                } else {
                    return Ok((head, tail));
                }
            }

            if slot.index != head {
                return Ok((head, tail));
            }

            let next_slot = self.from_index(slot.next)?;
            next_slot.set_prev(NIL8);
            self.release_slot(slot.index)?;

            Ok((next_slot.index, tail))
        } else if slot.next == NIL8 {
            // Remove tail
            if slot.prev == NIL8 {
                // The only one slot
                if head == slot.index && tail == slot.index {
                    self.release_slot(slot.index)?;
                    return Ok((NIL8, NIL8));
                } else {
                    return Ok((head, tail));
                }
            }

            if slot.index != tail {
                return Ok((head, tail));
            }

            let prev_slot = self.from_index(slot.prev)?;
            prev_slot.set_next(NIL8);
            self.release_slot(slot.index)?;

            Ok((head, prev_slot.index))
        } else {
            // TODO: how to maket sure slot(index) is on the list(head,tail)??
            let next_slot = self.from_index(slot.next)?;
            let prev_slot = self.from_index(slot.prev)?;

            if next_slot.prev != slot.index || prev_slot.next != slot.index {
                return Ok((head, tail));
            }

            next_slot.set_prev(prev_slot.index);
            prev_slot.set_next(next_slot.index);

            self.release_slot(slot.index)?;

            Ok((head, tail))
        }
    }

    pub fn remove(&self, index: u8) -> DexResult {
        let (head, tail) = self.remove_from_list(self.head(), self.tail(), index)?;
        self.set_head(head)?;
        self.set_tail(tail)
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use std::cell::RefMut;

    use super::*;
    use crate::utils::unit_test::*;
    use bumpalo::Bump;

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct Position {
        size: u64,
        price: u64,
    }

    const LARGE_LIST: u8 = 128;
    fn create_list<'a>(slots: u8) -> SmallList<'a, Position> {
        let bump = Bump::new();
        let actual_size = mem::size_of::<SmallListHeader>()
            + slots as usize * mem::size_of::<SmallListSlot<Position>>();

        let account = gen_account(actual_size, &bump);
        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => panic!("Can not get account data ptr."),
        };

        let list = SmallList::<Position>::mount(data_ptr, slots, false).assert_unwrap();
        list.initialize().assert_ok();
        list
    }

    fn link_slot(list: &SmallList<Position>, size: u64, price: u64) {
        let slot = list.new_slot().assert_unwrap();
        slot.data.size = size;
        slot.data.price = price;
        let (head, tail) = list
            .add_to_list_tail(list.head(), list.tail(), slot)
            .assert_unwrap();
        list.set_head(head).assert_ok();
        list.set_tail(tail).assert_ok();
    }

    fn assert_slot(list: &SmallList<Position>, index: u8, size: u64, price: u64) {
        let slot = list.from_index(index).assert_unwrap();
        assert_eq!(slot.data.size, size);
        assert_eq!(slot.data.price, price);
    }

    fn assert_slot_pointer(list: &SmallList<Position>, index: u8, next: u8, prev: u8) {
        let slot = list.from_index(index).assert_unwrap();
        assert_eq!(slot.next, next);
        assert_eq!(slot.prev, prev);
    }

    fn remove(list: &SmallList<Position>, index: u8) {
        let (head, tail) = list
            .remove_from_list(list.head(), list.tail(), index)
            .assert_unwrap();
        list.set_head(head).assert_ok();
        list.set_tail(tail).assert_ok();
    }

    #[test]
    fn test_list_mount_and_initialize() {
        let bump = Bump::new();
        let account = gen_account(10 * 1024, &bump);
        let data_ptr = match account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => panic!("Can not get account data ptr."),
        };

        let list = SmallList::<u8>::mount(data_ptr, 20, false);
        list.assert_unwrap().initialize().assert_ok();

        SmallList::<u8>::mount(data_ptr, 32, true).assert_ok();
    }

    #[test]
    fn test_list_init_state() {
        let list = create_list(10);
        assert_eq!(list.header.total_raw, 10);
        assert_eq!(list.header.next_raw, 0);
        assert_eq!(list.header.top_free, NIL8);
        assert_eq!(list.header.head, NIL8);
        assert_eq!(list.header.tail, NIL8);
    }

    #[test]
    fn test_list_get_item() {
        let list = create_list(LARGE_LIST);

        let slot_0 = list.new_slot();
        assert!(slot_0.is_ok());
        assert_eq!(slot_0.unwrap().index, 0);

        let slot_1 = list.new_slot().unwrap();
        slot_1.data.size = 500;
        slot_1.data.price = 900;

        let slot_1_readback = list.from_index(1).unwrap();
        assert_eq!(slot_1_readback.data.size, 500);
        assert_eq!(slot_1_readback.data.price, 900);
        assert_eq!(slot_1_readback.index, 1);
    }

    #[test]
    fn test_list_no_more_free_item() {
        let list = create_list(10);
        for n in 0..10 {
            link_slot(&list, n, 10);
        }

        list.new_slot().assert_err();
    }

    #[test]
    fn test_list_push_tail() {
        let list = create_list(LARGE_LIST);

        link_slot(&list, 1, 10);
        link_slot(&list, 2, 20);
        link_slot(&list, 3, 30);
        link_slot(&list, 4, 40);

        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 3);

        assert_slot(&list, 0, 1, 10);
        assert_slot(&list, 1, 2, 20);
        assert_slot(&list, 2, 3, 30);
        assert_slot(&list, 3, 4, 40);

        assert_slot_pointer(&list, 0, 1, NIL8);
        assert_slot_pointer(&list, 1, 2, 0);
        assert_slot_pointer(&list, 2, 3, 1);
        assert_slot_pointer(&list, 3, NIL8, 2);
    }

    #[test]
    fn test_list_remove_head() {
        let list = create_list(LARGE_LIST);

        link_slot(&list, 1, 10);
        link_slot(&list, 2, 20);
        link_slot(&list, 3, 30);
        link_slot(&list, 4, 40);

        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 3);

        remove(&list, list.head());
        assert_eq!(list.head(), 1);
        assert_eq!(list.tail(), 3);

        remove(&list, list.head());
        assert_eq!(list.head(), 2);
        assert_eq!(list.tail(), 3);

        remove(&list, list.head());
        assert_eq!(list.head(), 3);
        assert_eq!(list.tail(), 3);

        remove(&list, list.head());
        assert_eq!(list.head(), NIL8);
        assert_eq!(list.tail(), NIL8);
    }

    #[test]
    fn test_list_remove_tail() {
        let list = create_list(LARGE_LIST);

        link_slot(&list, 1, 10);
        link_slot(&list, 2, 20);
        link_slot(&list, 3, 30);
        link_slot(&list, 4, 40);

        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 3);

        remove(&list, list.tail());
        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 2);

        remove(&list, list.tail());
        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 1);

        remove(&list, list.tail());
        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 0);

        remove(&list, list.tail());
        assert_eq!(list.head(), NIL8);
        assert_eq!(list.tail(), NIL8);
    }

    #[test]
    fn test_list_remove_middle() {
        let list = create_list(LARGE_LIST);

        link_slot(&list, 1, 10);
        link_slot(&list, 2, 20);
        link_slot(&list, 3, 30);
        link_slot(&list, 4, 40);

        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 3);

        remove(&list, 1);
        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 3);
        assert_slot_pointer(&list, 0, 2, NIL8);
        assert_slot_pointer(&list, 2, 3, 0);
        assert_slot_pointer(&list, 3, NIL8, 2);

        remove(&list, 2);
        assert_eq!(list.head(), 0);
        assert_eq!(list.tail(), 3);
        assert_slot_pointer(&list, 0, 3, NIL8);
        assert_slot_pointer(&list, 3, NIL8, 0);
    }

    #[test]
    fn test_list_new_slot_from_stack() {
        let list = create_list(5);
        for n in 0..5 {
            // New slot from raw slot
            link_slot(&list, n, 10);
        }

        // Remove all
        for _ in 0..5 {
            remove(&list, list.tail());
        }

        // New slot from raw slot
        for n in 0..5 {
            link_slot(&list, n, 10 * n);
        }

        for n in 0..5 {
            assert_slot(&list, n, n as u64, 10 * n as u64);
        }

        // No more free slot
        list.new_slot().assert_err();
    }

    #[test]
    fn test_list_free_slot_stack() {
        let list = create_list(5);
        for n in 0..5 {
            // New slot from raw slot
            link_slot(&list, n, 10);
        }
        // No more free slot
        list.new_slot().assert_err();

        // Free slot stack is empty
        assert_eq!(list.header.top_free, NIL8);

        // Remove randomly
        remove(&list, 2);
        assert_eq!(list.header.top_free, 2);

        remove(&list, 3);
        assert_eq!(list.header.top_free, 3);

        remove(&list, 4);
        assert_eq!(list.header.top_free, 4);

        remove(&list, 0);
        assert_eq!(list.header.top_free, 0);

        remove(&list, 1);
        assert_eq!(list.header.top_free, 1);
    }

    #[test]
    fn test_list_iterator() {
        let list = create_list(LARGE_LIST);

        link_slot(&list, 1, 10);
        link_slot(&list, 2, 20);
        link_slot(&list, 3, 30);
        link_slot(&list, 4, 40);

        assert_eq!(list.into_iter().count(), 4);

        for n in list.into_iter() {
            println!("slot: {} {}", n.data.size, n.data.price);
        }

        let first = list
            .into_iter()
            .filter(|x| x.data.size == 1)
            .take(1)
            .next()
            .unwrap();

        assert_eq!(first.data.size, 1);
        assert_eq!(first.data.price, 10);
    }
}
