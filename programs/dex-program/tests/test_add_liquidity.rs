#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::INIT_VLP_AMOUNT;
use context::DexTestContext;

#[tokio::test]
async fn test_add_liquidity_with_usdc() {
    let mut dtc = DexTestContext::new().await;
    let alice = &mut dtc.user_context[0];

    alice.add_liquidity_with_usdc(10_000.0).await;
    let user_asset_acc = alice.get_user_usdc_token_pubkey().await;

    //0.1% add liquidity fee
    alice.assert_vlp(9_990.0).await;
    alice.assert_usdc_balance(&user_asset_acc, 0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 9_990.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_btc() {
    let mut dtc = DexTestContext::new().await;
    let alice = &mut dtc.user_context[0];

    alice.add_liquidity_with_btc(1.0).await;
    let user_asset_acc = alice.get_user_btc_token_pubkey().await;

    //0.1% add liquidity fee
    alice.assert_vlp(19_980.0).await;
    alice.assert_btc_balance(&user_asset_acc, 0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 19_980.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_eth() {
    let mut dtc = DexTestContext::new().await;
    let alice = &mut dtc.user_context[0];

    alice.add_liquidity_with_eth(1.0).await;
    let user_asset_acc = alice.get_user_eth_token_pubkey().await;

    //0.1% add liquidity fee
    alice.assert_vlp(1_998.0).await;
    alice.assert_eth_balance(&user_asset_acc, 0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 1_998.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_sol() {
    let mut dtc = DexTestContext::new().await;
    let alice = &mut dtc.user_context[0];

    alice.add_liquidity_with_sol(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(199.8).await;

    alice.assert_vlp_total(INIT_VLP_AMOUNT + 199.8).await;
}
