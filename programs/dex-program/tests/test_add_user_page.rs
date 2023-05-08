#![cfg(test)]

mod context;
mod utils;

use crate::utils::TestResult;
use context::DexTestContext;
use dex_program::utils::MAX_USER_LIST_REMAINING_PAGES_COUNT;
use solana_program_test::tokio;

#[tokio::test]
async fn test_add_dex_user_page() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];

    for i in 0..MAX_USER_LIST_REMAINING_PAGES_COUNT {
        admin.add_user_page().await.assert_ok();
        admin.assert_user_page_count(i as u8 + 1).await;
    }
}
