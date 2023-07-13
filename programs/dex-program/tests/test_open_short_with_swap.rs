#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{
    add_fee, collateral_to_size, convert_to_big_number, minus_add_fee, swap_fee, DexAsset,
    DexMarket,
};
use context::DexTestContext;

#[tokio::test]
async fn test_open_btc_short_with_eth() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    // Alice open short
    let input_eth = 1.0;
    alice.mint_eth(input_eth).await;

    alice
        .assert_open(DexAsset::ETH, DexMarket::BTC, false, input_eth, 10 * 1000)
        .await;
    alice.assert_eth_balance(0.).await;

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
async fn test_open_btc_short_with_btc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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
        .assert_open(DexAsset::BTC, DexMarket::BTC, false, input_btc, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

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
async fn test_open_btc_short_with_sol() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    // Assert SOL
    user.add_liquidity_with_sol(1000.).await;
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
        .assert_open(DexAsset::SOL, DexMarket::BTC, false, input_sol, 10 * 1000)
        .await;

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
        .assert_liquidity(DexAsset::SOL, 1000. + input_sol - swap_fee)
        .await;

    market.assert_fee(DexAsset::SOL, swap_fee).await;
}
