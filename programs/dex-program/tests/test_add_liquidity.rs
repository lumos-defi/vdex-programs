#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use context::DexTestContext;

#[tokio::test]
async fn test_add_liquidity_with_usdc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_usdc(10_000.0).await;
    let user_vlp_acc = alice.get_user_vlp_token_pubkey().await;

    //0.1% add liquidity fee
    alice.assert_vlp_amount(&user_vlp_acc, 9_990.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_btc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_btc(1.0).await;
    let user_vlp_acc = alice.get_user_vlp_token_pubkey().await;

    //0.1% add liquidity fee
    alice.assert_vlp_amount(&user_vlp_acc, 19_980.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_eth() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_eth(1.0).await;
    let user_vlp_acc = alice.get_user_vlp_token_pubkey().await;

    //0.1% add liquidity fee
    alice.assert_vlp_amount(&user_vlp_acc, 1998.0).await;
}

#[tokio::test]
async fn test_add_liquidity_with_sol() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_sol(1.0).await;
    let user_vlp_acc = alice.get_user_vlp_token_pubkey().await;

    //0.1% add liquidity fee
    alice.assert_vlp_amount(&user_vlp_acc, 199.8).await;
}
