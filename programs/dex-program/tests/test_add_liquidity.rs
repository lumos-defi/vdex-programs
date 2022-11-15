#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::account::ReadableAccount;
use solana_program_test::tokio;

use context::DexTestContext;
use utils::convert_to_big_number;

#[tokio::test]
async fn test_add_liquidity_with_usdc() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    alice.add_liquidity_with_usdc(10_000.0).await;
}
