#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{add_fee, minus_add_fee, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_open_btc_long_fail_minimum() {
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

    // Alice open long, minimum is 25 USD
    alice.mint_btc(0.00125).await;

    alice
        .assert_open_fail(DexAsset::BTC, DexMarket::BTC, true, 0.00125, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.00125).await;

    // Add more collateral
    alice.mint_btc(0.00005).await;
    alice.assert_btc_balance(0.0013).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.0013, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;
}

#[tokio::test]
async fn test_open_btc_short_fail_minimum() {
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

    // Alice open long
    alice.mint_usdc(25.).await;

    alice
        .assert_open_fail(DexAsset::USDC, DexMarket::BTC, false, 25., 10 * 1000)
        .await;
    alice.assert_usdc_balance(25.).await;

    alice.mint_usdc(2.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 27., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;
}
