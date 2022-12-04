#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{add_fee, convert_to_big_number, minus_add_fee, swap_fee, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_open_btc_long_with_eth() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;

    // Assert BTC pool
    market.assert_liquidity(DexAsset::BTC, 10.).await;
    market.assert_fee(DexAsset::BTC, 0.).await;
    market.assert_borrow(DexAsset::BTC, 0.).await;
    market.assert_collateral(DexAsset::BTC, 0.).await;

    // Assert ETH pool
    market
        .assert_liquidity(DexAsset::ETH, minus_add_fee(1000.))
        .await;
    market.assert_fee(DexAsset::ETH, add_fee(1000.)).await;
    market.assert_borrow(DexAsset::ETH, 0.).await;
    market.assert_collateral(DexAsset::ETH, 0.).await;

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
    market
        .assert_liquidity(DexAsset::BTC, 10.0 - swapped_btc - expected_borrow)
        .await;
    market.assert_fee(DexAsset::BTC, expected_open_fee).await;
    market.assert_borrow(DexAsset::BTC, expected_borrow).await;
    market
        .assert_collateral(DexAsset::BTC, expected_collateral)
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
async fn test_open_btc_long_with_usdc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_usdc(100000.).await;

    // Assert BTC pool
    market.assert_liquidity(DexAsset::BTC, 10.).await;
    market.assert_fee(DexAsset::BTC, 0.).await;
    market.assert_borrow(DexAsset::BTC, 0.).await;
    market.assert_collateral(DexAsset::BTC, 0.).await;

    // Assert USDC pool
    market
        .assert_liquidity(DexAsset::USDC, minus_add_fee(100000.))
        .await;
    market.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
    market.assert_borrow(DexAsset::USDC, 0.).await;
    market.assert_collateral(DexAsset::USDC, 0.).await;

    // Alice open long
    let input_usdc = 2000.0;
    alice.mint_usdc(input_usdc).await;

    alice
        .open(DexAsset::USDC, DexMarket::BTC, true, input_usdc, 10)
        .await;
    alice.assert_usdc_balance(0.).await;

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
    market
        .assert_liquidity(DexAsset::BTC, 10.0 - swapped_btc - expected_borrow)
        .await;
    market.assert_fee(DexAsset::BTC, expected_open_fee).await;
    market.assert_borrow(DexAsset::BTC, expected_borrow).await;
    market
        .assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Check USDC liquidity
    market
        .assert_liquidity(
            DexAsset::USDC,
            minus_add_fee(100000.) + input_usdc - swap_fee,
        )
        .await;

    market
        .assert_fee(DexAsset::USDC, add_fee(100000.) + swap_fee)
        .await;
}

#[tokio::test]
async fn test_open_btc_long_with_sol() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, add_fee(1000.)).await;
    market.assert_borrow(DexAsset::SOL, 0.).await;
    market.assert_collateral(DexAsset::SOL, 0.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;

    // Assert BTC pool
    market
        .assert_liquidity(DexAsset::BTC, minus_add_fee(10.))
        .await;
    market.assert_fee(DexAsset::BTC, add_fee(10.)).await;
    market.assert_borrow(DexAsset::BTC, 0.).await;
    market.assert_collateral(DexAsset::BTC, 0.).await;

    market.assert_fee(DexAsset::SOL, 0.).await;

    // Alice open long
    let input_sol = 100.0;
    alice.mint_sol(input_sol).await;

    alice
        .open(DexAsset::SOL, DexMarket::BTC, true, input_sol, 10)
        .await;

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
    market
        .assert_liquidity(
            DexAsset::BTC,
            minus_add_fee(10.0) - swapped_btc - expected_borrow,
        )
        .await;
    market
        .assert_fee(DexAsset::BTC, add_fee(10.) + expected_open_fee)
        .await;
    market.assert_borrow(DexAsset::BTC, expected_borrow).await;
    market
        .assert_collateral(DexAsset::BTC, expected_collateral)
        .await;

    // Check SOL liquidity
    market
        .assert_liquidity(DexAsset::SOL, minus_add_fee(1000.) + input_sol - swap_fee)
        .await;

    market.assert_fee(DexAsset::SOL, swap_fee).await;
}
