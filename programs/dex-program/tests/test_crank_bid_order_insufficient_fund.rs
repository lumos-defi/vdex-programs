#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::{DexAsset, DexMarket, TestResult};
use context::DexTestContext;

#[tokio::test]
async fn test_crank_bid_long_insufficient_fund() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];

    // Prepare liquidity & price
    user.mock_btc_price(21000.).await;
    user.mock_eth_price(2000.).await;
    user.mock_sol_price(20.).await;

    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.add_liquidity_with_usdc(100000.).await;

    user.assert_liquidity(DexAsset::BTC, 10.).await;
    user.assert_fee(DexAsset::BTC, 0.).await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    // Alice bids long with BTC
    alice.mint_btc(0.5).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 20000., 0.5, 10 * 1000)
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

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 20000., 22000.)
        .await;

    // Market price change @ 20000
    user.mock_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Bob market-open long with BTC
    bob.mint_btc(0.7).await;
    bob.assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.7, 10 * 1000)
        .await;

    let expected_open_fee = 0.020388349;
    let expected_collateral = 0.7 - expected_open_fee;
    let expected_size = expected_collateral * 10.;

    bob.assert_position(
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
    user.assert_liquidity(DexAsset::BTC, 10. - expected_size)
        .await;
    user.assert_fee(DexAsset::BTC, expected_open_fee).await;
    user.assert_borrow(DexAsset::BTC, expected_size).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Crank the filled order. Alice will be refunded because of insufficient fund
    user.crank(true).await;
    user.assert_no_match_event().await;
    alice.assert_btc_balance(0.5).await;
}

#[tokio::test]
async fn test_crank_bid_long_insufficient_fund_with_invalid_user_mint_account() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];

    // Prepare liquidity & price
    user.mock_btc_price(21000.).await;
    user.mock_eth_price(2000.).await;
    user.mock_sol_price(20.).await;

    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.add_liquidity_with_usdc(100000.).await;

    user.assert_liquidity(DexAsset::BTC, 10.).await;
    user.assert_fee(DexAsset::BTC, 0.).await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    // Alice bids long with BTC
    alice.mint_btc(0.5).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 20000., 0.5, 10 * 1000)
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

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 20000., 22000.)
        .await;

    // Market price change @ 20000
    user.mock_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Bob market-open long with BTC
    // which leads to alice's order has no enough liquidity
    bob.mint_btc(0.7).await;
    bob.assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.7, 10 * 1000)
        .await;

    let expected_open_fee = 0.020388349;
    let expected_collateral = 0.7 - expected_open_fee;
    let expected_size = expected_collateral * 10.;

    bob.assert_position(
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
    user.assert_liquidity(DexAsset::BTC, 10. - expected_size)
        .await;
    user.assert_fee(DexAsset::BTC, expected_open_fee).await;
    user.assert_borrow(DexAsset::BTC, expected_size).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Mock the situation that alice's btc associate account does not exist
    alice.close_mint_account(DexAsset::BTC).await;
    user.crank(false).await;
    user.assert_no_match_event().await;

    // Check Alice's asset
    alice.assert_asset(DexAsset::BTC, 0.5).await;

    // Alice withdraws asset
    alice.create_mint_account(DexAsset::BTC).await;
    alice.withdraw_asset(DexAsset::BTC).await.assert_ok();

    alice.assert_btc_balance(0.5).await;
    alice.assert_asset(DexAsset::BTC, 0.).await;
}
