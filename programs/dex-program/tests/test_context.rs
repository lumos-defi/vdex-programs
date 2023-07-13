#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::account::ReadableAccount;
use solana_program_test::tokio;

use context::DexTestContext;
use utils::{convert_to_big_number, DexAsset, DexMarket};

#[tokio::test]
async fn test_init_context() {
    let dtc = DexTestContext::new().await;

    if let [_alice, _bob, ..] = &dtc.user_context[..] {
        assert!(dtc.dex_info.borrow().assets[0].symbol == *b"USDC\0\0\0\0\0\0\0\0\0\0\0\0");
        assert!(dtc.dex_info.borrow().assets[1].symbol == *b"BTC\0\0\0\0\0\0\0\0\0\0\0\0\0");
        assert!(dtc.dex_info.borrow().assets[2].symbol == *b"ETH\0\0\0\0\0\0\0\0\0\0\0\0\0");
        assert!(dtc.dex_info.borrow().assets[3].symbol == *b"SOL\0\0\0\0\0\0\0\0\0\0\0\0\0");
    }

    let user = &dtc.user_context[0];

    user.assert_liquidity(DexAsset::BTC, 0.0).await;
    user.assert_liquidity(DexAsset::ETH, 0.0).await;
    user.assert_liquidity(DexAsset::USDC, 0.0).await;
    user.assert_liquidity(DexAsset::SOL, 0.0).await;

    user.assert_borrow(DexAsset::BTC, 0.0).await;
    user.assert_borrow(DexAsset::ETH, 0.0).await;
    user.assert_borrow(DexAsset::USDC, 0.0).await;

    user.assert_fee(DexAsset::BTC, 0.0).await;
    user.assert_fee(DexAsset::ETH, 0.0).await;
    user.assert_fee(DexAsset::USDC, 0.0).await;
    user.assert_fee(DexAsset::SOL, 0.0).await;

    user.assert_collateral(DexAsset::BTC, 0.0).await;
    user.assert_collateral(DexAsset::ETH, 0.0).await;
    user.assert_collateral(DexAsset::USDC, 0.0).await;

    user.assert_global_long(DexMarket::BTC, 0.0, 0.0).await;
    user.assert_global_short(DexMarket::BTC, 0.0, 0.0).await;

    user.assert_global_long(DexMarket::ETH, 0.0, 0.0).await;
    user.assert_global_short(DexMarket::ETH, 0.0, 0.0).await;
}

#[tokio::test]
async fn test_get_order_book_account() {
    let market = 0;
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    let order_book = alice
        .get_account(alice.dex_info.borrow().markets[market as usize].order_book)
        .await;

    assert!(order_book.lamports() > 0);
}

#[tokio::test]
async fn test_convert_to_big_number() {
    let decimals: u8 = 8;

    {
        let x: f64 = 1.0;
        let y: f64 = 0.01;

        let z = convert_to_big_number(x.into(), decimals) + convert_to_big_number(y, decimals);
        assert_eq!(z, convert_to_big_number(x + y, decimals));
    }

    {
        let x: f64 = 1.32;
        let y: f64 = 0.56;

        let z = convert_to_big_number(x.into(), decimals) + convert_to_big_number(y, decimals);
        assert_eq!(z, convert_to_big_number(x + y, decimals));
    }

    {
        let x: f64 = 100.0;
        let y: f64 = 10.0;

        let z = convert_to_big_number(x.into(), decimals) + convert_to_big_number(y, decimals);
        assert_eq!(z, convert_to_big_number(x + y, decimals));
    }

    {
        let x: f64 = 1.0;
        let y: f64 = 1.0;

        let z = convert_to_big_number(x.into(), decimals) + convert_to_big_number(y, decimals);
        assert_eq!(z, convert_to_big_number(x + y, decimals));
    }

    // TODO: lost precision
    // {
    //     let x: f64 = 1.001;
    //     let y: u64 = 1001000;

    //     assert_eq!(y, convert_to_big_number(x, 6));
    // }
}
