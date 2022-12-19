#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{collateral_to_size, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_global_long_same_price() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];
    let mike = &dtc.user_context[3];

    user.add_liquidity_with_btc(10.).await;
    user.feed_btc_price(20000.).await;

    // Open positions
    alice.mint_btc(0.1).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;

    bob.mint_btc(0.1).await;
    bob.assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;

    let expected_open_fee = 0.002912621;
    let expected_collateral = 0.1 - expected_open_fee;
    let expected_size = expected_collateral * 10.;

    user.assert_global_long(DexMarket::BTC, 20000., expected_size * 2.0)
        .await;

    mike.mint_btc(0.1).await;
    mike.assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;

    user.assert_global_long(DexMarket::BTC, 20000., expected_size * 3.0)
        .await;

    // Close positions
    alice.assert_close(DexMarket::BTC, true, 1000.).await;
    user.assert_global_long(DexMarket::BTC, 20000., expected_size * 2.0)
        .await;

    bob.assert_close(DexMarket::BTC, true, 1000.).await;
    user.assert_global_long(DexMarket::BTC, 20000., expected_size * 1.0)
        .await;

    mike.assert_close(DexMarket::BTC, true, 1000.).await;
    user.assert_global_long(DexMarket::BTC, 0., 0.0).await;
}

#[tokio::test]
async fn test_global_long_different_price() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];
    let mike = &dtc.user_context[3];

    user.add_liquidity_with_btc(10.).await;
    user.feed_btc_price(20000.).await;

    // Open positions
    alice.mint_btc(0.1).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;

    user.feed_btc_price(22000.).await;
    bob.mint_btc(0.1).await;
    bob.assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;

    let expected_open_fee = 0.002912621;
    let expected_collateral = 0.1 - expected_open_fee;
    let expected_size = expected_collateral * 10.;

    user.assert_global_long(DexMarket::BTC, 21000., expected_size * 2.0)
        .await;

    user.feed_btc_price(23000.).await;
    mike.mint_btc(0.2).await;
    mike.assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.2, 10 * 1000)
        .await;

    user.assert_global_long(DexMarket::BTC, 22000., expected_size * 4.0)
        .await;

    // Close positions
    alice.assert_close(DexMarket::BTC, true, 1000.).await;
    user.assert_global_long(DexMarket::BTC, 22000., expected_size * 3.0)
        .await;

    bob.assert_close(DexMarket::BTC, true, 1000.).await;
    user.assert_global_long(DexMarket::BTC, 22000., expected_size * 2.0)
        .await;

    mike.assert_close(DexMarket::BTC, true, 1000.).await;
    user.assert_global_long(DexMarket::BTC, 0., 0.0).await;
}

#[tokio::test]
async fn test_global_short_same_price() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];
    let mike = &dtc.user_context[3];

    user.add_liquidity_with_usdc(100000.).await;
    user.feed_btc_price(20000.).await;

    // Open positions
    alice.mint_usdc(2000.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;

    bob.mint_usdc(2000.).await;
    bob.assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;

    let expected_open_fee = 58.252427;
    let expected_collateral = 2000. - expected_open_fee;
    let expected_size = collateral_to_size(expected_collateral, 10., 20000., 9);

    user.assert_global_short(DexMarket::BTC, 20000., expected_size * 2.0)
        .await;

    mike.mint_usdc(2000.).await;
    mike.assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;

    user.assert_global_short(DexMarket::BTC, 20000., expected_size * 3.0)
        .await;

    // Close positions
    alice.assert_close(DexMarket::BTC, false, 1000.).await;
    user.assert_global_short(DexMarket::BTC, 20000., expected_size * 2.0)
        .await;

    bob.assert_close(DexMarket::BTC, false, 1000.).await;
    user.assert_global_short(DexMarket::BTC, 20000., expected_size * 1.0)
        .await;

    mike.assert_close(DexMarket::BTC, false, 1000.).await;
    user.assert_global_short(DexMarket::BTC, 0., 0.0).await;
}

#[tokio::test]
async fn test_global_short_different_price() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];
    let mike = &dtc.user_context[3];

    user.add_liquidity_with_usdc(100000.).await;
    user.feed_btc_price(20000.).await;

    // Open positions
    alice.mint_usdc(2000.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    let expected_open_fee = 58.252427;
    let expected_collateral = 2000. - expected_open_fee;
    let alice_size = collateral_to_size(expected_collateral, 10., 20000., 9);

    user.feed_btc_price(22000.).await;
    bob.mint_usdc(2000.).await;
    bob.assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;

    let bob_size = collateral_to_size(expected_collateral, 10., 22000., 9);

    let merge_price = (20000. * alice_size + 22000. * bob_size) / (alice_size + bob_size);
    user.assert_global_short(DexMarket::BTC, merge_price, alice_size + bob_size)
        .await;

    user.feed_btc_price(24000.).await;
    mike.mint_usdc(2000.).await;
    mike.assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    let mike_size = collateral_to_size(expected_collateral, 10., 24000., 9);

    let merge_price = (20000. * alice_size + 22000. * bob_size + 24000. * mike_size)
        / (alice_size + bob_size + mike_size);
    user.assert_global_short(
        DexMarket::BTC,
        merge_price,
        alice_size + bob_size + mike_size,
    )
    .await;

    // Close positions
    alice.assert_close(DexMarket::BTC, false, 1000.).await;
    user.assert_global_short(DexMarket::BTC, merge_price, bob_size + mike_size)
        .await;

    bob.assert_close(DexMarket::BTC, false, 1000.).await;
    user.assert_global_short(DexMarket::BTC, merge_price, mike_size)
        .await;

    mike.assert_close(DexMarket::BTC, false, 1000.).await;
    user.assert_global_short(DexMarket::BTC, 0., 0.0).await;
}
