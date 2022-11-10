use crate::{
    dex::{get_oracle_price, Dex},
    errors::{DexError, DexResult},
    utils::SafeMath,
};

use anchor_lang::{error, prelude::AccountInfo, require_eq};

pub fn get_asset_aum(dex: &Dex, remaining_accounts: &[AccountInfo]) -> DexResult<u64> {
    //get pool asset sum
    let mut asset_sum = 0;
    let mut asset_offset = 0;
    for asset_index in 0..dex.assets.len() {
        let asset_info = &dex.assets[asset_index];
        if !asset_info.valid {
            continue;
        }
        require_eq!(
            dex.assets[asset_index].oracle,
            *remaining_accounts[asset_offset].key,
            DexError::InvalidOracleAccount
        );

        let oracle_price =
            get_oracle_price(asset_info.oracle_source, &remaining_accounts[asset_offset])?;

        asset_sum += (asset_info
            .liquidity_amount
            .safe_add(asset_info.collateral_amount)?)
        .safe_mul(oracle_price.into())?
        .safe_div(10u128.pow(asset_info.decimals.into()))? as u64;

        asset_offset += 1;
    }

    //get pool pnl
    let mut pnl = 0;
    let mut market_offset = asset_offset;
    for market_index in 0..dex.markets.len() {
        let market_info = &dex.markets[market_index];
        if !market_info.valid {
            continue;
        }
        require_eq!(
            dex.markets[market_index].oracle,
            *remaining_accounts[market_offset].key,
            DexError::InvalidOracleAccount
        );

        let oracle_price = get_oracle_price(
            market_info.oracle_source,
            &remaining_accounts[market_offset],
        )?;

        if market_info.global_long.size > 0 {
            pnl += -(market_info.global_long.pnl(
                market_info.global_long.size,
                oracle_price,
                market_info.global_long.average_price,
                market_info.decimals,
            )?);
        }

        if market_info.global_short.size > 0 {
            pnl += -(market_info.global_short.pnl(
                market_info.global_short.size,
                oracle_price,
                market_info.global_short.average_price,
                market_info.decimals,
            )?);
        }

        market_offset += 1;
    }

    if pnl > 0 {
        asset_sum += pnl as u64;
    } else {
        asset_sum -= pnl as u64;
    }

    Ok(asset_sum)
}
