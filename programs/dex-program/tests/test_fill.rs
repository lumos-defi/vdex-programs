#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::{DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_fill_bid_order() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];

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

    // Alice bids another long with BTC
    alice.mint_btc(0.2).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 18000., 0.2, 8 * 1000)
        .await;

    // Bob bid short with USDC
    bob.mint_usdc(2000.).await;
    bob.assert_bid(
        DexAsset::USDC,
        DexMarket::BTC,
        false,
        22000.,
        2000.,
        5 * 1000,
    )
    .await;

    // Bob bid short with ETH
    bob.mint_eth(1.).await;
    bob.assert_bid(DexAsset::ETH, DexMarket::BTC, false, 23000., 1., 5 * 1000)
        .await;

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 19000., 22000.)
        .await;

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;

    // Market price change @ 19000
    user.mock_btc_price(19000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 18000., 22000.)
        .await;

    // Market price change @ 22500
    user.mock_btc_price(22500.).await;
    user.fill(DexMarket::BTC).await;

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 18000., 23000.)
        .await;

    // Market price change @ 18000
    user.mock_btc_price(18000.).await;
    user.fill(DexMarket::BTC).await;

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 0., 23000.)
        .await;
}

#[tokio::test]
async fn test_fill_ask_order() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(100000.).await;
    user.add_liquidity_with_btc(10.).await;
    user.mock_btc_price(20000.).await;

    // Alice open long
    alice.mint_btc(0.1).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;
    // Place ask long partially
    alice.assert_ask(DexMarket::BTC, true, 22000., 0.5).await;
    alice
        .assert_ask(DexMarket::BTC, true, 23000., u64::MAX as f64)
        .await;

    // Bob open short
    bob.mint_usdc(2000.).await;
    bob.assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    bob.assert_ask(DexMarket::BTC, false, 18000., 0.2).await;
    bob.assert_ask(DexMarket::BTC, false, 17000., 0.3).await;

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 18000., 22000.)
        .await;

    // Market price change @ 22500
    user.mock_btc_price(22500.).await;
    user.fill(DexMarket::BTC).await;

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 18000., 23000.)
        .await;

    // Market price change @ 18000
    user.mock_btc_price(18000.).await;
    user.fill(DexMarket::BTC).await;

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 17000., 23000.)
        .await;

    // Market price change @ 17000
    user.mock_btc_price(17000.).await;
    user.fill(DexMarket::BTC).await;

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 0., 23000.)
        .await;
}
