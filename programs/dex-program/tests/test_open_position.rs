#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{add_fee, minus_add_fee, minus_swap_fee, swap_fee, DexAsset, DexMarket};
use context::DexTestContext;

fn collateral_to_size(collateral: f64, leverage: f64, price: f64, base_decimals: u8) -> f64 {
    let adjust_decimals = 10f64.powi(base_decimals as i32 - 6);

    collateral * leverage * adjust_decimals / price
}

#[tokio::test]
async fn test_btc_open_long_with_btc() {
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
        .open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10)
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
async fn test_btc_open_long_with_eth() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.feed_btc_price(20000.).await;
    user.feed_eth_price(2000.).await;

    // Assert BTC pool
    user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.))
        .await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;
    assert!(false);
    // Assert ETH pool
    // user.assert_liquidity(DexAsset::ETH, minus_add_fee(1000.))
    //     .await;
    // user.assert_fee(DexAsset::ETH, add_fee(1000.)).await;
    // user.assert_borrow(DexAsset::ETH, 0.).await;
    // user.assert_collateral(DexAsset::ETH, 0.).await;

    // Alice open long
    let input_eth = 1.0;
    alice.mint_eth(input_eth).await;

    alice
        .open(DexAsset::ETH, DexMarket::BTC, true, input_eth, 10)
        .await;
    alice.assert_eth_balance(0.).await;

    let swap_fee = swap_fee(input_eth);
    assert_eq!(swap_fee, 0.001);

    let swapped_btc = (input_eth - swap_fee) * 2000.0 / 20000.;
    assert_eq!(swapped_btc, 0.0999);

    // let expected_open_fee = 0.002912621;
    // let expected_collateral = swapped_btc - expected_open_fee;
    // let expected_size = expected_collateral * 10.;

    // alice
    //     .assert_position(
    //         DexMarket::BTC,
    //         true,
    //         20000.,
    //         expected_size,
    //         expected_collateral,
    //         expected_size,
    //         0.,
    //     )
    //     .await;

    // Check liquidity pool
    // user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.) - expected_size)
    //     .await;
    // user.assert_fee(DexAsset::BTC, add_fee(10.) + expected_open_fee)
    //     .await;
    // user.assert_borrow(DexAsset::BTC, expected_size).await;
    // user.assert_collateral(DexAsset::BTC, expected_collateral)
    //     .await;
}

#[tokio::test]
async fn test_btc_open_short_with_usdc() {
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
    alice.mint_usdc(2000.).await;

    alice
        .open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10)
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
