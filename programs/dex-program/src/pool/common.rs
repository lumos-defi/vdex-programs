use crate::{
    dex::{get_oracle_price, Dex, OracleInfo},
    errors::{DexError, DexResult},
};

use anchor_lang::{error, prelude::AccountInfo, require_eq, Key};

// pub fn get_asset_aum_in_usdc(dex: &Dex, remaining_accounts: &[AccountInfo]) -> DexResult<u64> {
//     let (asset_oracles, offset) = collect_asset_oracles(dex, remaining_accounts, 0)?;
//     let (market_oracles, _) = collect_market_oracles(dex, remaining_accounts, offset)?;

//     let aumInUsdc = 0;
//     let asset_oracle_offset = 0;
//     for asset in &dex.assets {
//         if asset.valid {
//             let oracle_price = get_oracle_price(
//                 asset.oracle_source,
//                 asset_oracles[asset_oracle_offset]?.oracle_account,
//             )?;
//         }
//     }

//     Ok(aumInUsdc)
// }
