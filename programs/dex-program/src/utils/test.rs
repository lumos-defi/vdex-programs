#![cfg(test)]
use core::panic;
use std::fmt::Debug;

use anchor_lang::prelude::{AccountInfo, Clock, Pubkey};
use bumpalo::{collections::Vec as BumpVec, vec as bump_vec, Bump};
use rand::prelude::*;
use safe_transmute::to_bytes::{transmute_to_bytes, transmute_to_bytes_mut};

use super::LEVERAGE_DECIMALS;

pub const USDC_DECIMALS: u8 = 6;
pub const SOL_DECIMALS: u8 = 9;
pub const BTC_DECIMALS: u8 = 9;
pub const ETH_DECIMALS: u8 = 9;

pub trait TestResult<T, E> {
    fn assert_unwrap(self) -> T;
    fn assert_err(self);
    fn assert_ok(self);
}
impl<T, E> TestResult<T, E> for Result<T, E>
where
    E: Debug,
{
    fn assert_unwrap(self) -> T {
        if !self.is_ok() {
            panic!("error: {:?}", self.err().unwrap());
        }
        self.unwrap()
    }

    fn assert_err(self) {
        assert!(self.is_err());
    }

    fn assert_ok(self) {
        if !self.is_ok() {
            panic!("error: {:?}", self.err().unwrap());
        }
    }
}

pub trait TestCertainError<T, E> {
    fn assert_certain_err(self, error: E);
}
impl<T, E> TestCertainError<T, E> for Result<T, E>
where
    E: Debug + PartialEq,
{
    fn assert_certain_err(self, error: E) {
        if !self.is_err() {
            panic!("should return Err {:?}, but return Ok", error);
        }
        if *self.as_ref().err().unwrap() != error {
            panic!(
                "should return Err {:?}, but return {:?}",
                error,
                self.err().unwrap()
            );
        }
    }
}

pub fn gen_pubkey<'a, G: rand::Rng>(_rng: &mut G, bump: &'a Bump) -> &'a Pubkey {
    bump.alloc(Pubkey::new(transmute_to_bytes(&rand::random::<[u64; 4]>())))
}

pub fn rand_pubkey() -> Pubkey {
    Pubkey::new(transmute_to_bytes(&rand::random::<[u64; 4]>()))
}

#[allow(clippy::mut_from_ref)]
pub fn allocate_account_data(size: usize, bump: &Bump) -> &mut [u8] {
    let size_u64 = ((size + 8) & (!0x7 as usize)) >> 3;
    let data_vec: BumpVec<'_, u64> = bump_vec![in bump; 0u64; size_u64];
    &mut transmute_to_bytes_mut(data_vec.into_bump_slice_mut())[..size]
}

pub fn gen_account<'a>(size: usize, bump: &'a Bump) -> AccountInfo<'a> {
    let mut rng = StdRng::seed_from_u64(0);
    let program_id = gen_pubkey(&mut rng, bump);

    AccountInfo::new(
        gen_pubkey(&mut rng, bump),
        false,
        true,
        bump.alloc(1024 * 1024),
        allocate_account_data(size, bump),
        program_id,
        false,
        Clock::default().epoch,
    )
}

pub fn pnl(size: i64) -> i64 {
    (size * (10i64.pow(USDC_DECIMALS as u32) as i64)) as i64
}

pub fn usdc(size: f64) -> u64 {
    (size * (10u64.pow(USDC_DECIMALS as u32) as f64)) as u64
}

pub fn btc(size: f64) -> u64 {
    (size * (10u64.pow(BTC_DECIMALS as u32) as f64)) as u64
}

pub fn sol(size: f64) -> u64 {
    (size * (10u64.pow(SOL_DECIMALS as u32) as f64)) as u64
}

pub fn eth(size: f64) -> u64 {
    (size * (10u64.pow(ETH_DECIMALS as u32) as f64)) as u64
}

pub fn usdc_i(size: f64) -> i64 {
    (size * (10u64.pow(USDC_DECIMALS as u32) as f64)) as i64
}

pub fn btc_i(size: f64) -> i64 {
    (size * (10u64.pow(BTC_DECIMALS as u32) as f64)) as i64
}

pub fn eth_i(size: f64) -> i64 {
    (size * (10u64.pow(ETH_DECIMALS as u32) as f64)) as i64
}

pub fn sol_i(size: f64) -> i64 {
    (size * (10u64.pow(SOL_DECIMALS as u32) as f64)) as i64
}

pub fn leverage(l: u32) -> u32 {
    l * 10u32.pow(LEVERAGE_DECIMALS as u32)
}
