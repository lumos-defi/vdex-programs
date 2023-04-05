#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{minus_add_fee, DexAsset, INIT_VLP_AMOUNT};
use context::DexTestContext;

#[tokio::test]
async fn test_add_liquidity_with_usdc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_usdc(10_000.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(9_990.0).await;
    alice.assert_usdc_balance(0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 9_990.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_btc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_btc(10.0).await;
    alice
        .assert_liquidity(utils::DexAsset::BTC, minus_add_fee(10.0))
        .await;

    //0.1% add liquidity fee
    alice.assert_vlp(19_9800.0).await;
    alice.assert_btc_balance(0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 19_9800.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_eth() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_eth(1.0).await;
    alice
        .assert_liquidity(utils::DexAsset::ETH, minus_add_fee(1.0))
        .await;

    //0.1% add liquidity fee
    alice.assert_vlp(1_998.0).await;
    alice.assert_eth_balance(0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 1_998.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_sol() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_sol(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(199.8).await;

    alice.assert_vlp_total(INIT_VLP_AMOUNT + 199.8).await;
}

#[tokio::test]
async fn test_add_multiple_liquidity() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];
    let mike = &dtc.user_context[1];
    let joe = &dtc.user_context[2];
    let market = &dtc.user_context[3];

    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Add BTC
    mike.add_liquidity_with_btc(1.0).await; // fee = 0.001 BTC (20 USD/1 SOL)
    market.assert_liquidity(DexAsset::BTC, 0.999).await;
    market.assert_fee(DexAsset::BTC, 0.001).await;

    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;
    market.assert_vlp_rewards(1.0).await; // Fee rewards are collected and distributed among the vlp stakers

    // Add ETH
    joe.add_liquidity_with_eth(10.0).await; // fee = 0.01 ETH (20 USD)

    market.assert_liquidity(DexAsset::ETH, 9.99).await;
    market.assert_fee(DexAsset::ETH, 0.01).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_fee(DexAsset::BTC, 0.).await; // BTC fees are converted to SOL

    market.assert_liquidity(DexAsset::SOL, 998.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;
    market.assert_vlp_rewards(2.0).await;

    // Add USDC
    alice.add_liquidity_with_usdc(20000.).await; // fee = (20 USD/1 SOL)

    market.assert_liquidity(DexAsset::USDC, 19980.).await;
    market.assert_fee(DexAsset::USDC, 20.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_fee(DexAsset::BTC, 0.).await; // BTC fees are converted to SOL

    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_fee(DexAsset::ETH, 0.).await;

    market.assert_liquidity(DexAsset::SOL, 997.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;
    market.assert_vlp_rewards(3.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_usdc_use_feed_price() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    //use price feed
    alice.update_usdc_price(1.1).await;
    alice.add_liquidity_with_usdc(10_000.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(10_989.0).await;
    alice.assert_usdc_balance(0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 10_989.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_btc_use_feed_price() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.update_btc_price(20_000.0).await;
    alice.add_liquidity_with_btc(10.0).await;
    alice
        .assert_liquidity(utils::DexAsset::BTC, minus_add_fee(10.0))
        .await;

    //0.1% add liquidity fee
    alice.assert_vlp(19_9800.0).await;
    alice.assert_btc_balance(0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 19_9800.0).await;
}

#[tokio::test]
async fn test_add_multiple_liquidity_use_price_feed() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];
    let mike = &dtc.user_context[1];
    let joe = &dtc.user_context[2];
    let market = &dtc.user_context[3];

    //feed mock oracle price
    market.feed_btc_price(20001.).await;
    market.feed_eth_price(2001.).await;
    market.feed_sol_price(21.).await;

    //update price feed
    alice.update_usdc_price(1.).await;
    alice.update_btc_price(20000.).await;
    alice.update_eth_price(2000.).await;
    alice.update_sol_price(20.).await;

    // Assert SOL (added when creating dex)
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Add BTC
    mike.add_liquidity_with_btc(1.0).await; // fee = 0.001 BTC (20 USD/1 SOL)
    market.assert_liquidity(DexAsset::BTC, 0.999).await;
    market.assert_fee(DexAsset::BTC, 0.001).await;

    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;
    market.assert_vlp_rewards(1.0).await; // Fee rewards are collected and distributed among the vlp stakers

    // Add ETH
    joe.add_liquidity_with_eth(10.0).await; // fee = 0.01 ETH (20 USD)

    market.assert_liquidity(DexAsset::ETH, 9.99).await;
    market.assert_fee(DexAsset::ETH, 0.01).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_fee(DexAsset::BTC, 0.).await; // BTC fees are converted to SOL

    market.assert_liquidity(DexAsset::SOL, 998.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;
    market.assert_vlp_rewards(2.0).await;

    // Add USDC
    alice.add_liquidity_with_usdc(20000.).await; // fee = (20 USD/1 SOL)

    market.assert_liquidity(DexAsset::USDC, 19980.).await;
    market.assert_fee(DexAsset::USDC, 20.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_fee(DexAsset::BTC, 0.).await; // BTC fees are converted to SOL

    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_fee(DexAsset::ETH, 0.).await;

    market.assert_liquidity(DexAsset::SOL, 997.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;
    market.assert_vlp_rewards(3.0).await;
}
