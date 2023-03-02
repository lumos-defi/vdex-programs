#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::{add_fee, minus_add_fee, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_close_size() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.feed_btc_price(20000.).await;
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
    let _expected_borrow = expected_size;

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

    assert_eq!(expected_size, 0.97087379);

    // Limit-price close the position @19000, order 0
    alice.assert_ask(DexMarket::BTC, true, 19000., 0.2).await;
    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_size,
            0.2,
        )
        .await;

    // Limit-price close the position @19000, order 1
    alice.assert_ask(DexMarket::BTC, true, 19000., 0.3).await;
    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            expected_size,
            expected_collateral,
            expected_size,
            0.5,
        )
        .await;

    alice.assert_close(DexMarket::BTC, true, 1000.).await; // market close the available size
    alice
        .assert_position(DexMarket::BTC, true, 20000., 0.5, 0.05, 0.5, 0.5)
        .await;

    // No filled orders right now
    user.fill(DexMarket::BTC).await;
    user.assert_no_match_event().await;
    user.assert_order_book_bid_max_ask_min(DexMarket::BTC, 19000., u64::MAX as f64)
        .await;

    // Market price change @ 19000
    user.feed_btc_price(19000.).await;
    user.fill(DexMarket::BTC).await;

    // Crank the filled order
    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 0);
    user.crank(true).await;

    let event = user.read_match_event().await;
    assert_eq!(event.user, alice.user.pubkey().to_bytes());
    assert_eq!(event.user_order_slot, 1);
    user.crank(true).await;

    user.assert_no_match_event().await;
}
