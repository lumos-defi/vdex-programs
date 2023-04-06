#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{btc, collateral_to_size, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_cancel_bid_orders() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.mock_btc_price(20000.).await;
    user.mock_eth_price(2000.).await;
    user.mock_sol_price(20.).await;

    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.add_liquidity_with_usdc(100000.).await;

    // Alice bids long with BTC
    alice.mint_btc(0.2).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 19000., 0.1, 10 * 1000)
        .await;

    alice.assert_btc_balance(0.1).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 18000., 0.1, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    alice.mint_usdc(2000.).await;
    alice.assert_usdc_balance(2000.).await;
    alice
        .assert_bid(
            DexAsset::USDC,
            DexMarket::BTC,
            false,
            22000.,
            2000.,
            10 * 1000,
        )
        .await;
    alice.assert_usdc_balance(0.).await;

    alice.mint_sol(10.).await;
    alice
        .assert_bid(DexAsset::SOL, DexMarket::ETH, false, 2200., 10., 10 * 1000)
        .await;

    alice.cancel_call().await;
    alice.assert_btc_balance(0.2).await;
    alice.assert_usdc_balance(2000.).await;
    alice.assert_no_order().await;
}

#[tokio::test]
async fn test_cancel_bid_and_ask_orders() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_usdc(100000.).await;
    user.add_liquidity_with_eth(100.).await;
    user.mock_btc_price(20000.).await;
    user.mock_eth_price(2000.).await;

    // Alice open long
    alice.mint_btc(0.1).await;

    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    let expected_open_long_fee = 0.002912621;
    let expected_long_collateral = 0.1 - expected_open_long_fee;
    let expected_long_size = expected_long_collateral * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_long_size,
            expected_long_collateral,
            expected_long_size,
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
            expected_long_size,
            expected_long_collateral,
            expected_long_size,
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
            expected_long_size,
            expected_long_collateral,
            expected_long_size,
            expected_long_size,
        )
        .await;

    // Should not ask more
    alice
        .assert_ask_fail(DexMarket::BTC, true, 22000., 0.000001)
        .await;

    // Alice open short
    alice.mint_usdc(2000.).await;

    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;

    let expected_open_short_fee = 58.252427;
    let expected_short_collateral = 2000. - expected_open_short_fee;
    let expected_short_size = collateral_to_size(expected_short_collateral, 10., 20000., 9);
    let expected_short_borrow = expected_short_collateral * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            false,
            20000.,
            expected_short_size,
            expected_short_collateral,
            expected_short_borrow,
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
            expected_short_size,
            expected_short_collateral,
            expected_short_borrow,
            expected_short_size,
        )
        .await;

    // Bid order
    alice.mint_eth(1.).await;
    alice
        .assert_bid(DexAsset::ETH, DexMarket::ETH, true, 1800., 1., 10 * 1000)
        .await;
    alice.assert_eth_balance(0.).await;

    alice.mint_sol(10.).await;
    alice
        .assert_bid(DexAsset::SOL, DexMarket::ETH, false, 2200., 10., 10 * 1000)
        .await;

    // Cancel order all
    alice.cancel_call().await;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_long_size,
            expected_long_collateral,
            expected_long_size,
            0.,
        )
        .await;

    alice
        .assert_position(
            DexMarket::BTC,
            false,
            20000.,
            expected_short_size,
            expected_short_collateral,
            expected_short_borrow,
            0.,
        )
        .await;

    alice.assert_eth_balance(1.).await;
    alice.assert_no_order().await;
}
