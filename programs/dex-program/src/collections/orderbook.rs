use crate::collections::paged_list::{PagedList, PagedListSlot};
use crate::errors::{DexError, DexResult};
use crate::utils::constant::{NIL16, NIL32};
use crate::utils::math::SafeMath;
use anchor_lang::prelude::*;
use num_enum::TryFromPrimitive;
use std::cell::RefMut;
use std::mem::{self, swap};

const RBT_MAGIC: u16 = 0x5600;

#[derive(Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum OrderSide {
    BID = 0,
    ASK = 1,
}

impl OrderSide {
    pub fn opposite(self) -> Self {
        match self {
            OrderSide::BID => OrderSide::ASK,
            OrderSide::ASK => OrderSide::BID,
        }
    }
}

#[derive(Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum OrderType {
    LIMIT = 0,
    MARKET = 1,
}

#[derive(PartialEq, TryFromPrimitive)]
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum MatchStrategy {
    GTC = 0,
    IOC = 1,
    FOK = 2,
    POSTONLY = 3,
}

#[repr(C)]
pub struct RBTHeader {
    magic: u16,
    total_raw: u16,
    next_raw: u16,
    top_free: u16,
    bid_root: u16,
    ask_root: u16,
    bid_maximum: u16,
    ask_minimum: u16,
}

#[repr(C)]
pub struct RBTNode {
    // RBT related fields
    red: u16,
    parent: u16,
    left: u16,
    right: u16,
    index: u16,
    padding: u16,

    // Order queue
    order_head: u32,
    order_tail: u32,
    order_count: u32,

    // Price and total size
    price: u64,
    size: u64,
}

impl RBTNode {
    #[inline]
    fn set_color(&mut self, red: u16) {
        self.red = red;
    }

    #[inline]
    fn set_parent(&mut self, parent: u16) {
        self.parent = parent;
    }

    #[inline]
    fn set_left_child(&mut self, child: u16) {
        self.left = child;
    }

    #[inline]
    fn set_right_child(&mut self, child: u16) {
        self.right = child;
    }

    #[inline]
    fn zero(&mut self) {
        self.red = 1u16;
        self.parent = NIL16;
        self.left = NIL16;
        self.right = NIL16;
        self.price = 0;
        self.size = 0;
        self.order_head = NIL32;
        self.order_tail = NIL32;
        self.order_count = 0;
    }
}

pub trait LinkedOrder<T> {
    fn index(&self) -> u32;
    fn size(&self) -> u64;
    fn price(&self) -> u64;
    fn price_node(&self) -> u16;

    fn fill(&mut self, size: u64) -> DexResult;
    fn detach(&mut self, pool: &PagedList<T>, ol_head: u32, ol_tail: u32) -> DexResult<(u32, u32)>;
    fn attach(
        &mut self,
        pool: &PagedList<T>,
        price_node: u16,
        ol_head: u32,
        ol_tail: u32,
    ) -> DexResult<(u32, u32)>;
}

pub struct OrderBook<'a> {
    rbt_data_ptr: *mut u8,
    rbt_data_size: usize,
    header: &'a mut RBTHeader,
}

impl<'a> OrderBook<'a> {
    pub fn mount(price_account: &AccountInfo, should_initialized: bool) -> DexResult<Self> {
        let rbt_data_size = price_account.data_len();
        let rbt_data_ptr = match price_account.try_borrow_mut_data() {
            Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
            Err(_) => return Err(error!(DexError::FailedMountAccount)),
        };

        let header = unsafe { rbt_data_ptr.cast::<RBTHeader>().as_mut() };

        let initialized = if let Some(th) = header {
            th.magic == RBT_MAGIC
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
            rbt_data_ptr,
            rbt_data_size,
            header: unsafe { rbt_data_ptr.cast::<RBTHeader>().as_mut() }.unwrap(),
        })
    }

    pub fn initialize(&mut self) -> DexResult {
        if self.header.magic == RBT_MAGIC {
            return Err(error!(DexError::AlreadyInUse));
        }
        self.header.magic = RBT_MAGIC;
        self.header.next_raw = 0;
        self.header.top_free = NIL16;
        self.header.total_raw =
            ((self.rbt_data_size - mem::size_of::<RBTHeader>()) / mem::size_of::<RBTNode>()) as u16;
        self.header.bid_root = NIL16;
        self.header.ask_root = NIL16;
        self.header.ask_minimum = NIL16;
        self.header.bid_maximum = NIL16;

        Ok(())
    }

    pub fn initialize_force(&mut self) -> DexResult {
        self.header.magic = RBT_MAGIC;
        self.header.next_raw = 0;
        self.header.top_free = NIL16;
        self.header.total_raw =
            ((self.rbt_data_size - mem::size_of::<RBTHeader>()) / mem::size_of::<RBTNode>()) as u16;
        self.header.bid_root = NIL16;
        self.header.ask_root = NIL16;
        self.header.ask_minimum = NIL16;
        self.header.bid_maximum = NIL16;

        Ok(())
    }

    fn tree_header(&self) -> DexResult<&mut RBTHeader> {
        match unsafe { self.rbt_data_ptr.cast::<RBTHeader>().as_mut() } {
            Some(v) => Ok(v),
            None => Err(error!(DexError::InvalidRBTHeader)),
        }
    }

    fn get_free_rbt_node(&self, price: u64) -> DexResult<&mut RBTNode> {
        let header: &mut RBTHeader = self.tree_header()?;

        let node = if header.next_raw < header.total_raw {
            let offset = mem::size_of::<RBTHeader>()
                + (header.next_raw as usize) * mem::size_of::<RBTNode>();

            let raw_item = unsafe {
                self.rbt_data_ptr
                    .add(offset)
                    .cast::<RBTNode>()
                    .as_mut()
                    .unwrap()
            };
            raw_item.index = header.next_raw;

            header.next_raw += 1;
            raw_item
        } else {
            if header.top_free == NIL16 || header.top_free >= header.total_raw {
                return Err(error!(DexError::NoFreeRBTNode));
            }

            let offset = mem::size_of::<RBTHeader>()
                + (header.top_free as usize) * mem::size_of::<RBTNode>();
            let free_node = unsafe {
                self.rbt_data_ptr
                    .add(offset)
                    .cast::<RBTNode>()
                    .as_mut()
                    .unwrap()
            };
            header.top_free = free_node.left;
            free_node
        };

        node.zero();
        node.price = price;

        Ok(node)
    }

    fn release_rbt_node(&self, index: u16) -> DexResult {
        let header: &mut RBTHeader = self.tree_header()?;

        match self.from_index(index) {
            Some(n) => n.left = header.top_free,
            None => return Err(error!(DexError::InvalidIndex)),
        }

        header.top_free = index;

        Ok(())
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_index(&self, index: u16) -> Option<&mut RBTNode> {
        if index >= self.header.total_raw {
            return None;
        }

        let offset = mem::size_of::<RBTHeader>() + (index as usize) * mem::size_of::<RBTNode>();
        unsafe { self.rbt_data_ptr.add(offset).cast::<RBTNode>().as_mut() }
    }

    fn root_index(&self, side: OrderSide) -> u16 {
        match side {
            OrderSide::ASK => self.header.ask_root,
            OrderSide::BID => self.header.bid_root,
        }
    }

    fn root(&self, side: OrderSide) -> Option<&mut RBTNode> {
        self.from_index(self.root_index(side))
    }

    fn parent(&self, n: Option<&mut RBTNode>) -> Option<&mut RBTNode> {
        match n {
            Some(v) => self.from_index(v.parent),
            None => None,
        }
    }

    fn parent_index(&self, index: u16) -> u16 {
        let n = self.from_index(index);
        match n {
            Some(v) => v.parent,
            None => NIL16,
        }
    }

    fn grand_parent_index(&self, index: u16) -> u16 {
        let n = self.from_index(index);
        match self.parent(n) {
            Some(v) => v.parent,
            None => NIL16,
        }
    }

    fn uncle(&self, index: u16) -> u16 {
        let n = self.from_index(index);
        match n {
            Some(v) => {
                let gparent = self.from_index(self.grand_parent_index(index));
                match gparent {
                    Some(gp) => {
                        if v.parent == gp.left {
                            gp.right
                        } else {
                            gp.left
                        }
                    }
                    None => NIL16,
                }
            }
            None => NIL16,
        }
    }

    // Returns (index, parent, left, right, number_of_children)
    fn node_state(&self, n: Option<&mut RBTNode>) -> (u16, u16, u16, u16, u16, bool) {
        match n {
            Some(v) => {
                let mut children = 0;
                if v.left != NIL16 {
                    children += 1;
                }

                if v.right != NIL16 {
                    children += 1;
                }
                (v.index, v.parent, v.left, v.right, children, v.red > 0)
            }
            None => (NIL16, NIL16, NIL16, NIL16, 0, false),
        }
    }

    fn is_red(&self, index: u16) -> bool {
        let n = self.from_index(index);
        match n {
            Some(v) => (v.red > 0),
            None => false,
        }
    }

    fn is_black(&self, index: u16) -> bool {
        let n = self.from_index(index);
        match n {
            Some(v) => (v.red == 0),
            None => true,
        }
    }

    fn is_same(&self, n: &mut RBTNode, o: Option<&RBTNode>) -> bool {
        match o {
            Some(v) => v.index == n.index,
            None => false,
        }
    }

    fn is_left(&self, p_index: u16, c_index: u16) -> bool {
        let parent = self.from_index(p_index);
        let child = self.from_index(c_index);
        match parent {
            Some(p) => match child {
                Some(c) => p.left == c.index,
                None => false,
            },
            None => false,
        }
    }

    fn is_right(&self, p_index: u16, c_index: u16) -> bool {
        let parent = self.from_index(p_index);
        let child = self.from_index(c_index);

        match parent {
            Some(p) => match child {
                Some(c) => p.right == c.index,
                None => false,
            },
            None => false,
        }
    }

    fn left(&self, p: Option<&mut RBTNode>) -> Option<&mut RBTNode> {
        match p {
            Some(v) => self.from_index(v.left),
            None => None,
        }
    }

    fn left_index(&self, p: Option<&mut RBTNode>) -> u16 {
        match p {
            Some(v) => v.left,
            None => NIL16,
        }
    }

    fn right(&self, p: Option<&mut RBTNode>) -> Option<&mut RBTNode> {
        match p {
            Some(v) => self.from_index(v.right),
            None => None,
        }
    }

    fn right_index(&self, p: Option<&mut RBTNode>) -> u16 {
        match p {
            Some(v) => v.right,
            None => NIL16,
        }
    }

    fn set_left(&self, parent: Option<&mut RBTNode>, left: Option<&mut RBTNode>) {
        if let Some(p) = parent {
            match left {
                Some(v) => {
                    p.set_left_child(v.index);
                    v.set_parent(p.index);
                }
                None => p.set_left_child(NIL16),
            }
        }
    }

    fn set_right(&self, parent: Option<&mut RBTNode>, right: Option<&mut RBTNode>) {
        if let Some(p) = parent {
            match right {
                Some(v) => {
                    p.set_right_child(v.index);
                    v.set_parent(p.index);
                }
                None => p.set_right_child(NIL16),
            }
        }
    }

    fn set_parent(&self, n: &mut RBTNode, parent: Option<&RBTNode>) {
        match parent {
            Some(v) => n.set_parent(v.index),
            None => n.set_parent(NIL16),
        }
    }

    fn set_parent_index(&self, n: Option<&mut RBTNode>, parent_index: u16) {
        if let Some(v) = n {
            v.set_parent(parent_index)
        }
    }

    fn set_root(&self, root: Option<&mut RBTNode>, side: OrderSide) {
        let hdr: &mut RBTHeader =
            unsafe { self.rbt_data_ptr.cast::<RBTHeader>().as_mut().unwrap() };

        match root {
            Some(v) => match side {
                OrderSide::ASK => {
                    hdr.ask_root = v.index;
                    v.parent = NIL16;
                }
                OrderSide::BID => {
                    hdr.bid_root = v.index;
                    v.parent = NIL16;
                }
            },
            None => match side {
                OrderSide::ASK => {
                    hdr.ask_root = NIL16;
                }
                OrderSide::BID => {
                    hdr.bid_root = NIL16;
                }
            },
        }
    }

    fn set_root_index(&self, index: u16, side: OrderSide) {
        let header: &mut RBTHeader =
            unsafe { self.rbt_data_ptr.cast::<RBTHeader>().as_mut().unwrap() };

        match side {
            OrderSide::ASK => {
                header.ask_root = index;
            }
            OrderSide::BID => {
                header.bid_root = index;
            }
        }
    }

    fn set_red(&self, index: u16) {
        if let Some(v) = self.from_index(index) {
            v.set_color(1)
        }
    }

    fn set_black(&self, index: u16) {
        if let Some(v) = self.from_index(index) {
            v.set_color(0)
        }
    }

    fn rbt_next(&self, index: u16) -> u16 {
        // If we have a right-hand child, go down and then left as far as we can.
        //      6
        //     / \
        //    2  n(8)   --> From here
        //       / \
        //      7   10
        //         /
        //        9     --> Until find this one
        let n = self.from_index(index);
        let right = self.right_index(n);

        if right != NIL16 {
            let mut next: u16 = right;
            loop {
                let left = self.left_index(self.from_index(next));
                if left == NIL16 {
                    return next;
                }
                next = left;
            }
        }

        // No right child.
        //           9      --> Until find this one
        //          / \
        //         6  10
        //        / \
        //       2   7
        //            \
        //            n(8)  --> From here
        let mut current = index;
        loop {
            let parent = self.parent_index(current); // If no parent, there is no next node. Return none.
            if self.is_right(parent, current) {
                current = parent;
            } else {
                return parent;
            }
        }
    }

    fn rbt_prev(&self, index: u16) -> u16 {
        // If we have a left-hand child, go down and then right as far as we can.
        //     n(8)         --> From here
        //     / \
        //    4   10
        //   / \
        //  3   6
        //     / \
        //    5   7         --> Until find this one(7)
        let n = self.from_index(index);
        let left = self.left_index(n);
        if left != NIL16 {
            let mut prev: u16 = left;
            loop {
                let right = self.right_index(self.from_index(prev));
                if right == NIL16 {
                    return prev;
                }
                prev = right;
            }
        }

        // No left child.
        //           6          --> Until find this one
        //          / \
        //         5  10
        //            / \
        //           8   11
        //          /
        //         n(7)         --> From here
        let mut current = index;
        loop {
            let parent = self.parent_index(current); // If no parent, there is no next node. Return none.
            if self.is_left(parent, current) {
                current = parent;
            } else {
                return parent;
            }
        }
    }

    fn rotate_left(&self, index: u16, side: OrderSide) -> Option<&mut RBTNode> {
        //      pn                              pn
        //     /                               /
        //    n                               b
        //   /  \            ---->           / \
        //  nl   b                          n  br
        //     /   \                       /  \
        //    bl   br                     nl  bl
        let n = match self.from_index(index) {
            Some(v) => v,
            None => return None,
        };
        let pn = self.parent(Some(n));
        let b = self.right(Some(n))?; // If b doesn't exist, do nothing

        let n_is_left = self.is_left(self.parent_index(index), index);

        self.set_right(Some(n), self.left(Some(b)));
        self.set_left(Some(b), Some(n));

        match pn {
            Some(p) => {
                if n_is_left {
                    self.set_left(Some(p), Some(b));
                } else {
                    self.set_right(Some(p), Some(b));
                }
            }
            None => {
                self.set_parent(b, None);
                self.set_root(Some(b), side);
            }
        }

        Some(b)
    }

    fn rotate_right(&self, index: u16, side: OrderSide) -> Option<&mut RBTNode> {
        //            pn                               pn
        //           /                                /
        //          n                                b
        //         /  \            ---->            /  \
        //        b   nr                           bl   n
        //       / \                                   / \
        //      bl  br                                br  nr

        let n = match self.from_index(index) {
            Some(v) => v,
            None => return None,
        };

        let pn = self.parent(Some(n));
        let b = self.left(Some(n))?; // If b doesn't exist, do nothing

        self.set_parent(n, Some(b));
        self.set_left(Some(n), self.right(Some(b)));

        self.set_right(Some(b), Some(n));

        match pn {
            Some(v) => {
                if self.is_same(n, self.left(Some(v)).as_deref()) {
                    self.set_left(Some(v), Some(b));
                } else {
                    self.set_right(Some(v), Some(b));
                }
                self.set_parent(b, Some(v));
            }
            None => {
                self.set_root(Some(b), side);
                self.set_parent(b, None);
            }
        }

        Some(b)
    }

    #[cfg(test)]
    fn rbt_find(&self, price: u64, side: OrderSide) -> Option<&mut RBTNode> {
        let mut it = self.root(side);

        loop {
            match it {
                Some(v) => {
                    if v.price == price {
                        return Some(v);
                    }

                    if v.price > price {
                        it = self.left(Some(v));
                    } else {
                        it = self.right(Some(v));
                    }
                }
                None => return None,
            }
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn rbt_dump(&self, index: u16, prompt: &str) {
        let node = self.from_index(index);
        match node {
            Some(n) => println!(
                "{}: {}, left: {}, right: {}, parent: {}",
                prompt, n.price, n.left, n.right, n.parent
            ),
            None => println!("none"),
        }
    }

    fn rbt_insert(&self, price: u64, side: OrderSide) -> DexResult<&mut RBTNode> {
        let mut link_to: u16 = NIL16;
        let mut it = self.root(side);

        while let Some(v) = it {
            link_to = v.index;
            if v.price == price {
                return Ok(v);
            }

            if v.price > price {
                it = self.left(Some(v));
            } else {
                it = self.right(Some(v));
            }
        }

        let new = self.get_free_rbt_node(price)?;
        if link_to == NIL16 {
            self.set_root(Some(new), side);
        } else {
            let parent = self.from_index(link_to).unwrap();
            if parent.price > price {
                self.set_left(Some(parent), Some(new));
            } else {
                self.set_right(Some(parent), Some(new));
            }
        }

        self.insert_cases(new, side);

        match side {
            OrderSide::BID => self.update_bid_maximum_when_add(new.price, new.index),
            OrderSide::ASK => self.update_ask_minimum_when_add(new.price, new.index),
        }?;

        Ok(new)
    }

    fn insert_cases(&self, node: &mut RBTNode, side: OrderSide) {
        let mut current = node.index;

        loop {
            let mut parent = self.parent_index(current);
            if self.is_black(parent) {
                break;
            }
            let gparent = self.grand_parent_index(current);

            // Parent is left child of grand parent
            if self.is_left(gparent, parent) {
                // Case 1: uncle is red
                let uncle = self.uncle(current);
                if uncle != NIL16 && self.is_red(uncle) {
                    self.set_black(uncle);
                    self.set_black(parent);
                    self.set_red(gparent);

                    current = gparent;
                    continue;
                }

                // Case 2: uncle is black and current is right child of parent
                if self.is_right(parent, current) {
                    self.rotate_left(parent, side);
                    swap(&mut parent, &mut current);
                }

                // Case 3: uncle is black and current is left child of parent
                self.set_black(parent);
                self.set_red(gparent);
                self.rotate_right(gparent, side);
            } else {
                // Case 1: uncle is red and left child of grand parent
                let uncle = self.uncle(current);
                if uncle != NIL16 && self.is_red(uncle) {
                    self.set_black(uncle);
                    self.set_black(parent);
                    self.set_red(gparent);

                    current = gparent;
                    continue;
                }

                // Case 2: uncle is black and current is left child of parent
                if self.is_left(parent, current) {
                    self.rotate_right(parent, side);
                    swap(&mut parent, &mut current);
                }

                // Case 3: uncle is black and current is right child of parent
                self.set_black(parent);
                self.set_red(gparent);
                self.rotate_left(gparent, side);
            }
        }
        self.set_black(self.root_index(side));
    }

    fn replace(&self, old: u16, new: u16, side: OrderSide) {
        if let Some(ov) = self.from_index(old) {
            let ov_parent = self.from_index(ov.parent);

            match self.from_index(new) {
                Some(nv) => {
                    if self.is_left(ov.parent, ov.index) {
                        self.set_left(ov_parent, Some(nv));
                    } else if self.is_right(ov.parent, ov.index) {
                        self.set_right(ov_parent, Some(nv));
                    }

                    if ov.left != nv.index {
                        nv.left = ov.left;
                        self.set_parent_index(self.from_index(nv.left), nv.index);
                    }

                    if ov.right != nv.index {
                        nv.right = ov.right;
                        self.set_parent_index(self.from_index(nv.right), nv.index);
                    }

                    if old == self.root_index(side) {
                        self.set_parent_index(Some(nv), NIL16);
                    }
                }
                None => {
                    if self.is_left(ov.parent, ov.index) {
                        self.set_left(ov_parent, None);
                    } else if self.is_right(ov.parent, ov.index) {
                        self.set_right(ov_parent, None);
                    }
                }
            }
        }

        if old == self.root_index(side) {
            self.set_root_index(new, side);
        }

        self.release_rbt_node(old).unwrap();
    }

    // Splice out indexed node. Its parent is supposed to exist. Link the parent's corresponding child to node's right child(denoted as x node)
    // Return which side the node's right child is linked to the new parent
    fn splice_out(&self, index: u16) -> (u16, bool) {
        let parent = self.parent_index(index);
        let right = self.right(self.from_index(index));

        let result = if self.is_left(parent, index) {
            self.set_left(self.from_index(parent), right);
            (parent, true) //  On left side of parent
        } else {
            self.set_right(self.from_index(parent), right);
            (parent, false) //  On right side of parent
        };

        self.set_parent_index(self.from_index(index), parent);
        result
    }

    fn rbt_remove(&self, node: Option<&mut RBTNode>, side: OrderSide) -> DexResult {
        self.rbt_remove_internal(node, side)?;

        if let Some(r) = self.root(side) {
            r.parent = NIL16;
        }

        Ok(())
    }

    fn rbt_remove_internal(&self, node: Option<&mut RBTNode>, side: OrderSide) -> DexResult {
        let x_on_left: bool;
        let (current, parent, left, right, children, is_red) = self.node_state(node);

        match side {
            OrderSide::BID => self.update_bid_maximum_when_remove(current),
            OrderSide::ASK => self.update_ask_minimum_when_remove(current),
        }?;

        match children {
            0 => {
                // Step 1
                x_on_left = self.is_left(parent, current);
                self.replace(current, NIL16, side);
                if is_red || current == self.root_index(side) {
                    // Deleted node is red, replacement is NIL32, done
                    // -- OR --
                    // Deleted node is black, replacement is NIL32 and current is the root, done
                    return Ok(());
                } else {
                    // Deleted node is black , replacement is NIL32 and current is not the root, jump to cases
                    self.remove_cases(parent, x_on_left, side);
                }
            }
            1 => {
                // Replacement(same as x) is the child
                let replacement = if left == NIL16 { right } else { left };
                self.replace(current, replacement, side);
                x_on_left = self.is_left(parent, replacement);

                if is_red {
                    // Deleted node is red(which means it has parent) and the replacement must be black, color the replacement red and jump to cases
                    self.set_red(replacement);
                    self.remove_cases(parent, x_on_left, side);
                } else if self.is_red(replacement) {
                    // Deleted node is black and replacement child is red, color the replacement black, done.
                    self.set_black(replacement);
                } else {
                    // Deleted node is black and replacement child is black

                    // If the replacement is not the root, jump to cases., otherwise done.
                    if self.root_index(side) != replacement {
                        self.remove_cases(parent, x_on_left, side);
                    }

                    // If the replacement is the root, done.
                }
            }
            2 => {
                // Find the replacement
                let successor = self.rbt_next(current);
                let (mut x_parent, x_on_left) = self.splice_out(successor);
                let successor_is_red = self.is_red(successor);
                self.replace(current, successor, side);
                if x_parent == current {
                    x_parent = successor;
                }

                if is_red {
                    if successor_is_red {
                        // Deleted node is red, replacement is red, done
                        return Ok(());
                    } else {
                        // Deleted node is red, replacement is black, set the replacement red and jump to cases
                        self.set_red(successor);
                        self.remove_cases(x_parent, x_on_left, side);
                    }
                } else if successor_is_red {
                    // Deleted node is black, replacement is red, done
                    self.set_black(successor);
                } else {
                    // Deleted node is black, replacement is black and x can't be the root jump to cases
                    self.remove_cases(x_parent, x_on_left, side);
                }
            }
            _ => {}
        }

        Ok(())
    }

    // Always return as (x, sibling)
    fn x_and_sibling(&self, p_index: u16, x_on_left: bool) -> (u16, u16) {
        let parent = self.from_index(p_index);
        match parent {
            Some(p) => {
                if x_on_left {
                    (p.left, p.right)
                } else {
                    (p.right, p.left)
                }
            }
            None => (NIL16, NIL16),
        }
    }

    // Return children state as: (left_child, left_child_color, right_child, right_child_color)
    fn children_info(&self, parent: u16) -> (u16, bool, u16, bool) {
        match self.from_index(parent) {
            Some(p) => {
                let left_child = self.left(Some(p));
                let right_child = self.right(Some(p));

                let (left, left_red) = match left_child {
                    Some(c) => (c.index, c.red > 0),
                    None => (NIL16, false),
                };

                let (right, right_red) = match right_child {
                    Some(c) => (c.index, c.red > 0),
                    None => (NIL16, false),
                };

                (left, left_red, right, right_red)
            }
            None => (NIL16, false, NIL16, false),
        }
    }

    fn remove_cases(&self, mut parent: u16, mut x_on_left: bool, side: OrderSide) {
        loop {
            let (x, sibling) = self.x_and_sibling(parent, x_on_left);

            if self.is_red(x) {
                // Case 0
                self.set_black(x);
                break;
            } else if self.is_red(sibling) {
                // Case 1: x is black and sibling is red
                self.set_black(sibling);
                self.set_red(parent);

                if x_on_left {
                    self.rotate_left(parent, side);
                } else {
                    self.rotate_right(parent, side);
                }

                continue;
            } else {
                let (sibling_left, sibling_left_red, sibling_right, sibling_right_red) =
                    self.children_info(sibling);

                // Case 2: x is black and sibling is black, both of sibling's children are black
                if !sibling_left_red && !sibling_right_red {
                    self.set_red(sibling);
                    if self.is_red(parent) {
                        self.set_black(parent);
                        break;
                    } else if self.root_index(side) == parent {
                        break;
                    } else {
                        let x = parent;
                        parent = self.parent_index(x);
                        x_on_left = self.is_left(parent, x);
                        continue;
                    }
                }

                // Case 3: x is black and sibling is black, and
                //    If x is on left, w's left is red and w's right is black;
                //    Or if x is on right, w's right is red and w's left is black;
                if (x_on_left && sibling_left_red && !sibling_right_red)
                    || (!x_on_left && !sibling_left_red && sibling_right_red)
                {
                    if x_on_left {
                        self.set_black(sibling_left);
                    } else {
                        self.set_black(sibling_right);
                    }

                    self.set_red(sibling);

                    if x_on_left {
                        self.rotate_right(sibling, side);
                    } else {
                        self.rotate_left(sibling, side);
                    }

                    // Proceed to case 4
                }

                {
                    // Case 4: x is black and sibling is black, and
                    //    If x is on left, w's right is red;
                    //    Or if x is on right, w's left is red;
                    let (_, sibling) = self.x_and_sibling(parent, x_on_left);

                    let (sibling_left, sibling_left_red, sibling_right, sibling_right_red) =
                        self.children_info(sibling);

                    if (x_on_left && sibling_right_red) || (!x_on_left && sibling_left_red) {
                        if self.is_red(parent) {
                            self.set_red(sibling);
                        } else {
                            self.set_black(sibling);
                        }

                        self.set_black(parent);

                        if x_on_left {
                            self.set_black(sibling_right);
                        } else {
                            self.set_black(sibling_left);
                        }

                        if x_on_left {
                            self.rotate_left(parent, side);
                        } else {
                            self.rotate_right(parent, side);
                        }

                        break;
                    }
                }
            }
        }
    }

    fn bid_maximum(&self) -> Option<&mut RBTNode> {
        self.from_index(self.header.bid_maximum)
    }

    fn ask_minimum(&self) -> Option<&mut RBTNode> {
        self.from_index(self.header.ask_minimum)
    }

    fn update_bid_maximum_when_add(&self, price: u64, index: u16) -> DexResult {
        let header = self.tree_header()?;
        match self.from_index(self.header.bid_maximum) {
            Some(v) => {
                if price > v.price {
                    header.bid_maximum = index;
                }
            }
            None => header.bid_maximum = index,
        }
        Ok(())
    }

    fn update_ask_minimum_when_add(&self, price: u64, index: u16) -> DexResult {
        let header = self.tree_header()?;
        match self.from_index(header.ask_minimum) {
            Some(v) => {
                if price < v.price {
                    header.ask_minimum = index;
                }
            }
            None => header.ask_minimum = index,
        }
        Ok(())
    }

    fn update_bid_maximum_when_remove(&self, index: u16) -> DexResult {
        let header = self.tree_header()?;
        if index == header.bid_maximum {
            header.bid_maximum = self.rbt_prev(index);
        }

        Ok(())
    }

    fn update_ask_minimum_when_remove(&self, index: u16) -> DexResult {
        let header = self.tree_header()?;
        if index == header.ask_minimum {
            header.ask_minimum = self.rbt_next(index);
        }

        Ok(())
    }

    fn attach_order<T>(
        &self,
        node: &mut RBTNode,
        order: &mut PagedListSlot<T>,
        order_pool: &'a PagedList<T>,
    ) -> DexResult
    where
        PagedListSlot<T>: LinkedOrder<T>,
    {
        let (head, tail) =
            order.attach(order_pool, node.index, node.order_head, node.order_tail)?;

        node.order_head = head;
        node.order_tail = tail;
        node.size += order.size();
        node.order_count += 1;

        Ok(())
    }

    fn detach_order<T>(
        &self,
        side: OrderSide,
        node: &mut RBTNode,
        order: &mut PagedListSlot<T>,
        order_pool: &'a PagedList<T>,
    ) -> DexResult<u64>
    where
        PagedListSlot<T>: LinkedOrder<T>,
    {
        if node.size < order.size() {
            return Err(error!(DexError::ExceedOrderSize));
        }

        let detached_size = order.size();
        let (head, tail) = order.detach(order_pool, node.order_head, node.order_tail)?;
        order_pool
            .release_slot(order.index())
            .map_err(|_| DexError::PageLinkedListError)?;

        node.order_head = head;
        node.order_tail = tail;
        node.size = node.size.safe_sub(detached_size)?;
        node.order_count -= 1;

        if node.order_count == 0 {
            self.rbt_remove(Some(node), side)?;
        }

        Ok(detached_size)
    }

    // Returns (OrderSlot, price)
    pub fn get_next_match_order<T>(
        &self,
        price: u64,
        side: OrderSide,
        order_type: OrderType,
        order_pool: &'a PagedList<T>,
    ) -> Option<(&'a mut PagedListSlot<T>, u64)>
    where
        PagedListSlot<T>: LinkedOrder<T>,
    {
        let node = match side {
            OrderSide::BID => self.ask_minimum(),
            OrderSide::ASK => self.bid_maximum(),
        };

        let adjusted_limit_price = match order_type {
            OrderType::LIMIT => price,
            OrderType::MARKET => match side {
                OrderSide::BID => std::u64::MAX,
                OrderSide::ASK => 0u64,
            },
        };

        match node {
            Some(n) => {
                if match side {
                    OrderSide::BID => n.price <= adjusted_limit_price,
                    OrderSide::ASK => n.price >= adjusted_limit_price,
                } {
                    match order_pool.from_index(n.order_head) {
                        Ok(order) => Some((order, n.price)),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            None => None,
        }
    }

    // Return filled size
    pub fn fill_order<T>(
        &self,
        size: u64,
        side: OrderSide,
        order: &mut PagedListSlot<T>,
        order_pool: &'a PagedList<T>,
    ) -> DexResult<u64>
    where
        PagedListSlot<T>: LinkedOrder<T>,
    {
        match self.from_index(order.price_node()) {
            Some(n) => {
                let filled_size = if order.size() <= size {
                    self.detach_order(side.opposite(), n, order, order_pool)?
                } else {
                    n.size = n.size.safe_sub(size)?;
                    order.fill(size)?;

                    size
                };

                Ok(filled_size)
            }
            None => Err(error!(DexError::NoMatchOrder)),
        }
    }

    pub fn link_order<T>(
        &self,
        side: OrderSide,
        order: &mut PagedListSlot<T>,
        order_pool: &'a PagedList<T>,
    ) -> DexResult<u16>
    where
        PagedListSlot<T>: LinkedOrder<T>,
    {
        if order.size() == 0 {
            return Err(error!(DexError::ZeroSizeOrder));
        }

        let node = self.rbt_insert(order.price(), side)?;
        self.attach_order(node, order, order_pool)?;

        Ok(node.index)
    }

    pub fn unlink_order<T>(
        &self,
        side: OrderSide,
        order: &mut PagedListSlot<T>,
        order_pool: &'a PagedList<T>,
    ) -> DexResult
    where
        PagedListSlot<T>: LinkedOrder<T>,
    {
        match self.from_index(order.price_node()) {
            Some(node) => self.detach_order(side, node, order, order_pool)?,
            None => return Err(error!(DexError::InvalidRBTNode)),
        };

        Ok(())
    }

    pub fn get_best_price(&self, side: OrderSide) -> Option<u64> {
        let node = match side {
            OrderSide::BID => self.bid_maximum(),
            OrderSide::ASK => self.ask_minimum(),
        };
        match node {
            Some(n) => Some(n.price),
            None => None,
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::collections::MountMode;
    use crate::utils::unit_test::*;
    use bumpalo::Bump;
    use colored::*;
    use rand::prelude::*;

    #[test]
    fn test_order_book_mount() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        assert!(order_book.rbt_data_size == 2048);
    }

    #[test]
    fn test_order_book_initialize() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();
        order_book.initialize().assert_err();

        let unwrapped_order_tree = OrderBook::mount(&price_account, true).assert_unwrap();

        let tree_header = unwrapped_order_tree.tree_header().unwrap();
        assert_eq!(tree_header.magic, RBT_MAGIC);
        assert_eq!(tree_header.next_raw, 0);
        assert_eq!(tree_header.top_free, NIL16);
        assert_eq!(
            tree_header.total_raw,
            ((price_account.data_len() - mem::size_of::<RBTHeader>()) / mem::size_of::<RBTNode>())
                as u16
        );
        assert_eq!(tree_header.bid_root, NIL16);
        assert_eq!(tree_header.ask_root, NIL16);
        assert_eq!(tree_header.bid_maximum, NIL16);
        assert_eq!(tree_header.ask_minimum, NIL16);
    }

    #[test]
    fn test_order_book_rbt_node_management() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let max_rbt_node =
            (price_account.data_len() - mem::size_of::<RBTHeader>()) / mem::size_of::<RBTNode>();

        let mut rbt_node_vec: Vec<&mut RBTNode> = Vec::new();
        let mut rbt_node_count = max_rbt_node;

        loop {
            let node = order_book.get_free_rbt_node(10).assert_unwrap();
            rbt_node_vec.push(node);

            rbt_node_count -= 1;
            if rbt_node_count == 0 {
                break;
            }
        }

        // Can not get more.
        order_book.get_free_rbt_node(10).assert_err();

        loop {
            let node = rbt_node_vec.pop();
            assert!(node.is_some());

            // Can release every node.
            order_book.release_rbt_node(node.unwrap().index).assert_ok();

            if rbt_node_vec.is_empty() {
                break;
            }
        }

        assert_eq!(rbt_node_vec.len(), 0);

        loop {
            let item = order_book.get_free_rbt_node(10);

            match item {
                Ok(v) => rbt_node_vec.push(v),
                Err(_) => break,
            }
        }

        assert_eq!(rbt_node_vec.len(), max_rbt_node);
    }

    // Printable binary tree node, only for trees whose height less than 6
    // It is preferable that value is less than 100 for printing nicely
    #[derive(Eq, PartialEq, Copy, Clone, Debug)]
    struct PrintableNode {
        red: bool,
        index: u16,
        value: u64,
        left: bool,
        right: bool,
    }

    impl PrintableNode {
        fn new(
            value: u64,
            red: bool,
            left: bool,
            right: bool,
            index: u16,
        ) -> Option<Box<PrintableNode>> {
            Some(Box::new(PrintableNode {
                value,
                red,
                left,
                right,
                index,
            }))
        }
    }

    fn print_binary_tree(tree_vec: &Vec<Vec<Option<Box<PrintableNode>>>>) {
        // Tree height must less than 6
        assert!(tree_vec.len() < 6);

        let edge_level = vec![0usize, 1, 2, 5, 11];
        let offset = vec![0, 2, 5, 11, 23];
        let space = vec![(3, 1), (5, 5), (11, 11), (23, 23), (43, 43)];
        let space_c = " ";
        for (pos, level) in tree_vec.iter().enumerate() {
            let height = tree_vec.len() - pos - 1;
            // Print offset
            if offset[height] > 0 {
                print!("{:width$}", " ", width = offset[height]);
            }

            // Print nodes
            for (index, node) in level.iter().enumerate() {
                let occupy = match node {
                    Some(v) => {
                        if v.red {
                            print!("{}", v.as_ref().value.to_string().red());
                        } else {
                            print!("{}", v.as_ref().value);
                        }

                        v.as_ref().value.to_string().len() - 1
                    }
                    None => {
                        print!("{}", space_c);
                        0
                    }
                };

                if index % 2 == 0 {
                    print!("{:width$}", space_c, width = space[height].0 - occupy);
                } else if space[height].1 - occupy > 0 {
                    print!("{:width$}", space_c, width = space[height].1 - occupy);
                }
            }
            println!();

            if height == 0 {
                continue;
            }

            // Print edges
            for edge in 1..=edge_level[height] {
                print!("{:width$}", " ", width = offset[height] - edge);
                for (_, node) in level.iter().enumerate() {
                    let (left_edge, right_edge) = match node {
                        Some(v) => (
                            if v.as_ref().left { "/" } else { space_c },
                            if v.as_ref().right { "\\" } else { space_c },
                        ),
                        None => (space_c, space_c),
                    };

                    print!("{}", left_edge);
                    print!("{:width$}", space_c, width = 2 * edge - 1);
                    print!("{}", right_edge);

                    print!("{:width$}", space_c, width = (space[height].0 - edge * 2));
                }
                println!();
            }
        }
    }

    // cargo test test_order_book_print_binary_tree -- --nocapture
    #[test]
    fn test_order_book_print_binary_tree() {
        let mut tree_vec: Vec<Vec<Option<Box<PrintableNode>>>> = Vec::new();

        let mut root_vec: Vec<Option<Box<PrintableNode>>> = Vec::new();
        root_vec.push(PrintableNode::new(2, false, true, true, NIL16));
        tree_vec.push(root_vec);

        let mut level_1_vec: Vec<Option<Box<PrintableNode>>> = Vec::new();
        level_1_vec.push(PrintableNode::new(3, false, true, true, NIL16));
        level_1_vec.push(PrintableNode::new(4, false, true, true, NIL16));
        tree_vec.push(level_1_vec);

        let mut level_2_vec: Vec<Option<Box<PrintableNode>>> = Vec::new();
        level_2_vec.push(PrintableNode::new(61, false, false, false, NIL16));
        level_2_vec.push(PrintableNode::new(29, true, true, true, NIL16));
        level_2_vec.push(PrintableNode::new(38, true, false, true, NIL16));
        level_2_vec.push(PrintableNode::new(57, true, true, true, NIL16));
        tree_vec.push(level_2_vec);

        let mut level_3_vec: Vec<Option<Box<PrintableNode>>> = Vec::new();
        level_3_vec.push(None);
        level_3_vec.push(None);
        level_3_vec.push(PrintableNode::new(18, false, false, false, NIL16));
        level_3_vec.push(PrintableNode::new(78, false, false, false, NIL16));
        level_3_vec.push(None);
        level_3_vec.push(PrintableNode::new(20, false, false, false, NIL16));
        level_3_vec.push(PrintableNode::new(13, false, false, false, NIL16));
        level_3_vec.push(PrintableNode::new(91, false, false, false, NIL16));
        tree_vec.push(level_3_vec);

        print_binary_tree(&tree_vec);
    }

    fn print_rb_tree(rbt: &OrderBook, side: OrderSide) {
        let mut tree_vec: Vec<Vec<Option<Box<PrintableNode>>>> = Vec::new();

        let mut root_vec: Vec<Option<Box<PrintableNode>>> = Vec::new();
        let root = rbt.root(side);
        match root {
            Some(v) => {
                root_vec.push(PrintableNode::new(
                    v.price,
                    v.red == 1,
                    v.left != NIL16,
                    v.right != NIL16,
                    v.index,
                ));
            }
            None => {
                println!("Empty tree!");
                return;
            }
        };
        tree_vec.push(root_vec);

        loop {
            let mut valid_node = 0;
            let mut next_level_vec: Vec<Option<Box<PrintableNode>>> = Vec::new();
            let prev_level_vec = tree_vec.last().unwrap();
            for (_, node) in prev_level_vec.iter().enumerate() {
                let rbt_parent_node = match node {
                    Some(pn) => rbt.from_index(pn.as_ref().index),
                    _ => None,
                };

                match rbt_parent_node {
                    Some(n) => {
                        let left_child = rbt.left(Some(n));
                        let right_child = rbt.right(Some(n));

                        let printable_left = match left_child {
                            Some(c) => {
                                valid_node += 1;
                                PrintableNode::new(
                                    c.price,
                                    c.red == 1,
                                    c.left != NIL16,
                                    c.right != NIL16,
                                    c.index,
                                )
                            }
                            _ => None,
                        };
                        next_level_vec.push(printable_left);

                        let printable_right = match right_child {
                            Some(c) => {
                                valid_node += 1;
                                PrintableNode::new(
                                    c.price,
                                    c.red == 1,
                                    c.left != NIL16,
                                    c.right != NIL16,
                                    c.index,
                                )
                            }
                            _ => None,
                        };
                        next_level_vec.push(printable_right);
                    }
                    _ => {
                        next_level_vec.push(None);
                        next_level_vec.push(None);
                    }
                };
            }

            if valid_node > 0 {
                tree_vec.push(next_level_vec);
            } else {
                break;
            }
        }

        print_binary_tree(&tree_vec);
    }

    #[test]
    fn test_order_book_print_rb_tree() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(19, OrderSide::BID).assert_ok();
        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        order_book.rbt_insert(29, OrderSide::BID).assert_ok();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        order_book.rbt_insert(24, OrderSide::BID).assert_ok();

        print_rb_tree(&order_book, OrderSide::BID);

        order_book.rbt_insert(10, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(23, OrderSide::BID).assert_ok();
        order_book.rbt_insert(20, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(2, OrderSide::BID).assert_ok();
        order_book.rbt_insert(9, OrderSide::BID).assert_ok();

        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_insert_case() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        /* Supposed to be:
                             19
                            / \
                           /   \
                          /     \
                         /       \
                        /         \
                       /           \
                      /             \
                     /               \
                    /                 \
                   /                   \
                  /                     \
                 12                      25
                / \                     / \
               /   \                   /   \
              /     \                 /     \
             /       \               /       \
            /         \             /         \
           R9         R16          R21        R50
          / \         / \         / \         / \
         /   \       /   \       /   \       /   \
        8     10    15    18    20    23    27    60
               \   /                 / \   / \
               R11R13               R22R24R26 R40
        */
        order_book.rbt_insert(19, OrderSide::BID).assert_ok();
        order_book.rbt_insert(18, OrderSide::BID).assert_ok();
        order_book.rbt_insert(20, OrderSide::BID).assert_ok();
        order_book.rbt_insert(25, OrderSide::BID).assert_ok();
        order_book.rbt_insert(26, OrderSide::BID).assert_ok();
        order_book.rbt_insert(21, OrderSide::BID).assert_ok();
        order_book.rbt_insert(12, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(9, OrderSide::BID).assert_ok();
        order_book.rbt_insert(16, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(23, OrderSide::BID).assert_ok();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        order_book.rbt_insert(10, OrderSide::BID).assert_ok();
        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(24, OrderSide::BID).assert_ok();
        order_book.rbt_insert(50, OrderSide::BID).assert_ok();
        order_book.rbt_insert(60, OrderSide::BID).assert_ok();
        order_book.rbt_insert(27, OrderSide::BID).assert_ok();
        order_book.rbt_insert(40, OrderSide::BID).assert_ok();

        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_00() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(6, OrderSide::BID).assert_ok();
        let n_10 = order_book.rbt_insert(10, OrderSide::BID).assert_unwrap();

        /* Supposed to be:
          10
         /
        R6
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book
            .rbt_remove(Some(n_10), OrderSide::BID)
            .assert_ok();

        /* Supposed to be:
           6
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_01() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        order_book.rbt_insert(17, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(1, OrderSide::BID).assert_ok();
        let n_6 = order_book.rbt_insert(6, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        order_book.rbt_insert(25, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(27, OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8         R17
          / \         / \
         /   \       /   \
        1     11    15    25
         \               / \
          R6            R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book.rbt_remove(Some(n_6), OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8          R17
          / \         / \
         /   \       /   \
        1     11    15    25
                         / \
                        R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_02() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        order_book.rbt_insert(17, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        let n_1 = order_book.rbt_insert(1, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(6, OrderSide::BID).assert_ok();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        order_book.rbt_insert(25, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(27, OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8         R17
          / \         / \
         /   \       /   \
        1     11    15    25
         \               / \
          R6            R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book.rbt_remove(Some(n_1), OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8          R17
          / \         / \
         /   \       /   \
        6     11    15    25
                         / \
                        R22 R27
               */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_03() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        let n_17 = order_book.rbt_insert(17, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(1, OrderSide::BID).assert_ok();
        order_book.rbt_insert(6, OrderSide::BID).assert_ok();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        order_book.rbt_insert(25, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(27, OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8         R17
          / \         / \
         /   \       /   \
        1     11    15    25
         \               / \
          R6            R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book
            .rbt_remove(Some(n_17), OrderSide::BID)
            .assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8          R22
          / \         / \
         /   \       /   \
        1     11    15    25
         \                 \
          R6                R27
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_04() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        order_book.rbt_insert(17, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(1, OrderSide::BID).assert_ok();
        order_book.rbt_insert(6, OrderSide::BID).assert_ok();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        let n_25 = order_book.rbt_insert(25, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(27, OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8         R17
          / \         / \
         /   \       /   \
        1     11    15    25
         \               / \
          R6            R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book
            .rbt_remove(Some(n_25), OrderSide::BID)
            .assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8         R17
          / \         / \
         /   \       /   \
        1     11    15    27
         \               /
          R6            R22
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_05() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(7, OrderSide::BID).assert_ok();
        order_book.rbt_insert(3, OrderSide::BID).assert_ok();
        let n_18 = order_book.rbt_insert(18, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(10, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(26, OrderSide::BID).assert_ok();

        /* Supposed to be:
              7
             / \
            /   \
           /     \
          /       \
         /         \
        3          R18
                   / \
                  /   \
                 10    22
                / \     \
               R8  R11   R26
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book
            .rbt_remove(Some(n_18), OrderSide::BID)
            .assert_ok();

        /* Supposed to be:
              7
             / \
            /   \
           /     \
          /       \
         /         \
        3           R22
                   / \
                  /   \
                 10    26
                / \
               R8  R11
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_06() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(5, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        let n_1 = order_book.rbt_insert(1, OrderSide::BID).assert_unwrap();
        let n_4 = order_book.rbt_insert(4, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(7, OrderSide::BID).assert_ok();
        order_book.rbt_insert(9, OrderSide::BID).assert_ok();
        let n_2 = order_book.rbt_insert(2, OrderSide::BID).assert_unwrap();

        order_book.set_black(n_1.index);
        order_book.set_black(n_4.index);
        order_book.set_red(n_2.index);

        /* Supposed to be:
             5
            / \
           /   \
          R2    8
         / \   / \
        1   4 R7  R9
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book
            .rbt_remove(order_book.from_index(n_2.index), OrderSide::BID)
            .assert_ok();

        /* Supposed to be:
             5
            / \
           /   \
          4     8
         /     / \
        R1    R7  R9
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_07() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        let n_13 = order_book.rbt_insert(13, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(17, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(1, OrderSide::BID).assert_ok();
        order_book.rbt_insert(6, OrderSide::BID).assert_ok();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        order_book.rbt_insert(25, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(27, OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8         R17
          / \         / \
         /   \       /   \
        1     11    15    25
         \               / \
          R6            R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book
            .rbt_remove(Some(n_13), OrderSide::BID)
            .assert_ok();

        /* Supposed to be:
                 15
                / \
               /   \
              /     \
             /       \
            /         \
           R8          R25
          / \         / \
         /   \       /   \
        1     11    17    27
         \           \
          R6          R22
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_08() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        order_book.rbt_insert(17, OrderSide::BID).assert_ok();
        let n_8 = order_book.rbt_insert(8, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(1, OrderSide::BID).assert_ok();
        order_book.rbt_insert(6, OrderSide::BID).assert_ok();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        order_book.rbt_insert(25, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(27, OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8         R17
          / \         / \
         /   \       /   \
        1     11    15    25
         \               / \
          R6            R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book.rbt_remove(Some(n_8), OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R6          R17
          / \         / \
         /   \       /   \
        1     11    15    25
                         / \
                        R22 R27
               */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_09() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(7, OrderSide::BID).assert_ok();
        let n_3 = order_book.rbt_insert(3, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(18, OrderSide::BID).assert_ok();
        order_book.rbt_insert(10, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(26, OrderSide::BID).assert_ok();

        /* Supposed to be:
              7
             / \
            /   \
           /     \
          /       \
         /         \
        3           R18
                   / \
                  /   \
                 10    22
                / \     \
               R8  R11   R26
            */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book.rbt_remove(Some(n_3), OrderSide::BID).assert_ok();

        /* Supposed to be:
                 18
                / \
               /   \
              /     \
             /       \
            /         \
           R10          22
          / \           \
         /   \           \
        7     11          R26
         \
          R8
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_10() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        order_book.rbt_insert(17, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();
        let n_11 = order_book.rbt_insert(11, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(1, OrderSide::BID).assert_ok();
        order_book.rbt_insert(6, OrderSide::BID).assert_ok();
        order_book.rbt_insert(15, OrderSide::BID).assert_ok();
        order_book.rbt_insert(25, OrderSide::BID).assert_ok();
        order_book.rbt_insert(22, OrderSide::BID).assert_ok();
        order_book.rbt_insert(27, OrderSide::BID).assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R8         R17
          / \         / \
         /   \       /   \
        1     11    15    25
         \               / \
          R6            R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);

        order_book
            .rbt_remove(Some(n_11), OrderSide::BID)
            .assert_ok();

        /* Supposed to be:
                 13
                / \
               /   \
              /     \
             /       \
            /         \
           R6          R17
          / \         / \
         /   \       /   \
        1     8     15    25
                         / \
                        R22 R27
        */
        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_rb_remove_11() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        let n_8 = order_book.rbt_insert(8, OrderSide::BID).assert_unwrap();
        order_book.rbt_insert(11, OrderSide::BID).assert_ok();
        order_book.rbt_insert(1, OrderSide::BID).assert_ok();
        order_book.rbt_insert(6, OrderSide::BID).assert_ok();

        /* Supposed to be:
           8
          / \
         /   \
        1     11
         \
          R6
        */

        print_rb_tree(&order_book, OrderSide::BID);

        order_book.rbt_remove(Some(n_8), OrderSide::BID).assert_ok();

        /* Supposed to be:
          6
         / \
        1   11
        */

        print_rb_tree(&order_book, OrderSide::BID);
    }

    #[test]
    fn test_order_book_large_number_of_nodes() {
        let total_nodes = 32767;
        let bump = Bump::new();
        let price_account = gen_account(
            mem::size_of::<RBTNode>() * total_nodes + mem::size_of::<RBTHeader>(),
            &bump,
        );

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let mut node_vec: Vec<&mut RBTNode> = Vec::new();

        // Add  prices
        let mut price = total_nodes;
        loop {
            let node = order_book
                .rbt_insert(price as u64, OrderSide::BID)
                .assert_unwrap();

            node_vec.push(node);

            price -= 1;
            if price == 0 {
                break;
            }
        }

        assert!(node_vec.len() == total_nodes);

        // Remove all the prices
        let mut count = 0;
        loop {
            let node = node_vec.pop();
            if node.is_none() {
                break;
            }
            order_book.rbt_remove(node, OrderSide::BID).assert_ok();

            count += 1;
        }

        assert!(node_vec.len() == 0);
        assert!(count == total_nodes);

        assert!(order_book.root_index(OrderSide::BID) == NIL16)
    }

    #[test]
    fn test_order_bid_maximum() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).unwrap();
        order_book.initialize().assert_ok();

        assert!(order_book.bid_maximum().is_none());
        assert!(order_book.ask_minimum().is_none());

        let n_13 = order_book.rbt_insert(13, OrderSide::BID).unwrap();
        assert_eq!(order_book.bid_maximum().unwrap().index, n_13.index);

        let n_17 = order_book.rbt_insert(17, OrderSide::BID).unwrap();
        assert_eq!(order_book.bid_maximum().unwrap().index, n_17.index);

        order_book.rbt_insert(8, OrderSide::BID).unwrap();
        assert_eq!(order_book.bid_maximum().unwrap().index, n_17.index);

        let n_25 = order_book.rbt_insert(25, OrderSide::BID).unwrap();
        assert_eq!(order_book.bid_maximum().unwrap().index, n_25.index);
    }

    #[test]
    fn test_order_bid_maximum_update() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let n_13 = order_book.rbt_insert(13, OrderSide::BID).assert_unwrap();
        let n_17 = order_book.rbt_insert(17, OrderSide::BID).assert_unwrap();
        let n_8 = order_book.rbt_insert(8, OrderSide::BID).assert_unwrap();
        let n_25 = order_book.rbt_insert(25, OrderSide::BID).assert_unwrap();

        assert_eq!(order_book.bid_maximum().unwrap().index, n_25.index);

        order_book
            .rbt_remove(Some(n_25), OrderSide::BID)
            .assert_ok();
        assert_eq!(order_book.bid_maximum().unwrap().index, n_17.index);

        order_book.rbt_remove(Some(n_8), OrderSide::BID).assert_ok();
        assert_eq!(order_book.bid_maximum().unwrap().index, n_17.index);

        order_book
            .rbt_remove(Some(n_13), OrderSide::BID)
            .assert_ok();
        assert_eq!(order_book.bid_maximum().unwrap().index, n_17.index);

        order_book
            .rbt_remove(Some(n_17), OrderSide::BID)
            .assert_ok();
        assert!(order_book.bid_maximum().is_none());
    }

    fn list_book(rbt: &OrderBook, side: OrderSide) -> Vec<u64> {
        let mut book: Vec<u64> = Vec::new();
        let mut node = match side {
            OrderSide::BID => rbt.bid_maximum(),
            OrderSide::ASK => rbt.ask_minimum(),
        };
        loop {
            let index: u16;
            match node {
                Some(node) => {
                    book.push(node.price);
                    index = node.index;
                }
                None => break,
            }

            let prev_index = match side {
                OrderSide::BID => rbt.rbt_prev(index),
                OrderSide::ASK => rbt.rbt_next(index),
            };
            node = rbt.from_index(prev_index);
        }

        return book;
    }

    #[test]
    fn test_order_book_test_list_book() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        order_book.rbt_insert(25, OrderSide::BID).assert_ok();
        order_book.rbt_insert(29, OrderSide::BID).assert_ok();
        order_book.rbt_insert(39, OrderSide::BID).assert_ok();
        order_book.rbt_insert(13, OrderSide::BID).assert_ok();
        order_book.rbt_insert(17, OrderSide::BID).assert_ok();
        order_book.rbt_insert(49, OrderSide::BID).assert_ok();
        order_book.rbt_insert(59, OrderSide::BID).assert_ok();
        order_book.rbt_insert(8, OrderSide::BID).assert_ok();

        let bid_book = list_book(&order_book, OrderSide::BID);

        let mut sort_book = vec![25, 29, 39, 13, 17, 49, 59, 8];
        sort_book.sort_by(|a, b| b.cmp(a));

        assert_eq!(sort_book.len(), bid_book.len());
        for i in 0..bid_book.len() {
            assert_eq!(sort_book[i], bid_book[i]);
        }
    }

    #[test]
    fn test_order_book_remove_root() {
        let bump = Bump::new();
        let price_account = gen_account(512 * 1024, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let mut sort_book: Vec<u64> = Vec::new();

        for _ in 0..2048 {
            let price = rand::thread_rng().gen_range(0, 100000);

            order_book.rbt_insert(price, OrderSide::BID).assert_ok();

            sort_book.push(price);
        }

        let bid_book = list_book(&order_book, OrderSide::BID);

        // Remove dup items
        let set: HashSet<_> = sort_book.drain(..).collect();
        sort_book.extend(set.into_iter());

        // Sort from big to small
        sort_book.sort_by(|a, b| b.cmp(a));

        for i in 0..bid_book.len() {
            assert_eq!(bid_book[i], sort_book[i]);
        }

        loop {
            // Remove the root
            let root = order_book.root(OrderSide::BID).unwrap();
            order_book
                .rbt_remove(Some(root), OrderSide::BID)
                .assert_ok();

            let index = sort_book.iter().position(|&r| r == root.price).unwrap();
            sort_book.remove(index);

            let bid_book = list_book(&order_book, OrderSide::BID);

            assert_eq!(bid_book.len(), sort_book.len());

            for i in 0..bid_book.len() {
                assert_eq!(bid_book[i], sort_book[i]);
            }

            if sort_book.len() == 0 {
                break;
            }
        }
    }

    #[test]
    fn test_order_book_remove_bid_maximum() {
        let bump = Bump::new();
        let price_account = gen_account(512 * 1024, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let mut sort_book: Vec<u64> = Vec::new();

        for _ in 0..2048 {
            let price = rand::thread_rng().gen_range(0, 100000);

            order_book.rbt_insert(price, OrderSide::BID).assert_ok();

            sort_book.push(price);
        }

        let bid_book = list_book(&order_book, OrderSide::BID);

        // Remove dup items
        let set: HashSet<_> = sort_book.drain(..).collect();
        sort_book.extend(set.into_iter());

        // Sort from big to small
        sort_book.sort_by(|a, b| b.cmp(a));

        for i in 0..bid_book.len() {
            assert_eq!(bid_book[i], sort_book[i]);
        }

        let book_len = bid_book.len();

        let mut repeat = 2048;

        loop {
            // Remove the bid maximum
            let index = 0;
            let price = sort_book[index];

            sort_book.remove(index);

            let node = order_book.rbt_find(price, OrderSide::BID);

            order_book.rbt_remove(node, OrderSide::BID).assert_ok();

            let bid_book = list_book(&order_book, OrderSide::BID);

            assert_eq!(bid_book.len(), sort_book.len());

            for i in 0..bid_book.len() {
                assert_eq!(bid_book[i], sort_book[i]);
            }

            // Insert a price larger than bid maximum
            let price = sort_book[0] + 1;

            order_book.rbt_insert(price, OrderSide::BID).assert_ok();
            sort_book.push(price);

            let bid_book = list_book(&order_book, OrderSide::BID);

            // Remove dup items
            let set: HashSet<_> = sort_book.drain(..).collect();
            sort_book.extend(set.into_iter());

            sort_book.sort_by(|a, b| b.cmp(a));

            for i in 0..bid_book.len() {
                assert_eq!(bid_book[i], sort_book[i]);
            }

            assert_eq!(bid_book.len(), book_len);

            repeat -= 1;
            if repeat == 0 || sort_book.len() == 0 {
                break;
            }
        }
    }

    #[test]
    fn test_order_book_insert_and_remove_ask_book() {
        let bump = Bump::new();
        let price_account = gen_account(512 * 1024, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let mut sort_book: Vec<u64> = Vec::new();

        for _ in 0..2048 {
            let price = rand::thread_rng().gen_range(0, 100000);

            order_book.rbt_insert(price, OrderSide::ASK).assert_ok();

            sort_book.push(price);
        }

        let ask_book = list_book(&order_book, OrderSide::ASK);

        // Remove dup items
        let set: HashSet<_> = sort_book.drain(..).collect();
        sort_book.extend(set.into_iter());

        // Sort from small to big
        sort_book.sort_by(|a, b| a.cmp(b));

        // println!("Ask  book {:?}", ask_book);
        // println!("Sort book {:?}", sort_book);

        for i in 0..ask_book.len() {
            assert_eq!(ask_book[i], sort_book[i]);
        }

        loop {
            // Remove the ask minimum
            let price = sort_book[0];

            sort_book.remove(0);

            let node = order_book.rbt_find(price, OrderSide::ASK);

            order_book.rbt_remove(node, OrderSide::ASK).assert_ok();

            let ask_book = list_book(&order_book, OrderSide::ASK);

            assert_eq!(ask_book.len(), sort_book.len());

            for i in 0..ask_book.len() {
                assert_eq!(ask_book[i], sort_book[i]);
            }

            if sort_book.len() == 0 {
                break;
            }
        }

        let tree_header = order_book.tree_header().assert_unwrap();
        assert_eq!(tree_header.ask_minimum, NIL16);
        assert_eq!(tree_header.ask_root, NIL16);
    }

    #[test]
    fn test_order_book_list_bid_book() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let n_13 = order_book.rbt_insert(13, OrderSide::BID).assert_unwrap();
        let n_17 = order_book.rbt_insert(17, OrderSide::BID).assert_unwrap();
        let n_8 = order_book.rbt_insert(8, OrderSide::BID).assert_unwrap();
        let n_25 = order_book.rbt_insert(25, OrderSide::BID).assert_unwrap();

        let mut bid_book = list_book(&order_book, OrderSide::BID);

        assert_eq!(bid_book.len(), 4);
        assert_eq!(bid_book[0], 25);
        assert_eq!(bid_book[1], 17);
        assert_eq!(bid_book[2], 13);
        assert_eq!(bid_book[3], 8);

        order_book
            .rbt_remove(Some(n_25), OrderSide::BID)
            .assert_ok();

        bid_book = list_book(&order_book, OrderSide::BID);
        assert_eq!(bid_book.len(), 3);
        assert_eq!(bid_book[0], 17);
        assert_eq!(bid_book[1], 13);
        assert_eq!(bid_book[2], 8);

        order_book
            .rbt_remove(Some(n_13), OrderSide::BID)
            .assert_ok();

        bid_book = list_book(&order_book, OrderSide::BID);
        assert_eq!(bid_book.len(), 2);
        assert_eq!(bid_book[0], 17);
        assert_eq!(bid_book[1], 8);

        order_book
            .rbt_remove(Some(n_17), OrderSide::BID)
            .assert_ok();

        bid_book = list_book(&order_book, OrderSide::BID);
        assert_eq!(bid_book.len(), 1);
        assert_eq!(bid_book[0], 8);

        order_book.rbt_remove(Some(n_8), OrderSide::BID).assert_ok();

        bid_book = list_book(&order_book, OrderSide::BID);
        assert_eq!(bid_book.len(), 0);
    }

    #[test]
    fn test_order_ask_minimum() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        assert!(order_book.bid_maximum().is_none());
        assert!(order_book.ask_minimum().is_none());

        let n_13 = order_book.rbt_insert(13, OrderSide::ASK).unwrap();
        assert_eq!(order_book.ask_minimum().unwrap().index, n_13.index);

        order_book.rbt_insert(17, OrderSide::ASK).unwrap();
        assert_eq!(order_book.ask_minimum().unwrap().index, n_13.index);

        let n_8 = order_book.rbt_insert(8, OrderSide::ASK).unwrap();
        assert_eq!(order_book.ask_minimum().unwrap().index, n_8.index);

        order_book.rbt_insert(25, OrderSide::ASK).unwrap();
        assert_eq!(order_book.ask_minimum().unwrap().index, n_8.index);
    }

    #[test]
    fn test_order_book_list_ask_book() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let n_13 = order_book.rbt_insert(13, OrderSide::ASK).unwrap();
        let n_17 = order_book.rbt_insert(17, OrderSide::ASK).unwrap();
        let n_8 = order_book.rbt_insert(8, OrderSide::ASK).unwrap();
        let n_25 = order_book.rbt_insert(25, OrderSide::ASK).unwrap();

        let mut ask_book = list_book(&order_book, OrderSide::ASK);

        assert_eq!(ask_book.len(), 4);
        assert_eq!(ask_book[0], 8);
        assert_eq!(ask_book[1], 13);
        assert_eq!(ask_book[2], 17);
        assert_eq!(ask_book[3], 25);

        order_book
            .rbt_remove(Some(n_25), OrderSide::ASK)
            .assert_ok();

        ask_book = list_book(&order_book, OrderSide::ASK);
        assert_eq!(ask_book.len(), 3);
        assert_eq!(ask_book[0], 8);
        assert_eq!(ask_book[1], 13);
        assert_eq!(ask_book[2], 17);

        order_book
            .rbt_remove(Some(n_13), OrderSide::ASK)
            .assert_ok();

        ask_book = list_book(&order_book, OrderSide::ASK);
        assert_eq!(ask_book.len(), 2);
        assert_eq!(ask_book[0], 8);
        assert_eq!(ask_book[1], 17);

        order_book
            .rbt_remove(Some(n_17), OrderSide::ASK)
            .assert_ok();

        ask_book = list_book(&order_book, OrderSide::ASK);
        assert_eq!(ask_book.len(), 1);
        assert_eq!(ask_book[0], 8);

        order_book.rbt_remove(Some(n_8), OrderSide::ASK).assert_ok();

        ask_book = list_book(&order_book, OrderSide::ASK);
        assert_eq!(ask_book.len(), 0);
    }

    struct OrderSlot {
        pub price: u64,
        pub size: u64,
        pub next: u32,
        pub prev: u32,
        pub price_node: u16,
        padding: u16,
    }

    impl LinkedOrder<OrderSlot> for PagedListSlot<OrderSlot> {
        fn index(&self) -> u32 {
            self.index()
        }

        fn size(&self) -> u64 {
            self.data.size
        }

        fn price(&self) -> u64 {
            self.data.price
        }

        fn price_node(&self) -> u16 {
            self.data.price_node
        }

        fn fill(&mut self, size: u64) -> DexResult {
            self.data.size = self.data.size.safe_sub(size)?;
            Ok(())
        }

        fn detach(
            &mut self,
            pool: &PagedList<OrderSlot>,
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
            pool: &PagedList<OrderSlot>,
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

    #[allow(clippy::mut_from_ref)]
    fn new_and_link_order<'a>(
        order_book: &'a OrderBook,
        order_pool: &'a PagedList<OrderSlot>,
        side: OrderSide,
        price: u64,
        size: u64,
    ) -> &'a mut PagedListSlot<OrderSlot> {
        let order = order_pool.new_slot().assert_unwrap();
        order.data.price = price;
        order.data.size = size;
        order_book.link_order(side, order, order_pool).assert_ok();

        order
    }

    fn assert_price_node(
        order_book: &OrderBook,
        side: OrderSide,
        price: u64,
        size: u64,
        head: u32,
        tail: u32,
    ) {
        let price_node = order_book.rbt_find(price, side);
        match price_node {
            Some(p) => {
                assert!(p.order_head == head && p.order_tail == tail && p.size == size)
            }
            None => unreachable!(),
        }
    }

    #[test]
    fn test_order_book_link_unlink_order() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);
        let order_account = gen_account(4096, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let order_pool =
            PagedList::<OrderSlot>::mount(&order_account, &[], 0x1, MountMode::Initialize)
                .assert_unwrap();

        for side in [OrderSide::BID, OrderSide::ASK] {
            // Link orders
            let o_900 = new_and_link_order(&order_book, &order_pool, side, 20000, 900);
            let o_500 = new_and_link_order(&order_book, &order_pool, side, 20000, 500);
            assert_price_node(&order_book, side, 20000, 1400, o_900.index(), o_500.index());

            let o_200 = new_and_link_order(&order_book, &order_pool, side, 20000, 200);
            assert_price_node(&order_book, side, 20000, 1600, o_900.index(), o_200.index());

            // Unlink o_200
            order_book
                .unlink_order(side, o_200, &order_pool)
                .assert_ok();
            assert_price_node(&order_book, side, 20000, 1400, o_900.index(), o_500.index());

            // Unlink o_500
            order_book
                .unlink_order(side, o_500, &order_pool)
                .assert_ok();
            assert_price_node(&order_book, side, 20000, 900, o_900.index(), o_900.index());

            // Unlink o_900
            order_book
                .unlink_order(side, o_900, &order_pool)
                .assert_ok();
            let price_20000 = order_book.rbt_find(20000, side);
            assert!(price_20000.is_none());
        }
    }

    #[test]
    fn test_order_book_fill_one_order() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);
        let order_account = gen_account(4096, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let order_pool =
            PagedList::<OrderSlot>::mount(&order_account, &[], 0x1, MountMode::Initialize)
                .assert_unwrap();

        // Link o_900
        let o_900 = new_and_link_order(&order_book, &order_pool, OrderSide::BID, 20000, 900);

        // Consume the first order of price 20000
        let bid_900 =
            order_book.get_next_match_order(20000, OrderSide::ASK, OrderType::LIMIT, &order_pool);
        assert!(bid_900.is_some());

        let bid_900_unwrapped = bid_900.unwrap().0;

        // Fill 100
        order_book
            .fill_order(100, OrderSide::ASK, bid_900_unwrapped, &order_pool)
            .assert_ok();

        assert_price_node(
            &order_book,
            OrderSide::BID,
            20000,
            800,
            o_900.index(),
            o_900.index(),
        );

        // Fill another 100
        order_book
            .fill_order(100, OrderSide::ASK, bid_900_unwrapped, &order_pool)
            .assert_ok();

        assert_price_node(
            &order_book,
            OrderSide::BID,
            20000,
            700,
            o_900.index(),
            o_900.index(),
        );

        // Fill more than 700
        order_book
            .fill_order(1000, OrderSide::ASK, bid_900_unwrapped, &order_pool)
            .assert_ok();

        let price_20000 = order_book.rbt_find(20000, OrderSide::BID);
        assert!(price_20000.is_none());
    }

    #[test]
    fn test_order_book_fill_multiple_orders() {
        let bump = Bump::new();
        let price_account = gen_account(2048, &bump);
        let order_account = gen_account(4096, &bump);

        let mut order_book = OrderBook::mount(&price_account, false).assert_unwrap();
        order_book.initialize().assert_ok();

        let order_pool =
            PagedList::<OrderSlot>::mount(&order_account, &[], 0x1, MountMode::Initialize)
                .assert_unwrap();

        // Link o_900
        let o_900 = new_and_link_order(&order_book, &order_pool, OrderSide::BID, 20000, 900);
        assert_price_node(
            &order_book,
            OrderSide::BID,
            20000,
            900,
            o_900.index(),
            o_900.index(),
        );

        // Link o_500
        let o_500 = new_and_link_order(&order_book, &order_pool, OrderSide::BID, 20000, 500);
        assert_price_node(
            &order_book,
            OrderSide::BID,
            20000,
            1400,
            o_900.index(),
            o_500.index(),
        );

        // Link o_200
        let o_200 = new_and_link_order(&order_book, &order_pool, OrderSide::BID, 20000, 200);
        assert_price_node(
            &order_book,
            OrderSide::BID,
            20000,
            1600,
            o_900.index(),
            o_200.index(),
        );

        // Fill the first order(0_900) of price 20000
        order_book
            .fill_order(900, OrderSide::ASK, o_900, &order_pool)
            .assert_ok();

        // Head is o_500 and tail is o_200 now
        assert_price_node(
            &order_book,
            OrderSide::BID,
            20000,
            700,
            o_500.index(),
            o_200.index(),
        );

        // Fill next order(o_500) of price 20000
        order_book
            .fill_order(500, OrderSide::ASK, o_500, &order_pool)
            .assert_ok();

        // Head and tail is o_200
        assert_price_node(
            &order_book,
            OrderSide::BID,
            20000,
            200,
            o_200.index(),
            o_200.index(),
        );

        // Fill last order(o_200) of price 20000
        order_book
            .fill_order(200, OrderSide::ASK, o_200, &order_pool)
            .assert_ok();

        // No price 20000
        let price_20000 = order_book.rbt_find(20000, OrderSide::BID);
        assert!(price_20000.is_none());
    }
}
