#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use context::DexTestContext;

#[tokio::test]
async fn test_remove_liquidity_withdraw_usdc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_usdc(10_000.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(9_990.0).await;
    alice.assert_usdc_balance(0.).await;
    alice.assert_vlp_total(9_990.0).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_usdc(9_990.0).await;

    alice.assert_vlp(10.0).await;
    alice.assert_usdc_balance(9_980.01).await;
    alice.assert_vlp_total(10.0).await;
}

#[tokio::test]
async fn test_remove_liquidity_withdraw_btc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_btc(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(19_980.0).await;
    alice.assert_btc_balance(0.).await;
    alice.assert_vlp_total(19_980.0).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_btc(19_980.0).await;

    // Alice have all the VLPs which are collected from fees
    alice.assert_vlp(20.0).await;

    //(1.0-1.0*0.1%)-(1.0-1.0*0.1%)*0.1%=0.998001
    alice.assert_btc_balance(0.998001).await;
    alice.assert_vlp_total(20.0).await;
}

#[tokio::test]
async fn test_remove_liquidity_withdraw_eth() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_eth(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(1998.0).await;
    alice.assert_eth_balance(0.).await;
    alice.assert_vlp_total(1998.0).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_eth(1998.0).await;

    alice.assert_vlp(2.0).await;
    //(1.0-1.0*0.1%)-(1.0-1.0*0.1%)*0.1%=0.998001
    alice.assert_eth_balance(0.998001).await;
    alice.assert_vlp_total(2.0).await;
}

#[tokio::test]
async fn test_remove_liquidity_withdraw_sol() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_sol(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(199.8).await;
    alice.assert_vlp_total(199.8).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_sol(199.8).await;

    alice.assert_vlp(0.2).await;
    //(1.0-1.0*0.1%)-(1.0-1.0*0.1%)*0.1%=0.998001

    alice.assert_vlp_total(0.2).await;
}

#[tokio::test]
async fn test_remove_liquidity_withdraw_sol_use_price_feed() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.mock_sol_price(100.0).await;
    alice.feed_sol_price(20.0).await;
    alice.add_liquidity_with_sol(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(19.98).await;
    alice.assert_vlp_total(19.98).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_sol(19.98).await;

    alice.assert_vlp(0.02).await;
    //(1.0-1.0*0.1%)-(1.0-1.0*0.1%)*0.1%=0.998001

    alice.assert_vlp_total(0.02).await;
}
