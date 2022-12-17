#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{add_fee, collateral_to_size, minus_add_fee, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_open_btc_long_with_btc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.feed_btc_price(20000.).await;
    user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.))
        .await;
    user.assert_fee(DexAsset::BTC, add_fee(10.)).await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    // Alice open long
    alice.mint_btc(0.1).await;

    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    let expected_open_fee = 0.002912621;
    let expected_collateral = 0.1 - expected_open_fee;
    let expected_size = expected_collateral * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_size,
            0.,
        )
        .await;

    // Check liquidity pool
    user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.) - expected_size)
        .await;
    user.assert_fee(DexAsset::BTC, add_fee(10.) + expected_open_fee)
        .await;
    user.assert_borrow(DexAsset::BTC, expected_size).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;
}

#[tokio::test]
async fn test_open_btc_short_with_usdc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(100000.).await;
    user.feed_btc_price(20000.).await;
    user.assert_liquidity(DexAsset::USDC, minus_add_fee(100000.))
        .await;
    user.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
    user.assert_borrow(DexAsset::USDC, 0.).await;
    user.assert_collateral(DexAsset::USDC, 0.).await;

    // Alice open short
    alice.mint_usdc(2000.).await;

    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;

    let expected_open_fee = 58.252427;
    let expected_collateral = 2000. - expected_open_fee;
    let expected_size = collateral_to_size(expected_collateral, 10., 20000., 9);
    let expect_borrow = expected_collateral * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            false,
            20000.,
            expected_size,
            expected_collateral,
            expect_borrow,
            0.,
        )
        .await;

    // Check liquidity pool
    user.assert_liquidity(DexAsset::USDC, minus_add_fee(100000.) - expect_borrow)
        .await;
    user.assert_fee(DexAsset::USDC, add_fee(100000.) + expected_open_fee)
        .await;
    user.assert_borrow(DexAsset::USDC, expect_borrow).await;
    user.assert_collateral(DexAsset::USDC, expected_collateral)
        .await;
}
