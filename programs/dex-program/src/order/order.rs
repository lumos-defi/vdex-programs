use crate::{
    collections::{orderbook::*, PagedList, PagedListSlot},
    errors::DexResult,
    utils::{SafeMath, NIL32},
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Order {
    pub user: [u8; 32],
    pub user_state: [u8; 32],
    pub price: u64,
    pub size: u64,
    pub price_node: u16,
    pub user_order_slot: u8,
    padding: u8,
    next: u32,
    prev: u32,
}

impl Order {
    pub fn init(&mut self, price: u64, size: u64, user: [u8; 32], user_state: [u8; 32]) {
        self.price = price;
        self.size = size;
        self.user = user;
        self.user_state = user_state;
    }

    pub fn set_extra_slot(&mut self, price_node: u16, user_order_slot: u8) {
        self.price_node = price_node;
        self.user_order_slot = user_order_slot;
    }
}

impl LinkedOrder<Order> for PagedListSlot<Order> {
    #[inline]
    fn index(&self) -> u32 {
        self.index()
    }

    #[inline]
    fn size(&self) -> u64 {
        self.data.size
    }

    #[inline]
    fn price(&self) -> u64 {
        self.data.price
    }

    #[inline]
    fn price_node(&self) -> u16 {
        self.data.price_node
    }

    #[inline]
    fn fill(&mut self, size: u64) -> DexResult {
        self.data.size = self.data.size.safe_sub(size)?;
        Ok(())
    }

    fn detach(
        &mut self,
        pool: &PagedList<Order>,
        ol_head: u32,
        ol_tail: u32,
    ) -> DexResult<(u32, u32)> {
        let next = pool.from_index(self.data.next);
        let prev = pool.from_index(self.data.prev);

        let mut order_head = ol_head;
        let mut order_tail = ol_tail;

        match next {
            Ok(n) => match prev {
                Ok(p) => {
                    p.data.next = n.index();
                    n.data.prev = p.index();
                }
                Err(_) => {
                    // Has next item, no prev item, it's queue head
                    n.data.prev = NIL32;
                    order_head = n.index();
                }
            },
            Err(_) => match prev {
                Ok(p) => {
                    // Has prev item, no next item, it's queue head
                    p.data.next = NIL32;
                    order_tail = p.index();
                }
                Err(_) => {
                    order_tail = NIL32;
                    order_head = NIL32;
                }
            },
        }

        Ok((order_head, order_tail))
    }

    fn attach(
        &mut self,
        pool: &PagedList<Order>,
        price_node: u16,
        ol_head: u32,
        ol_tail: u32,
    ) -> DexResult<(u32, u32)> {
        let mut order_head = ol_head;
        let mut order_tail = ol_tail;

        let tail = pool.from_index(order_tail);
        match tail {
            Ok(t) => {
                self.data.next = NIL32;
                self.data.prev = t.index();

                t.data.next = self.index();
                order_tail = self.index();
            }
            Err(_) => {
                // Empty list
                order_tail = self.index();
                order_head = self.index();

                self.data.prev = NIL32;
                self.data.next = NIL32;
            }
        }

        self.data.price_node = price_node;

        Ok((order_head, order_tail))
    }
}

pub fn select_side(open: bool, long: bool) -> OrderSide {
    if open {
        if long {
            OrderSide::BID
        } else {
            OrderSide::ASK
        }
    } else {
        if long {
            OrderSide::ASK
        } else {
            OrderSide::BID
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::{
        collections::MountMode,
        errors::DexError,
        utils::{
            test::{gen_account, TestResult},
            ORDER_POOL_MAGIC_BYTE,
        },
    };
    use bumpalo::Bump;
    use rand::prelude::*;

    #[test]
    fn test_order_pool() {
        let bump = Bump::new();
        let account = gen_account(1024 * 16, &bump);

        PagedList::<Order>::mount(&account, &[], ORDER_POOL_MAGIC_BYTE, MountMode::Initialize)
            .map_err(|_| DexError::FailedInitOrderPool)
            .assert_ok();

        let order_pool =
            PagedList::<Order>::mount(&account, &[], ORDER_POOL_MAGIC_BYTE, MountMode::ReadWrite)
                .map_err(|_| DexError::FailedMountOrderPool)
                .assert_unwrap();

        let mut test_loop = 0;
        let mut slot_index_list: Vec<u32> = Vec::new();

        loop {
            let new_order_count = rand::thread_rng().gen_range(0, 20);
            if new_order_count == 0 {
                continue;
            }
            for _ in 0..new_order_count {
                let order = order_pool
                    .new_slot()
                    .map_err(|_| DexError::NoFreeSlotInOrderPool)
                    .assert_unwrap();

                assert!(
                    !slot_index_list.iter().any(|&x| x == order.index()),
                    "Found dup slot index."
                );

                // println!("New slot {}", order.index());

                slot_index_list.push(order.index());
            }

            let cancel_order_count =
                rand::thread_rng().gen_range(slot_index_list.len() >> 1, slot_index_list.len());

            for _ in 0..cancel_order_count {
                let index = slot_index_list
                    .pop()
                    .unwrap_or_else(|| panic!("Slot list is empty"));
                order_pool
                    .release_slot(index)
                    .map_err(|_| DexError::PageLinkedListError)
                    .assert_ok();

                // println!("\tCancel slot {}", index);
            }

            test_loop += 1;
            if test_loop > 30 {
                break;
            }
            // println!("\t\tExisting order count {}", slot_index_list.len());
        }
    }
}
