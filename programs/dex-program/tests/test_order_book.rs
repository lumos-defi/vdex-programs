#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_bid_max_ask_min() {
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

    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 0., u64::MAX as f64)
        .await;

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

    // Alice bid short with ETH
    alice.mint_eth(1.).await;
    alice
        .assert_bid(DexAsset::ETH, DexMarket::BTC, false, 23000., 1., 5 * 1000)
        .await;

    // Check order book bid max & ask minimum
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 19000., 22000.)
        .await;
}
