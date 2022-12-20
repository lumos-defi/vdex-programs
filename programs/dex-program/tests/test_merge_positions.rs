#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{collateral_to_size, DexAsset, DexMarket};
use context::DexTestContext;

#[tokio::test]
async fn test_merge_long() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_btc(10.).await;
    user.feed_btc_price(20000.).await;

    // Alice open #1
    alice.mint_btc(0.1).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    let open_fee_1 = 0.002912621;
    let collateral_1 = 0.1 - open_fee_1;
    let size_1 = collateral_1 * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            size_1,
            collateral_1,
            size_1,
            0.,
        )
        .await;

    // Alice open #2
    alice.mint_btc(0.2).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.2, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    let open_fee_2 = 0.005825242;
    let collateral_2 = 0.2 - open_fee_2;
    let size_2 = collateral_2 * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20000.,
            size_1 + size_2,
            collateral_1 + collateral_2,
            size_1 + size_2,
            0.,
        )
        .await;

    user.feed_btc_price(21000.).await;

    // Alice open #3
    alice.mint_btc(0.3).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.3, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    let open_fee_3 = 0.008737864;
    let collateral_3 = 0.3 - open_fee_3;
    let size_3 = collateral_3 * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            true,
            20500.,
            size_1 + size_2 + size_3,
            collateral_1 + collateral_2 + collateral_3,
            size_1 + size_2 + size_3,
            0.,
        )
        .await;
}

#[tokio::test]
async fn test_merge_short() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare liquidity & price
    user.add_liquidity_with_usdc(1000000.).await;
    user.feed_btc_price(20000.).await;

    // Alice open #1
    alice.mint_usdc(2000.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;

    let open_fee_1 = 58.252427;
    let collateral_1 = 2000. - open_fee_1;
    let size_1 = collateral_to_size(collateral_1, 10., 20000., 9);
    let borrow_1 = collateral_1 * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            false,
            20000.,
            size_1,
            collateral_1,
            borrow_1,
            0.,
        )
        .await;

    // Alice open #2
    alice.mint_usdc(4000.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 4000., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;

    let open_fee_2 = 116.504854;
    let collateral_2 = 4000. - open_fee_2;
    let size_2 = collateral_to_size(collateral_2, 10., 20000., 9);
    let borrow_2 = collateral_2 * 10.;

    alice
        .assert_position(
            DexMarket::BTC,
            false,
            20000.,
            size_1 + size_2,
            collateral_1 + collateral_2,
            borrow_1 + borrow_2,
            0.,
        )
        .await;

    user.feed_btc_price(22000.).await;

    // Alice open #3
    alice.mint_usdc(6000.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 6000., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;

    let open_fee_3 = 174.757281;
    let collateral_3 = 6000. - open_fee_3;
    let size_3 = collateral_to_size(collateral_3, 10., 22000., 9);
    let borrow_3 = collateral_3 * 10.;

    let average_price =
        (size_1 * 20000. + size_2 * 20000. + size_3 * 22000.) / (size_1 + size_2 + size_3);

    alice
        .assert_position(
            DexMarket::BTC,
            false,
            average_price,
            size_1 + size_2 + size_3,
            collateral_1 + collateral_2 + collateral_3,
            borrow_1 + borrow_2 + borrow_3,
            0.,
        )
        .await;
}
