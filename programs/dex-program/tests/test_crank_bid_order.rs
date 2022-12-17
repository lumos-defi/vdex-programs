#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::{
    add_fee, collateral_to_size, convert_to_big_number, minus_add_fee, swap_fee, DexAsset,
    DexMarket,
};
use context::DexTestContext;

#[tokio::test]
async fn test_crank_bid_long_no_swap() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.feed_btc_price(21000.).await;
    user.feed_eth_price(2000.).await;
    user.feed_sol_price(20.).await;

    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.add_liquidity_with_usdc(100000.).await;

    user.assert_liquidity(DexAsset::BTC, 10.).await;
    user.assert_fee(DexAsset::BTC, 0.).await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    // Alice bids long with BTC
    alice.mint_btc(0.1).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 20000., 0.1, 10 * 1000)
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
    user.feed_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Crank the filled order
    user.crank().await;
    user.assert_no_match_event().await;

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
    user.assert_liquidity(DexAsset::BTC, 10. - expected_size)
        .await;
    user.assert_fee(DexAsset::BTC, expected_open_fee).await;
    user.assert_borrow(DexAsset::BTC, expected_size).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;
}

#[tokio::test]
async fn test_crank_bid_long_with_eth() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.feed_btc_price(21000.).await;
    user.feed_eth_price(2000.).await;
    user.feed_sol_price(20.).await;

    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.add_liquidity_with_usdc(100000.).await;

    // Assert BTC pool
    user.assert_liquidity(DexAsset::BTC, 10.).await;
    user.assert_fee(DexAsset::BTC, 0.).await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    // Assert ETH pool
    user.assert_liquidity(DexAsset::ETH, 1000.).await;
    user.assert_fee(DexAsset::ETH, 0.).await;
    user.assert_borrow(DexAsset::ETH, 0.).await;
    user.assert_collateral(DexAsset::ETH, 0.).await;

    // Alice bids long with ETH
    let input_eth = 1.0;
    alice.mint_eth(input_eth).await;
    alice
        .assert_bid(
            DexAsset::ETH,
            DexMarket::BTC,
            true,
            20000.,
            input_eth,
            10 * 1000,
        )
        .await;
    alice.assert_eth_balance(0.).await;

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
    user.feed_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Crank the filled order
    user.crank().await;
    user.assert_no_match_event().await;

    let swap_fee = swap_fee(input_eth);
    assert_eq!(swap_fee, 0.001);

    let swapped_btc = (input_eth - swap_fee) * 2000.0 / 20000.;
    assert_eq!(swapped_btc, 0.0999);

    let expected_open_fee = 0.002909708;
    let expected_collateral = swapped_btc - expected_open_fee;
    let expected_size = expected_collateral * 10.;
    let expected_borrow = expected_size;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_borrow,
            0.,
        )
        .await;

    // Check BTC liquidity
    user.assert_liquidity(DexAsset::BTC, 10.0 - swapped_btc - expected_borrow)
        .await;
    user.assert_fee(DexAsset::BTC, expected_open_fee).await;
    user.assert_borrow(DexAsset::BTC, expected_borrow).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Check ETH liquidity
    user.assert_liquidity(DexAsset::ETH, 1000. + input_eth - swap_fee)
        .await;
    user.assert_fee(DexAsset::ETH, swap_fee).await;
    user.assert_borrow(DexAsset::ETH, 0.).await;
    user.assert_collateral(DexAsset::ETH, 0.).await;
}

#[tokio::test]
async fn test_crank_bid_long_with_usdc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];

    // Prepare liquidity & price
    user.feed_btc_price(21000.).await;
    user.feed_eth_price(2000.).await;
    user.feed_sol_price(20.).await;

    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.add_liquidity_with_usdc(100000.).await;

    // Assert BTC pool
    user.assert_liquidity(DexAsset::BTC, 10.).await;
    user.assert_fee(DexAsset::BTC, 0.).await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    // Assert USDC pool
    user.assert_liquidity(DexAsset::USDC, minus_add_fee(100000.))
        .await;
    user.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
    user.assert_borrow(DexAsset::USDC, 0.).await;
    user.assert_collateral(DexAsset::USDC, 0.).await;

    // Alice bids long with USDC
    let input_usdc = 2000.0;
    alice.mint_usdc(input_usdc).await;
    alice
        .assert_bid(
            DexAsset::USDC,
            DexMarket::BTC,
            true,
            20000.,
            input_usdc,
            10 * 1000,
        )
        .await;
    alice.assert_usdc_balance(0.).await;

    // Alice bid short with USDC
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

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 20000., 22000.)
        .await;

    // Market price change @ 20000
    user.feed_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Crank the filled order
    user.crank().await;
    user.assert_no_match_event().await;

    let swap_fee = swap_fee(input_usdc);
    assert_eq!(swap_fee, 2.0);

    let swapped_btc = (input_usdc - swap_fee) / 20000.;
    assert_eq!(swapped_btc, 0.0999);

    let expected_open_fee = 0.002909708;
    let expected_collateral = swapped_btc - expected_open_fee;
    let expected_size = expected_collateral * 10.;
    let expected_borrow = expected_size;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_borrow,
            0.,
        )
        .await;

    // Check BTC liquidity
    user.assert_liquidity(DexAsset::BTC, 10.0 - swapped_btc - expected_borrow)
        .await;
    user.assert_fee(DexAsset::BTC, expected_open_fee).await;
    user.assert_borrow(DexAsset::BTC, expected_borrow).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Check USDC liquidity
    user.assert_liquidity(
        DexAsset::USDC,
        minus_add_fee(100000.) + input_usdc - swap_fee,
    )
    .await;

    user.assert_fee(DexAsset::USDC, add_fee(100000.) + swap_fee)
        .await;
}

#[tokio::test]
async fn test_crank_bid_long_with_sol() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.feed_btc_price(21000.).await;
    user.feed_eth_price(2000.).await;
    user.feed_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    user.assert_liquidity(DexAsset::SOL, 999.).await;
    user.assert_fee(DexAsset::SOL, add_fee(1000.)).await;
    user.assert_borrow(DexAsset::SOL, 0.).await;
    user.assert_collateral(DexAsset::SOL, 0.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;

    // Assert BTC pool
    user.assert_liquidity(DexAsset::BTC, minus_add_fee(10.))
        .await;
    user.assert_fee(DexAsset::BTC, add_fee(10.)).await;
    user.assert_borrow(DexAsset::BTC, 0.).await;
    user.assert_collateral(DexAsset::BTC, 0.).await;

    user.assert_fee(DexAsset::SOL, 0.).await;

    // Alice bids long with SOL
    let input_sol = 100.0;
    alice.mint_sol(input_sol).await;
    alice
        .assert_bid(
            DexAsset::SOL,
            DexMarket::BTC,
            true,
            20000.,
            input_sol,
            10 * 1000,
        )
        .await;

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 20000., u64::MAX as f64)
        .await;

    // Market price change @ 20000
    user.feed_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Crank the filled order
    user.crank().await;
    user.assert_no_match_event().await;

    let swap_fee = swap_fee(input_sol);
    assert_eq!(swap_fee, 0.1);

    let swapped_btc = (input_sol - swap_fee) * 20. / 20000.;
    assert_eq!(swapped_btc, 0.0999);

    let expected_open_fee = 0.002909708;
    let expected_collateral = swapped_btc - expected_open_fee;
    let expected_size = expected_collateral * 10.;
    let expected_borrow = expected_size;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_borrow,
            0.,
        )
        .await;

    // Check BTC liquidity
    user.assert_liquidity(
        DexAsset::BTC,
        minus_add_fee(10.0) - swapped_btc - expected_borrow,
    )
    .await;
    user.assert_fee(DexAsset::BTC, add_fee(10.) + expected_open_fee)
        .await;
    user.assert_borrow(DexAsset::BTC, expected_borrow).await;
    user.assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Check SOL liquidity
    user.assert_liquidity(DexAsset::SOL, minus_add_fee(1000.) + input_sol - swap_fee)
        .await;

    user.assert_fee(DexAsset::SOL, swap_fee).await;
}
#[tokio::test]
async fn test_crank_bid_short_no_swap() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(100000.).await;
    user.add_liquidity_with_btc(10.).await;
    user.feed_btc_price(18000.).await;

    // Alice bids short with usdc
    alice.mint_usdc(2000.).await;
    alice
        .assert_bid(
            DexAsset::USDC,
            DexMarket::BTC,
            false,
            20000.,
            2000.,
            10 * 1000,
        )
        .await;
    alice.assert_usdc_balance(0.).await;

    // Bob bids long with BTC
    bob.mint_btc(0.1).await;
    bob.assert_bid(DexAsset::BTC, DexMarket::BTC, true, 17000., 0.1, 10 * 1000)
        .await;
    bob.assert_btc_balance(0.).await;

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 17000., 20000.)
        .await;

    // Market price change @ 20000
    user.feed_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Crank the filled order
    user.crank().await;
    user.assert_no_match_event().await;

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
    user.assert_liquidity(DexAsset::USDC, 100000. - expect_borrow)
        .await;
    user.assert_fee(DexAsset::USDC, expected_open_fee).await;
    user.assert_borrow(DexAsset::USDC, expect_borrow).await;
    user.assert_collateral(DexAsset::USDC, expected_collateral)
        .await;
}

#[tokio::test]
async fn test_crank_bid_short_with_eth() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.feed_btc_price(18000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(200000.).await;
    user.add_liquidity_with_eth(1000.).await;

    // Assert USDC pool
    market.assert_liquidity(DexAsset::USDC, 200000.).await;
    market.assert_fee(DexAsset::USDC, 0.).await;
    market.assert_borrow(DexAsset::USDC, 0.).await;
    market.assert_collateral(DexAsset::USDC, 0.).await;

    // Assert ETH pool
    market
        .assert_liquidity(DexAsset::ETH, minus_add_fee(1000.))
        .await;
    market.assert_fee(DexAsset::ETH, add_fee(1000.)).await;
    market.assert_borrow(DexAsset::ETH, 0.).await;
    market.assert_collateral(DexAsset::ETH, 0.).await;

    // Alice bid short
    let input_eth = 1.0;
    alice.mint_eth(input_eth).await;
    alice
        .assert_bid(
            DexAsset::ETH,
            DexMarket::BTC,
            false,
            20000.,
            input_eth,
            10 * 1000,
        )
        .await;
    alice.assert_eth_balance(0.).await;

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 0., 20000.)
        .await;

    // Market price change @ 20000
    user.feed_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Crank the filled order
    user.crank().await;
    user.assert_no_match_event().await;

    let swap_fee = swap_fee(input_eth);
    assert_eq!(swap_fee, 0.001);

    let swapped_usdc = (input_eth - swap_fee) * 2000.0;
    assert_eq!(swapped_usdc, 1998.);

    let expected_open_fee = 58.194174;
    let expected_collateral = swapped_usdc - expected_open_fee;
    let expected_size = collateral_to_size(expected_collateral, 10., 20000., 9);
    let expected_borrow = expected_collateral * 10.;

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

    // Check USDC liquidity
    market
        .assert_liquidity(DexAsset::USDC, 200000.0 - swapped_usdc - expected_borrow)
        .await;
    market.assert_fee(DexAsset::USDC, expected_open_fee).await;
    market.assert_borrow(DexAsset::USDC, expected_borrow).await;
    market
        .assert_collateral(DexAsset::USDC, expected_collateral)
        .await;

    // Check ETH liquidity
    market
        .assert_liquidity(DexAsset::ETH, minus_add_fee(1000.) + input_eth - swap_fee)
        .await;

    // Failed because f64 loses precision
    // market
    //     .assert_fee(DexAsset::ETH, add_fee(1000.) + swap_fee)
    //     .await;
    // Workaround to check fee
    let big_number = convert_to_big_number(add_fee(1000.), 6) + convert_to_big_number(swap_fee, 6);
    market.assert_fee_big(DexAsset::ETH, big_number).await;
}

#[tokio::test]
async fn test_crank_bid_short_with_btc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.feed_btc_price(18000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(200000.).await;
    user.add_liquidity_with_btc(10.).await;

    // Assert USDC pool
    market.assert_liquidity(DexAsset::USDC, 200000.).await;
    market.assert_fee(DexAsset::USDC, 0.).await;
    market.assert_borrow(DexAsset::USDC, 0.).await;
    market.assert_collateral(DexAsset::USDC, 0.).await;

    // Assert BTC pool
    market
        .assert_liquidity(DexAsset::BTC, minus_add_fee(10.))
        .await;
    market.assert_fee(DexAsset::BTC, add_fee(10.)).await;
    market.assert_borrow(DexAsset::BTC, 0.).await;
    market.assert_collateral(DexAsset::BTC, 0.).await;

    // Alice open short
    let input_btc = 0.1;
    alice.mint_btc(input_btc).await;
    alice
        .assert_bid(
            DexAsset::BTC,
            DexMarket::BTC,
            false,
            20000.,
            input_btc,
            10 * 1000,
        )
        .await;

    alice.assert_btc_balance(0.).await;

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 0., 20000.)
        .await;

    // Market price change @ 20000
    user.feed_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Crank the filled order
    user.crank().await;
    user.assert_no_match_event().await;

    let swap_fee = swap_fee(input_btc);
    assert_eq!(swap_fee, 0.0001);

    let swapped_usdc = (input_btc - swap_fee) * 20000.0;
    assert_eq!(swapped_usdc, 1998.);

    let expected_open_fee = 58.194174;
    let expected_collateral = swapped_usdc - expected_open_fee;
    let expected_size = collateral_to_size(expected_collateral, 10., 20000., 9);
    let expected_borrow = expected_collateral * 10.;

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

    // Check USDC liquidity
    market
        .assert_liquidity(DexAsset::USDC, 200000.0 - swapped_usdc - expected_borrow)
        .await;
    market.assert_fee(DexAsset::USDC, expected_open_fee).await;
    market.assert_borrow(DexAsset::USDC, expected_borrow).await;
    market
        .assert_collateral(DexAsset::USDC, expected_collateral)
        .await;

    // Check BTC liquidity
    market
        .assert_liquidity(DexAsset::BTC, minus_add_fee(10.) + input_btc - swap_fee)
        .await;

    market
        .assert_fee(DexAsset::BTC, add_fee(10.) + swap_fee)
        .await;
}

#[tokio::test]
async fn test_crank_bid_short_with_sol() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.feed_btc_price(18000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity
    user.add_liquidity_with_usdc(200000.).await;

    // Assert USDC pool
    market
        .assert_liquidity(DexAsset::USDC, minus_add_fee(200000.))
        .await;
    market.assert_fee(DexAsset::USDC, add_fee(200000.)).await;
    market.assert_borrow(DexAsset::USDC, 0.).await;
    market.assert_collateral(DexAsset::USDC, 0.).await;

    market.assert_fee(DexAsset::SOL, 0.).await;

    // Alice open short
    let input_sol = 100.;
    alice.mint_sol(input_sol).await;
    alice
        .assert_bid(
            DexAsset::SOL,
            DexMarket::BTC,
            false,
            20000.,
            input_sol,
            10 * 1000,
        )
        .await;

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 0., 20000.)
        .await;

    // Market price change @ 20000
    user.feed_btc_price(20000.).await;
    user.fill(DexMarket::BTC).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);

    // Crank the filled order
    user.crank().await;
    user.assert_no_match_event().await;

    let swap_fee = swap_fee(input_sol);
    assert_eq!(swap_fee, 0.1);

    let swapped_usdc = (input_sol - swap_fee) * 20.0;
    assert_eq!(swapped_usdc, 1998.);

    let expected_open_fee = 58.194174;
    let expected_collateral = swapped_usdc - expected_open_fee;
    let expected_size = collateral_to_size(expected_collateral, 10., 20000., 9);
    let expected_borrow = expected_collateral * 10.;

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

    // Check USDC liquidity
    market
        .assert_liquidity(
            DexAsset::USDC,
            minus_add_fee(200000.) - swapped_usdc - expected_borrow,
        )
        .await;

    // Failed because f64 loses precision
    // market
    //     .assert_fee(DexAsset::USDC, add_fee(200000.) + expected_open_fee)
    //     .await;
    // Workaround to check fee
    let big_number =
        convert_to_big_number(add_fee(200000.), 6) + convert_to_big_number(expected_open_fee, 6);
    market.assert_fee_big(DexAsset::USDC, big_number).await;

    market.assert_borrow(DexAsset::USDC, expected_borrow).await;
    market
        .assert_collateral(DexAsset::USDC, expected_collateral)
        .await;

    // Check SOL liquidity
    market
        .assert_liquidity(DexAsset::SOL, 999. + input_sol - swap_fee)
        .await;

    market.assert_fee(DexAsset::SOL, swap_fee).await;
}
