#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{add_fee, minus_add_fee, swap_fee, DexAsset};
use context::DexTestContext;

#[tokio::test]
async fn test_btc_to_usdc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    // Alice swap btc for usdc
    let btc_amount = 0.1;
    alice.mint_btc(btc_amount).await;
    alice
        .market_swap(DexAsset::BTC, DexAsset::USDC, btc_amount)
        .await;
    alice.assert_btc_balance(0.).await;

    let swap_btc_fee = swap_fee(btc_amount);
    assert_eq!(swap_btc_fee, 0.0001);
    let usdc_equivalent = (btc_amount - swap_btc_fee) * 20000.;
    alice.assert_usdc_balance(usdc_equivalent).await;

    market
        .assert_liquidity(DexAsset::BTC, 10. + btc_amount - swap_btc_fee)
        .await;
    market.assert_fee(DexAsset::BTC, swap_btc_fee).await;

    market
        .assert_liquidity(DexAsset::USDC, minus_add_fee(100000.) - usdc_equivalent)
        .await;
    market.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
}

#[tokio::test]
async fn test_btc_to_eth() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(200.).await;

    // Assert BTC pool
    market.assert_liquidity(DexAsset::BTC, 10.).await;
    market.assert_fee(DexAsset::BTC, 0.).await;
    market.assert_borrow(DexAsset::BTC, 0.).await;
    market.assert_collateral(DexAsset::BTC, 0.).await;

    // Assert ETH pool
    market
        .assert_liquidity(DexAsset::ETH, minus_add_fee(200.))
        .await;
    market.assert_fee(DexAsset::ETH, add_fee(200.)).await;
    market.assert_borrow(DexAsset::ETH, 0.).await;
    market.assert_collateral(DexAsset::ETH, 0.).await;

    // Alice swap btc for eth
    let btc_amount = 0.1;
    alice.mint_btc(btc_amount).await;
    alice
        .market_swap(DexAsset::BTC, DexAsset::ETH, btc_amount)
        .await;
    alice.assert_btc_balance(0.).await;

    let swap_btc_fee = swap_fee(btc_amount);
    assert_eq!(swap_btc_fee, 0.0001);
    let eth_equivalent = (btc_amount - swap_btc_fee) * 20000. / 2000.;
    alice.assert_eth_balance(eth_equivalent).await;

    market
        .assert_liquidity(DexAsset::BTC, 10. + btc_amount - swap_btc_fee)
        .await;
    market.assert_fee(DexAsset::BTC, swap_btc_fee).await;

    market
        .assert_liquidity(DexAsset::ETH, minus_add_fee(200.) - eth_equivalent)
        .await;
    market.assert_fee(DexAsset::ETH, add_fee(200.)).await;
}

#[tokio::test]
async fn test_btc_to_sol() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;

    // Assert BTC pool
    market
        .assert_liquidity(DexAsset::BTC, minus_add_fee(10.))
        .await;
    market.assert_fee(DexAsset::BTC, add_fee(10.)).await;
    market.assert_borrow(DexAsset::BTC, 0.).await;
    market.assert_collateral(DexAsset::BTC, 0.).await;

    // Alice swap btc for sol
    let btc_amount = 0.1;
    alice.mint_btc(btc_amount).await;
    let _balance_before_swap = alice.balance().await;
    alice
        .market_swap(DexAsset::BTC, DexAsset::SOL, btc_amount)
        .await;
    alice.assert_btc_balance(0.).await;
    let _balance_after_swap = alice.balance().await;

    let swap_btc_fee = swap_fee(btc_amount);
    assert_eq!(swap_btc_fee, 0.0001);
    let sol_equivalent = (btc_amount - swap_btc_fee) * 20000. / 20.;

    market
        .assert_liquidity(
            DexAsset::BTC,
            minus_add_fee(10.) + btc_amount - swap_btc_fee,
        )
        .await;
    market
        .assert_fee(DexAsset::BTC, add_fee(10.) + swap_btc_fee)
        .await;

    market
        .assert_liquidity(DexAsset::SOL, 999. - sol_equivalent)
        .await;
}

#[tokio::test]
async fn test_usdc_to_btc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    // Alice swap btc for usdc
    let usdc_amount = 2000.;
    alice.mint_usdc(usdc_amount).await;
    alice
        .market_swap(DexAsset::USDC, DexAsset::BTC, usdc_amount)
        .await;
    alice.assert_usdc_balance(0.).await;

    let swap_usdc_fee = swap_fee(usdc_amount);
    assert_eq!(swap_usdc_fee, 2.);
    let btc_equivalent = (usdc_amount - swap_usdc_fee) / 20000.;
    alice.assert_btc_balance(btc_equivalent).await;

    market
        .assert_liquidity(DexAsset::BTC, 10. - btc_equivalent)
        .await;
    market.assert_fee(DexAsset::BTC, 0.).await;

    market
        .assert_liquidity(
            DexAsset::USDC,
            minus_add_fee(100000.) + usdc_amount - swap_usdc_fee,
        )
        .await;
    market
        .assert_fee(DexAsset::USDC, add_fee(100000.) + swap_usdc_fee)
        .await;
}

#[tokio::test]
async fn test_usdc_to_eth() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_eth(100.).await;
    user.add_liquidity_with_usdc(100000.).await;

    // Assert ETH pool
    market.assert_liquidity(DexAsset::ETH, 100.).await;
    market.assert_fee(DexAsset::ETH, 0.).await;
    market.assert_borrow(DexAsset::ETH, 0.).await;
    market.assert_collateral(DexAsset::ETH, 0.).await;

    // Assert USDC pool
    market
        .assert_liquidity(DexAsset::USDC, minus_add_fee(100000.))
        .await;
    market.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
    market.assert_borrow(DexAsset::USDC, 0.).await;
    market.assert_collateral(DexAsset::USDC, 0.).await;

    // Alice swap btc for usdc
    let usdc_amount = 2000.;
    alice.mint_usdc(usdc_amount).await;
    alice
        .market_swap(DexAsset::USDC, DexAsset::ETH, usdc_amount)
        .await;
    alice.assert_usdc_balance(0.).await;

    let swap_usdc_fee = swap_fee(usdc_amount);
    assert_eq!(swap_usdc_fee, 2.);
    let eth_equivalent = (usdc_amount - swap_usdc_fee) / 2000.;
    alice.assert_eth_balance(eth_equivalent).await;

    market
        .assert_liquidity(DexAsset::ETH, 100. - eth_equivalent)
        .await;
    market.assert_fee(DexAsset::ETH, 0.).await;

    market
        .assert_liquidity(
            DexAsset::USDC,
            minus_add_fee(100000.) + usdc_amount - swap_usdc_fee,
        )
        .await;
    market
        .assert_fee(DexAsset::USDC, add_fee(100000.) + swap_usdc_fee)
        .await;
}

#[tokio::test]
async fn test_usdc_to_sol() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(100000.).await;

    // Assert USDC pool
    market
        .assert_liquidity(DexAsset::USDC, minus_add_fee(100000.))
        .await;
    market.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
    market.assert_borrow(DexAsset::USDC, 0.).await;
    market.assert_collateral(DexAsset::USDC, 0.).await;

    // Alice swap btc for usdc
    let usdc_amount = 2000.;
    alice.mint_usdc(usdc_amount).await;
    alice
        .market_swap(DexAsset::USDC, DexAsset::SOL, usdc_amount)
        .await;
    alice.assert_usdc_balance(0.).await;

    let swap_usdc_fee = swap_fee(usdc_amount);
    assert_eq!(swap_usdc_fee, 2.);
    let sol_equivalent = (usdc_amount - swap_usdc_fee) / 20.;

    market
        .assert_liquidity(DexAsset::SOL, 999. - sol_equivalent)
        .await;

    market
        .assert_liquidity(
            DexAsset::USDC,
            minus_add_fee(100000.) + usdc_amount - swap_usdc_fee,
        )
        .await;
    market
        .assert_fee(DexAsset::USDC, add_fee(100000.) + swap_usdc_fee)
        .await;
}

#[tokio::test]
async fn test_sol_to_usdc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(100000.).await;

    // Assert USDC pool
    market
        .assert_liquidity(DexAsset::USDC, minus_add_fee(100000.))
        .await;
    market.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
    market.assert_borrow(DexAsset::USDC, 0.).await;
    market.assert_collateral(DexAsset::USDC, 0.).await;

    // Alice swap btc for usdc
    let sol_amount = 100.;
    alice.mint_sol(sol_amount).await;
    alice
        .market_swap(DexAsset::SOL, DexAsset::USDC, sol_amount)
        .await;

    let swap_sol_fee = swap_fee(sol_amount);
    assert_eq!(swap_sol_fee, 0.1);
    let usdc_equivalent = (sol_amount - swap_sol_fee) * 20.;
    alice.assert_usdc_balance(usdc_equivalent).await;

    market
        .assert_liquidity(DexAsset::SOL, 999. + sol_amount - swap_sol_fee)
        .await;
    market.assert_fee(DexAsset::SOL, swap_sol_fee).await;

    market
        .assert_liquidity(DexAsset::USDC, minus_add_fee(100000.) - usdc_equivalent)
        .await;
    market.assert_fee(DexAsset::USDC, add_fee(100000.)).await;
}

#[tokio::test]
async fn test_sol_to_btc() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let market = &dtc.user_context[2];

    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;

    // Assert BTC pool
    market
        .assert_liquidity(DexAsset::BTC, minus_add_fee(10.))
        .await;
    market.assert_fee(DexAsset::BTC, add_fee(10.)).await;
    market.assert_borrow(DexAsset::BTC, 0.).await;
    market.assert_collateral(DexAsset::BTC, 0.).await;

    // Alice swap btc for usdc
    let sol_amount = 100.;
    alice.mint_sol(sol_amount).await;
    alice
        .market_swap(DexAsset::SOL, DexAsset::BTC, sol_amount)
        .await;

    let swap_sol_fee = swap_fee(sol_amount);
    assert_eq!(swap_sol_fee, 0.1);
    let btc_equivalent = (sol_amount - swap_sol_fee) * 20. / 20000.;
    alice.assert_btc_balance(btc_equivalent).await;

    market
        .assert_liquidity(DexAsset::SOL, 999. + sol_amount - swap_sol_fee)
        .await;
    market.assert_fee(DexAsset::SOL, swap_sol_fee).await;

    market
        .assert_liquidity(DexAsset::BTC, minus_add_fee(10.) - btc_equivalent)
        .await;
    market.assert_fee(DexAsset::BTC, add_fee(10.)).await;
}
