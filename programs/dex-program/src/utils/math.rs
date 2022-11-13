use anchor_lang::prelude::*;

use crate::errors::{DexError, DexResult};

pub trait SafeMath<T> {
    fn safe_add(self, v: T) -> DexResult<T>;
    fn safe_sub(self, v: T) -> DexResult<T>;
    fn safe_mul(self, v: T) -> DexResult<u128>;
    fn safe_div(self, v: T) -> DexResult<u128>;
    fn safe_pow(self, v: u32) -> DexResult<u128>;
}

impl SafeMath<u64> for u64 {
    fn safe_add(self, v: u64) -> DexResult<u64> {
        match self.checked_add(v) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn safe_sub(self, v: u64) -> DexResult<u64> {
        match self.checked_sub(v) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn safe_mul(self, v: u64) -> DexResult<u128> {
        match (self as u128).checked_mul(v as u128) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn safe_div(self, v: u64) -> DexResult<u128> {
        match (self as u128).checked_div(v as u128) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn safe_pow(self, v: u32) -> DexResult<u128> {
        match (self as u128).checked_pow(v) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }
}

impl SafeMath<u128> for u128 {
    fn safe_add(self, v: u128) -> DexResult<u128> {
        match self.checked_add(v) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn safe_sub(self, v: u128) -> DexResult<u128> {
        match self.checked_sub(v) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn safe_mul(self, v: u128) -> DexResult<u128> {
        match (self as u128).checked_mul(v as u128) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn safe_div(self, v: u128) -> DexResult<u128> {
        match (self as u128).checked_div(v as u128) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn safe_pow(self, v: u32) -> DexResult<u128> {
        match (self as u128).checked_pow(v) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }
}

pub trait ISafeMath<T> {
    fn i_safe_mul(self, v: T) -> DexResult<i128>;
    fn i_safe_div(self, v: T) -> DexResult<i128>;
}

impl ISafeMath<i128> for i128 {
    fn i_safe_mul(self, v: i128) -> DexResult<i128> {
        match (self as i128).checked_mul(v as i128) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn i_safe_div(self, v: i128) -> DexResult<i128> {
        match (self as i128).checked_div(v as i128) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }
}

pub trait ISafeAddSub {
    fn i_safe_add(self, v: i64) -> DexResult<i64>;
    fn i_safe_sub(self, v: i64) -> DexResult<i64>;
}

impl ISafeAddSub for i64 {
    fn i_safe_add(self, v: i64) -> DexResult<i64> {
        match self.checked_add(v) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }

    fn i_safe_sub(self, v: i64) -> DexResult<i64> {
        match self.checked_sub(v) {
            Some(r) => Ok(r),
            None => Err(error!(DexError::SafeMathError)),
        }
    }
}

pub fn get_timestamp() -> Result<i64> {
    #[cfg(test)]
    {
        Ok(0)
    }

    #[cfg(not(test))]
    {
        Ok(Clock::get()?.unix_timestamp)
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::{errors::DexResult, utils::test::TestResult};

    fn test_safe_add() -> DexResult {
        let result = 1u64.safe_add(2u64)?;
        assert_eq!(result, 3u64);
        Ok(())
    }

    fn test_safe_add_overflow() -> DexResult {
        match u64::MAX.safe_add(2u64) {
            Ok(_) => Err(error!(DexError::SafeMathError)),
            Err(_) => Ok(()),
        }
    }

    fn test_safe_sub() -> DexResult {
        let result = 100u64.safe_sub(2u64)?;
        assert_eq!(result, 98u64);
        Ok(())
    }

    fn test_safe_sub_overflow() -> DexResult {
        match 10u64.safe_sub(200u64) {
            Ok(_) => Err(error!(DexError::SafeMathError)),
            Err(_) => Ok(()),
        }
    }

    fn test_safe_mul() -> DexResult {
        let result = 1000u64.safe_mul(2u64)?;
        assert_eq!(result, 2000u128);
        Ok(())
    }

    fn test_safe_mul_u64_max() -> DexResult {
        let result = u64::MAX.safe_mul(u64::MAX)?;
        print!(
            "u64::MAX * u64::MAX is:\n{}\nu128::MAX is:\n{}\n",
            result,
            u128::MAX
        );

        assert!(result < u128::MAX);

        Ok(())
    }

    fn test_safe_div() -> DexResult {
        let result = 1000u64.safe_div(2u64)?;
        assert_eq!(result, 500u128);
        Ok(())
    }

    fn test_safe_div_overflow() -> DexResult {
        match 10u64.safe_div(0u64) {
            Ok(_) => Err(error!(DexError::SafeMathError)),
            Err(_) => Ok(()),
        }
    }

    fn test_safe_pow() -> DexResult {
        let result = 10u64.safe_pow(6u32)?;
        assert_eq!(result, 1000000u128);
        Ok(())
    }

    fn test_i_safe_add() -> DexResult {
        let result = 1i64.i_safe_add(10i64)?;
        assert_eq!(result, 11);
        Ok(())
    }

    fn test_i_safe_add_negative() -> DexResult {
        let result = 1i64.i_safe_add(-11i64)?;
        assert_eq!(result, -10);
        Ok(())
    }

    fn test_i_safe_sub() -> DexResult {
        let result = 1i64.i_safe_sub(11i64)?;
        assert_eq!(result, -10);
        Ok(())
    }

    fn test_i_safe_sub_negative() -> DexResult {
        let result = 1i64.i_safe_sub(-11i64)?;
        assert_eq!(result, 12);
        Ok(())
    }

    #[test]
    fn test_safe_math() {
        test_safe_add().assert_ok();
        test_safe_add_overflow().assert_ok();

        test_safe_sub().assert_ok();
        test_safe_sub_overflow().assert_ok();

        test_safe_mul().assert_ok();
        test_safe_mul_u64_max().assert_ok();

        test_safe_div().assert_ok();
        test_safe_div_overflow().assert_ok();

        test_safe_pow().assert_ok();

        test_i_safe_add().assert_ok();
        test_i_safe_add_negative().assert_ok();

        test_i_safe_sub().assert_ok();
        test_i_safe_sub_negative().assert_ok();
    }
}
