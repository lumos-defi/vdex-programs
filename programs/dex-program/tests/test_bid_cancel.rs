#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_cancel_basic() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.feed_btc_price(20000.).await;
    user.feed_eth_price(2000.).await;
    user.feed_sol_price(20.).await;

    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_eth(1000.).await;
    user.add_liquidity_with_usdc(100000.).await;

    // Alice bids long with BTC
    alice.mint_btc(0.1).await;
    alice
        .assert_bid(DexAsset::BTC, DexMarket::BTC, true, 19000., 0.1, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    alice.cancel(0).await;
    alice.assert_btc_balance(0.1).await;
}
