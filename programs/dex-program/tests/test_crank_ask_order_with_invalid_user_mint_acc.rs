#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::{
    add_fee, close_fee, collateral_to_size, minus_add_fee, DexAsset, DexMarket, TestResult,
};
use context::DexTestContext;

#[tokio::test]
async fn test_crank_ask_long_with_loss() {
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
    let expected_borrow = expected_size;

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
    user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.) - expected_borrow)
        .await;
    user.assert_fee(DexAsset::BTC, add_fee(10.) + expected_open_fee)
        .await;
    user.assert_borrow(DexAsset::BTC, expected_size).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Close the position @19000, borrow fee rate = 0
    alice.assert_ask(DexMarket::BTC, true, 19000., 1000.).await; // close all

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 19000., u64::MAX as f64)
        .await;

    // Market price change @ 19000
    user.mock_btc_price(19000.).await;
    user.fill(DexMarket::BTC).await;

    // Crank the filled order
    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    alice.close_mint_account(DexAsset::BTC).await;
    user.crank(false).await;
    user.assert_no_match_event().await;

    // Ask order should be filled
    let expected_close_fee = close_fee(expected_size);
    let expected_loss = expected_size * (20000. - 19000.) / 20000.;
    println!(
        "collateral {}, close fee {}, loss {}",
        expected_collateral, expected_close_fee, expected_loss
    );

    alice
        .assert_position(DexMarket::BTC, true, 0., 0., 0., 0., 0.)
        .await;

    // Check BTC liquidity
    user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.) + expected_loss)
        .await;
    user.assert_fee(
        DexAsset::BTC,
        add_fee(10.) + expected_open_fee + expected_close_fee,
    )
    .await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    // Check Alice's asset
    alice
        .assert_asset(
            DexAsset::BTC,
            expected_collateral - expected_close_fee - expected_loss,
        )
        .await;

    // Alice withdraws asset
    alice.create_mint_account(DexAsset::BTC).await;
    alice.withdraw_asset(DexAsset::BTC).await.assert_ok();

    alice
        .assert_btc_balance(expected_collateral - expected_close_fee - expected_loss)
        .await;
}

#[tokio::test]
async fn test_crank_ask_short_with_loss() {
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

    // Crank the filled order
    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    alice.close_mint_account(DexAsset::USDC).await;
    user.crank(false).await;
    user.assert_no_match_event().await;

    // Ask order should be filled
    let expected_close_fee = close_fee(expected_size) * 21000.;
    println!("close fee {}", expected_close_fee);
    let expected_loss = expected_size * (21000. - 20000.);

    alice
        .assert_position(DexMarket::BTC, false, 0., 0., 0., 0., 0.)
        .await;

    // Check USDC liquidity
    user.assert_liquidity(DexAsset::USDC, minus_add_fee(100000.) + expected_loss)
        .await;
    user.assert_fee(
        DexAsset::USDC,
        add_fee(100000.) + expected_open_fee + expected_close_fee,
    )
    .await;
    user.assert_borrow(DexAsset::USDC, 0.).await;
    user.assert_collateral(DexAsset::USDC, 0.).await;

    // Check Alice's asset
    alice
        .assert_asset(
            DexAsset::USDC,
            expected_collateral - expected_close_fee - expected_loss,
        )
        .await;

    // Alice withdraws asset
    alice.create_mint_account(DexAsset::USDC).await;
    alice.withdraw_asset(DexAsset::USDC).await.assert_ok();
    alice
        .assert_usdc_balance(expected_collateral - expected_close_fee - expected_loss)
        .await;
}

#[tokio::test]
async fn test_crank_ask_long_with_profit() {
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
    let expected_borrow = expected_size;

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
    user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.) - expected_borrow)
        .await;
    user.assert_fee(DexAsset::BTC, add_fee(10.) + expected_open_fee)
        .await;
    user.assert_borrow(DexAsset::BTC, expected_size).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Close the position @21000, borrow fee rate = 0
    alice.assert_ask(DexMarket::BTC, true, 21000., 1000.).await; // close all

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 0., 21000.)
        .await;

    // Market price change @ 21000
    user.mock_btc_price(21000.).await;
    user.fill(DexMarket::BTC).await;

    // Crank the filled order
    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    alice.close_mint_account(DexAsset::BTC).await;
    user.crank(false).await;
    user.assert_no_match_event().await;

    let expected_close_fee = close_fee(expected_size);
    let expected_profit = expected_size * (21000. - 20000.) / 20000.;
    println!(
        "collateral {}, close fee {}, loss {}",
        expected_collateral, expected_close_fee, expected_profit
    );

    alice
        .assert_position(DexMarket::BTC, true, 0., 0., 0., 0., 0.)
        .await;

    // Check BTC liquidity
    user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.) - expected_profit)
        .await;
    user.assert_fee(
        DexAsset::BTC,
        add_fee(10.) + expected_open_fee + expected_close_fee,
    )
    .await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    // Check Alice's asset
    alice
        .assert_asset(
            DexAsset::BTC,
            expected_collateral - expected_close_fee + expected_profit,
        )
        .await;

    // Alice withdraws asset
    alice.create_mint_account(DexAsset::BTC).await;
    alice.withdraw_asset(DexAsset::BTC).await.assert_ok();

    alice
        .assert_btc_balance(expected_collateral - expected_close_fee + expected_profit)
        .await;
}

#[tokio::test]
async fn test_crank_ask_short_with_profit() {
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

    // Close the position @19000, borrow fee rate = 0
    alice.assert_ask(DexMarket::BTC, false, 19000., 1000.).await; // close all

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 19000., u64::MAX as f64)
        .await;

    // Market price change @ 19000
    user.mock_btc_price(19000.).await;
    user.fill(DexMarket::BTC).await;

    // Crank the filled order
    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    alice.close_mint_account(DexAsset::USDC).await;
    user.crank(false).await;
    user.assert_no_match_event().await;

    let expected_close_fee = close_fee(expected_size) * 19000.;
    println!("close fee {}", expected_close_fee);
    let expected_profit = expected_size * (20000. - 19000.);

    alice
        .assert_position(DexMarket::BTC, false, 0., 0., 0., 0., 0.)
        .await;

    // Check USDC liquidity
    user.assert_liquidity(DexAsset::USDC, minus_add_fee(100000.) - expected_profit)
        .await;
    user.assert_fee(
        DexAsset::USDC,
        add_fee(100000.) + expected_open_fee + expected_close_fee,
    )
    .await;
    user.assert_borrow(DexAsset::USDC, 0.).await;
    user.assert_collateral(DexAsset::USDC, 0.).await;

    // Check Alice's asset
    alice
        .assert_asset(
            DexAsset::USDC,
            expected_collateral - expected_close_fee + expected_profit,
        )
        .await;

    // Alice withdraws asset
    alice.create_mint_account(DexAsset::USDC).await;
    alice.withdraw_asset(DexAsset::USDC).await.assert_ok();
    alice
        .assert_usdc_balance(expected_collateral - expected_close_fee + expected_profit)
        .await;
}
