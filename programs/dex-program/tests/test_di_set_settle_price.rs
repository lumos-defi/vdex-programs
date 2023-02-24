#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::DexAsset;
use crate::utils::TestResult;
use context::DexTestContext;
use utils::constant::*;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[tokio::test]
async fn test_no_authority() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();

    admin
        .di_create_option(
            100,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            500,
            now + 1,
            usdc(25000.),
            btc(0.1),
        )
        .await
        .assert_ok();

    dtc.advance_clock(now + 10).await;

    user.di_set_settle_price(100, usdc(26000.))
        .await
        .assert_err()
}

#[tokio::test]
async fn test_not_expired() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();

    admin
        .di_create_option(
            100,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            500,
            now + 1,
            usdc(25000.),
            btc(0.1),
        )
        .await
        .assert_ok();

    admin
        .di_set_settle_price(100, usdc(26000.))
        .await
        .assert_err()
}

#[tokio::test]
async fn test_ok() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();

    admin
        .di_create_option(
            100,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            500,
            now + 1,
            usdc(25000.),
            btc(0.1),
        )
        .await
        .assert_ok();

    dtc.advance_clock(now + 10).await;

    admin
        .di_set_settle_price(100, usdc(26000.))
        .await
        .assert_ok();

    admin.assert_di_settle_price(100, usdc(26000.)).await;
}
