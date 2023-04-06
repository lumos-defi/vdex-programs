#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::{add_fee, collateral_to_size, minus_add_fee, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_cannot_cancel_filled_bid_order() {
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

    // ALice can not cancel order @19000
    alice.fail_to_cancel(0).await
}

#[tokio::test]
async fn test_cannot_cancel_filled_ask_order() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(100000.).await;
    user.mock_btc_price(20000.).await;
    user.assert_liquidity(DexAsset::USDC, minus_add_fee(100000.))
        .await;
    user.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
    user.assert_borrow(DexAsset::USDC, 0.).await;
    user.assert_collateral(DexAsset::USDC, 0.).await;

    // Alice open long
    alice.mint_usdc(2000.).await;

    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;

    let expected_open_fee = 58.252427;
    let expected_collateral = 2000. - expected_open_fee;
    let expected_size = collateral_to_size(expected_collateral, 10., 20000., 9);
    let expected_borrow = expected_collateral * 10.;
    println!("close size {}", expected_size);

    alice
        .assert_position(
            DexMarket::BTC,
            false,
            20000.,
            expected_size,
            expected_collateral,
            expected_borrow,
            0.,
        )
        .await;

    // Check liquidity pool
    user.assert_liquidity(DexAsset::USDC, minus_add_fee(100000.) - expected_borrow)
        .await;
    user.assert_fee(DexAsset::USDC, add_fee(100000.) + expected_open_fee)
        .await;
    user.assert_borrow(DexAsset::USDC, expected_borrow).await;
    user.assert_collateral(DexAsset::USDC, expected_collateral)
        .await;

    // Close the position @21000, borrow fee rate = 0
    alice.assert_ask(DexMarket::BTC, false, 21000., 1000.).await; // close all

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 0., 21000.)
        .await;

    // Market price change @ 21000
    user.mock_btc_price(21000.).await;
    user.fill(DexMarket::BTC).await;

    // ALice can not cancel order @21000
    alice.fail_to_cancel(0).await
}
