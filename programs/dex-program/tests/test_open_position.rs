#![cfg(test)]

mod context;
mod utils;

use solana_program_test::tokio;

use crate::utils::INIT_VLP_AMOUNT;
use context::DexTestContext;

#[tokio::test]
async fn test_open_position_basic() {
    let mut dtc = DexTestContext::new().await;
    let alice = &mut dtc.user_context[0];
}
