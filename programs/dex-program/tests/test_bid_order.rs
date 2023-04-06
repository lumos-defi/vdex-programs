#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_bid_long_and_short() {
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
    alice.mint_btc(0.1).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 19000., 0.1, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    alice
        .assert_bid_order(DexAsset::BTC, DexMarket::BTC, true, 19000., 0.1, 10 * 1000)
        .await;

    // Alice bids another long with BTC
    alice.mint_btc(0.2).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 18000., 0.2, 8 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    alice
        .assert_bid_order(DexAsset::BTC, DexMarket::BTC, true, 18000., 0.2, 8 * 1000)
        .await;

    // Alice bid short with USDC
    alice.mint_usdc(2000.).await;
    alice
        .assert_bid(
            DexAsset::USDC,
            DexMarket::BTC,
            false,
            22000.,
            2000.,
            5 * 1000,
        )
        .await;
    alice.assert_usdc_balance(0.).await;
    alice
        .assert_bid_order(
            DexAsset::USDC,
            DexMarket::BTC,
            false,
            22000.,
            2000.,
            5 * 1000,
        )
        .await;

    // Alice bid short with ETH
    alice.mint_eth(1.).await;
    alice
        .assert_bid(DexAsset::ETH, DexMarket::BTC, false, 23000., 1., 5 * 1000)
        .await;
    alice.assert_eth_balance(0.).await;
    alice
        .assert_bid_order(DexAsset::ETH, DexMarket::BTC, false, 23000., 1., 5 * 1000)
        .await;
}

#[tokio::test]
async fn test_bid_fail_of_price() {
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

    // Long price should be less than 20000.
    alice.mint_btc(0.2).await;
    alice
        .assert_bid_fail(DexAsset::BTC, DexMarket::BTC, true, 21000., 0.2, 8 * 1000)
        .await;
    alice.assert_btc_balance(0.2).await;

    // Short price should be higher than 20000.
    alice.mint_usdc(2000.).await;
    alice
        .assert_bid_fail(
            DexAsset::USDC,
            DexMarket::BTC,
            false,
            18000.,
            2000.,
            5 * 1000,
        )
        .await;
    alice.assert_usdc_balance(2000.).await;
}

#[tokio::test]
async fn test_bid_fail_of_price_use_price_feed() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.mock_btc_price(20001.).await;
    user.mock_eth_price(2001.).await;
    user.mock_sol_price(21.).await;

    user.feed_usdc_price(1.).await;
    user.feed_btc_price(20000.).await;
    user.feed_eth_price(2000.).await;
    user.feed_sol_price(20.).await;

    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.add_liquidity_with_usdc(100000.).await;

    // Long price should be less than 20000.
    alice.mint_btc(0.2).await;
    alice
        .assert_bid_fail(DexAsset::BTC, DexMarket::BTC, true, 21000., 0.2, 8 * 1000)
        .await;
    alice.assert_btc_balance(0.2).await;

    // Short price should be higher than 20000.
    alice.mint_usdc(2000.).await;
    alice
        .assert_bid_fail(
            DexAsset::USDC,
            DexMarket::BTC,
            false,
            18000.,
            2000.,
            5 * 1000,
        )
        .await;
    alice.assert_usdc_balance(2000.).await;
}
