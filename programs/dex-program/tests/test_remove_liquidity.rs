#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::INIT_VLP_AMOUNT;
use context::DexTestContext;

#[tokio::test]
async fn test_remove_liquidity_withdraw_usdc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_usdc(10_000.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(9_990.0).await;
    alice.assert_usdc_balance(0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 9_990.0).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_usdc(9_990.0).await;

    alice.assert_vlp(0.0).await;
    alice.assert_usdc_balance(9_980.01).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT).await;
}

#[tokio::test]
async fn test_remove_liquidity_withdraw_btc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_btc(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(19_980.0).await;
    alice.assert_btc_balance(0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 19_980.0).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_btc(19_980.0).await;

    alice.assert_vlp(0.0).await;
    //(1.0-1.0*0.1%)-(1.0-1.0*0.1%)*0.1%=0.998001
    alice.assert_btc_balance(0.998001).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT).await;
}

#[tokio::test]
async fn test_remove_liquidity_withdraw_eth() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_eth(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(1998.0).await;
    alice.assert_eth_balance(0.).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 1998.0).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_eth(1998.0).await;

    alice.assert_vlp(0.0).await;
    //(1.0-1.0*0.1%)-(1.0-1.0*0.1%)*0.1%=0.998001
    alice.assert_eth_balance(0.998001).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 0.0).await;
}

#[tokio::test]
async fn test_remove_liquidity_withdraw_sol() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_sol(1.0).await;

    //0.1% add liquidity fee
    alice.assert_vlp(199.8).await;
    alice.assert_vlp_total(INIT_VLP_AMOUNT + 199.8).await;

    //0.1% remove liquidity fee
    alice.remove_liquidity_withdraw_sol(199.8).await;

    alice.assert_vlp(0.0).await;
    //(1.0-1.0*0.1%)-(1.0-1.0*0.1%)*0.1%=0.998001

    alice.assert_vlp_total(INIT_VLP_AMOUNT).await;
}
