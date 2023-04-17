#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::now;
use crate::utils::TestResult;
use context::DexTestContext;
use utils::constant::*;

#[tokio::test]
async fn test_anyone_can_remove_all_settled_option() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 0.1 btc
    let mut now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    // Open size: 0.1 btc
    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_ok();
    user.assert_btc_balance(0.).await;

    let options = user.di_collect_my_options(100).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(22000.))
        .await
        .assert_ok();

    // Settle user's option
    admin
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
        .await
        .assert_ok();

    admin.assert_di_settle_size(100, btc(0.1)).await;

    anyone.di_remove(100, false).await.assert_ok();
}

#[tokio::test]
async fn test_can_not_remove_unsettled_option_without_force() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 0.1 btc
    let mut now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    // Open size: 0.1 btc
    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_ok();
    user.assert_btc_balance(0.).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(22000.))
        .await
        .assert_ok();

    anyone.di_remove(100, false).await.assert_err();
}

#[tokio::test]
async fn test_admin_can_force_to_remove_unsettled_option() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 0.1 btc
    let mut now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    // Open size: 0.1 btc
    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_ok();
    user.assert_btc_balance(0.).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(22000.))
        .await
        .assert_ok();

    admin.di_remove(100, true).await.assert_ok();
}
