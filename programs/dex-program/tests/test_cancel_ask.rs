#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{add_fee, btc, collateral_to_size, minus_add_fee, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_cancel_ask_long() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.mock_btc_price(20000.).await;
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

    let position_size = alice.get_position_size(DexMarket::BTC, true).await;
    // Place ask order partially
    alice.assert_ask(DexMarket::BTC, true, 22000., 0.5).await;
    alice
        .assert_ask_order(DexMarket::BTC, true, 22000., btc(0.5))
        .await;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_size,
            0.5,
        )
        .await;

    // If placing an over-sized ask order, the actual size should be the minimum available size
    alice
        .assert_ask(DexMarket::BTC, true, 22000., u64::MAX as f64)
        .await;
    alice
        .assert_ask_order(DexMarket::BTC, true, 22000., position_size - btc(0.5))
        .await;
    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_size,
            expected_size,
        )
        .await;

    // Should not ask more
    alice
        .assert_ask_fail(DexMarket::BTC, true, 22000., 0.000001)
        .await;

    // Cancel order 0
    alice.cancel(0).await;
    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_size,
            expected_size - 0.5,
        )
        .await;

    // Cancel order 1
    alice.cancel(1).await;
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
}

#[tokio::test]
async fn test_cancel_ask_short() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(100000.).await;
    user.mock_btc_price(20000.).await;

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

    let position_size = alice.get_position_size(DexMarket::BTC, false).await;
    // Place ask order partially
    alice.assert_ask(DexMarket::BTC, false, 18000., 0.5).await;
    alice
        .assert_ask_order(DexMarket::BTC, false, 18000., btc(0.5))
        .await;

    alice
        .assert_ask(DexMarket::BTC, false, 17000., u64::MAX as f64)
        .await;
    alice
        .assert_ask_order(DexMarket::BTC, false, 17000., position_size - btc(0.5))
        .await;

    alice
        .assert_position(
            DexMarket::BTC,
            false,
            20000.,
            expected_size,
            expected_collateral,
            expect_borrow,
            expected_size,
        )
        .await;

    // Cancel order 0
    alice.cancel(0).await;
    alice
        .assert_position(
            DexMarket::BTC,
            false,
            20000.,
            expected_size,
            expected_collateral,
            expect_borrow,
            expected_size - 0.5,
        )
        .await;

    // Cancel order 1
    alice.cancel(1).await;
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
}
