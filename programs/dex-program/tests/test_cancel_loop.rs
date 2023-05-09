#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_loop_cancel_bid_and_ask_orders() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.add_liquidity_with_usdc(100000.).await;
    user.add_liquidity_with_eth(100.).await;
    user.mock_btc_price(20000.).await;
    user.mock_eth_price(2000.).await;

    // Alice open long
    alice.mint_btc(0.1).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    let expected_open_long_fee = 0.002912621;
    let expected_long_collateral = 0.1 - expected_open_long_fee;
    let expected_long_size = expected_long_collateral * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_long_size,
            expected_long_collateral,
            expected_long_size,
            0.,
        )
        .await;
    alice.mint_eth(1.).await;
    // alice.mint_sol(10.).await;

    for _ in 0..32 {
        alice.assert_ask(DexMarket::BTC, true, 22000., 0.5).await;

        alice
            .assert_bid(DexAsset::ETH, DexMarket::ETH, true, 1800., 1., 10 * 1000)
            .await;
        alice.assert_eth_balance(0.).await;

        alice
            .assert_bid(DexAsset::SOL, DexMarket::ETH, false, 2200., 10., 10 * 1000)
            .await;
        dtc.advance_clock(1).await;

        for order in alice.collect_orders().await {
            alice.cancel(order).await;
            dtc.advance_clock(1).await;
        }

        alice
            .assert_position(
                DexMarket::BTC,
                true,
                20000.,
                expected_long_size,
                expected_long_collateral,
                expected_long_size,
                0.,
            )
            .await;

        alice.assert_no_order().await;
    }
}
