#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use anchor_lang::prelude::AccountInfo;
use dex_program::dual_invest::DI;
use solana_program_test::tokio;

use crate::utils::TestResult;
use crate::utils::TEST_DI_FEE_RATE;
use context::DexTestContext;
use utils::helper::*;

#[tokio::test]
async fn test_di_init() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];

    let admin =
        get_keypair_from_file(&mut dtc.context.borrow_mut(), "tests/fixtures/admin.json").await;

    let mut di_account = user.get_account(dtc.dex_info.borrow().di_option).await;

    let owner = admin.pubkey();
    let di_option_account_info: AccountInfo = (&owner, true, &mut di_account).into();
    let di = DI::mount(&di_option_account_info, true).unwrap();

    assert_eq!(di.borrow().meta.admin, admin.pubkey());
    assert_eq!(di.borrow().meta.stopped, false);
    assert_eq!(di.borrow().meta.fee_rate, TEST_DI_FEE_RATE);
    assert_eq!(di.borrow().options.into_iter().count(), 0);
}

#[tokio::test]
async fn test_di_set_fee_rate() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];

    dtc.di_set_fee_rate(&dtc.admin, 50).await.assert_ok();
    user.assert_di_fee_rate(50).await;

    // Other can not set fee rate
    dtc.di_set_fee_rate(&user.user, 50).await.assert_err();
}

#[tokio::test]
async fn test_di_set_admin() {
    let dtc = DexTestContext::new().await;
    let user = &dtc.user_context[0];

    dtc.di_set_admin(&user.user.pubkey()).await;
    user.assert_di_admin(&user.user.pubkey()).await;

    // New admin sets fee rate
    dtc.di_set_fee_rate(&user.user, 60).await.assert_ok();
    user.assert_di_fee_rate(60).await;

    // Other can not set fee rate
    dtc.di_set_fee_rate(&dtc.admin, 60).await.assert_err();
}
