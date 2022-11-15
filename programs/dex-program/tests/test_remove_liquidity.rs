#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::{
    INIT_WALLET_BTC_ASSET_AMOUNT, INIT_WALLET_ETH_ASSET_AMOUNT, INIT_WALLET_SOL_ASSET_AMOUNT,
    INIT_WALLET_USDC_ASSET_AMOUNT,
};
use context::DexTestContext;

#[tokio::test]
async fn test_remove_liquidity_with_usdc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_usdc(10_000.0).await;
    let user_vlp_acc = alice.get_user_vlp_token_pubkey().await;
    let user_asset_acc = alice.get_user_usdc_token_pubkey().await;

    //0.1% add liquidity fee
    alice.assert_vlp_amount(&user_vlp_acc, 9_990.0).await;
    alice
        .assert_usdc_amount(&user_asset_acc, INIT_WALLET_USDC_ASSET_AMOUNT - 10_000.0)
        .await
}
