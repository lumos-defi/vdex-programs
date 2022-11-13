use anchor_lang::prelude::*;
use num_enum::TryFromPrimitive;
use std::cell::Ref;
use std::cell::RefMut;
use std::convert::TryFrom;

use crate::errors::{DexError, DexResult};
use crate::utils::constant::{USDC_DECIMALS, USDC_POW_DECIMALS};
use crate::utils::SafeMath;

use super::MockOracle;

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum OracleSource {
    Mock = 0,
    Pyth = 1,
    StableCoin = 2,
}

pub fn from_raw_price(raw_price: u64, expo: u32) -> DexResult<u64> {
    Ok(raw_price
        .safe_mul(USDC_POW_DECIMALS)?
        .safe_div(10u64.pow(expo) as u128)? as u64)
}

#[cfg(feature = "client-support")]
pub fn get_oracle_price_from_data(oracle_source: u8, oracle_data: &[u8]) -> DexResult<u64> {
    let source =
        OracleSource::try_from(oracle_source).map_err(|_| DexError::InvalidOracleSource)?;

    match source {
        OracleSource::Pyth => {
            let pyth_oracle = pyth_client::load_price(&oracle_data[..])
                .map_err(|_| DexError::InvalidOracleSource)?;

            from_raw_price(
                pyth_oracle.agg.price as u64,
                i32::abs(pyth_oracle.expo) as u32,
            )
        }
        OracleSource::Mock => {
            let mock_oracle = unsafe { oracle_data.as_ptr().add(8).cast::<MockOracle>().as_ref() }
                .ok_or(DexError::InvalidOracleSource)?;

            from_raw_price(mock_oracle.price, mock_oracle.expo as u32)
        }
        OracleSource::StableCoin => Ok(USDC_POW_DECIMALS),
    }
}

pub fn get_oracle_price(oracle_source: u8, oracle_account: &AccountInfo) -> DexResult<u64> {
    let source =
        OracleSource::try_from(oracle_source).map_err(|_| DexError::InvalidOracleSource)?;

    let oracle_price = match source {
        OracleSource::Pyth => get_pyth_price(oracle_account)?,
        OracleSource::Mock => get_mock_price(oracle_account)?,
        OracleSource::StableCoin => USDC_POW_DECIMALS,
    };
    Ok(oracle_price)
}

//Pyth Oracle
fn get_pyth_price(oracle_account: &AccountInfo) -> DexResult<u64> {
    let oracle_account_data = &oracle_account.data.borrow();
    let pyth_oracle =
        pyth_client::load_price(oracle_account_data).map_err(|_| DexError::InvalidOracleSource)?;

    from_raw_price(
        pyth_oracle.agg.price as u64,
        i32::abs(pyth_oracle.expo) as u32,
    )
}

//Mock Oracle
fn get_mock_price(oracle_account: &AccountInfo) -> DexResult<u64> {
    let data_ptr = match oracle_account.try_borrow_data() {
        Ok(p) => Ref::map(p, |data| *data).as_ptr(),
        Err(_) => return Err(error!(DexError::FailedLoadOracle)),
    };

    let mock_oracle = unsafe { data_ptr.add(8).cast::<MockOracle>().as_ref() }
        .ok_or(DexError::InvalidOracleSource)?;
    from_raw_price(mock_oracle.price, mock_oracle.expo as u32)
}

pub fn set_mock_price(account: &AccountInfo, price: u64) -> DexResult {
    let data_ptr = match account.try_borrow_mut_data() {
        Ok(p) => RefMut::map(p, |data| *data).as_mut_ptr(),
        Err(_) => return Err(error!(DexError::FailedMountAccount)),
    };

    let oracle_price = unsafe { data_ptr.add(8).cast::<MockOracle>().as_mut() }
        .ok_or(DexError::InvalidOracleSource)?;

    oracle_price.price = price;
    oracle_price.expo = USDC_DECIMALS;

    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::utils::test::{gen_account, TestResult};
    use bumpalo::Bump;

    #[test]
    fn test_mock_oracle() {
        let bump = Bump::new();
        let account = gen_account(1024, &bump);

        let preset_price = 40000u64 * 10u64.pow(6);

        set_mock_price(&account, preset_price).assert_ok();
        let price = get_mock_price(&account).assert_unwrap();

        assert_eq!(price, preset_price);
    }
}
