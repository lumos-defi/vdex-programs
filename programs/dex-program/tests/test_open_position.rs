#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use context::DexTestContext;

#[tokio::test]
async fn test_btc_open_long() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];
    let alice = &dtc.user_context[1];

    // Prepare BTC liquidity
    user.add_liquidity_with_btc(10.).await;
    alice.add_liquidity_with_btc(1.).await;
}
