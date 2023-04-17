#![cfg(test)]

mod context;
mod utils;

use anchor_client::solana_sdk::signer::Signer;
use solana_program_test::tokio;

use crate::utils::now;
use crate::utils::DexAsset;
use crate::utils::TestResult;
use context::DexTestContext;
use utils::constant::*;

#[tokio::test]
async fn test_di_create_err_no_authority() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();

    // No authority
    user.di_create_option(
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
    .assert_err();
}

#[tokio::test]
async fn test_di_create_err_invalid_expiry() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();

    user.di_create_option(
        100,
        true,
        DexAsset::BTC,
        DexAsset::USDC,
        500,
        now,
        usdc(25000.),
        btc(0.1),
    )
    .await
    .assert_err();

    user.di_create_option(
        100,
        true,
        DexAsset::BTC,
        DexAsset::USDC,
        500,
        now - 100,
        usdc(25000.),
        btc(0.1),
    )
    .await
    .assert_err();
}

#[tokio::test]
async fn test_di_create_dup_id() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();

    // Create ok
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

    admin.assert_di_options_count(1).await;

    // Dup id error
    admin
        .di_create_option(
            100,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            500,
            now + 2,
            usdc(25000.),
            btc(0.1),
        )
        .await
        .assert_err();

    admin.assert_di_options_count(1).await;
}

#[tokio::test]
async fn test_di_create_dup_attributes() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();

    // Create ok
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

    admin.assert_di_options_count(1).await;

    // Dup attributes error
    admin
        .di_create_option(
            101,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            500,
            now + 1,
            usdc(25000.),
            btc(0.1),
        )
        .await
        .assert_err();

    admin.assert_di_options_count(1).await;
}

#[tokio::test]
async fn test_di_create_read_back() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();

    // Create ok
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
        .assert_di_option(
            100,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            500,
            now + 1,
            usdc(25000.),
            btc(0.1),
            false,
        )
        .await;
}

#[tokio::test]
async fn test_di_create_max() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();
    for i in 0..64u64 {
        admin
            .di_create_option(
                100 + i,
                true,
                DexAsset::BTC,
                DexAsset::USDC,
                500,
                now + 1 + (i as i64),
                usdc(25000.),
                btc(0.1),
            )
            .await
            .assert_ok();
    }

    admin
        .di_create_option(
            200,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            500,
            now + 200,
            usdc(25000.),
            btc(0.1),
        )
        .await
        .assert_err();
}

#[tokio::test]
async fn test_di_create_multiple() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let mut now = now();
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

    now += 10;
    dtc.advance_clock(now).await;

    admin
        .di_create_option(
            101,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            500,
            now + 10,
            usdc(25000.),
            btc(0.1),
        )
        .await
        .assert_ok();
}
