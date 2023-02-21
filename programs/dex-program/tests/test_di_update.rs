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
async fn test_di_update() {
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

    let option = admin.di_read_option(100).await;

    admin
        .di_update_option(option.id, 600, false)
        .await
        .assert_ok();

    admin
        .assert_di_option(
            100,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            600,
            now + 1,
            usdc(25000.),
            btc(0.1),
            false,
        )
        .await;

    admin
        .di_update_option(option.id, 800, true)
        .await
        .assert_ok();

    admin
        .assert_di_option(
            100,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            800,
            now + 1,
            usdc(25000.),
            btc(0.1),
            true,
        )
        .await;
}

#[tokio::test]
async fn test_di_update_not_found() {
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

    let option = admin.di_read_option(100).await;

    admin
        .di_update_option(option.id + 10, 600, false)
        .await
        .assert_err();
}
