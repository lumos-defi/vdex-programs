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
async fn test_btc_call_not_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 0.1 btc
    let mut now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    // Open size: 0.1 btc
    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_ok();
    user.assert_btc_balance(0.).await;

    // Check borrowed btc: size * premium_rate
    let borrowed_btc = 0.1 * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::BTC, 1. - borrowed_btc)
        .await;

    // Check borrowed usdc: size * strike_price * ( 1 + premium_rate )
    let borrowed_usdc = 0.1 * 25000. * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let options = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_call(
        options[0].created,
        500,
        btc(0.1),
        btc(borrowed_btc),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, btc(0.1)).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(22000.))
        .await
        .assert_ok();

    // Close user's btc account
    user.close_mint_account(DexAsset::BTC).await;

    // Settle user's option
    admin
        .di_settle_with_invalid_user_mint_acc(
            &user.user.pubkey(),
            options[0].created,
            false,
            usdc(0.),
        )
        .await
        .assert_ok();

    admin.assert_di_settle_size(100, btc(0.1)).await;

    // Check liquidity
    market
        .assert_liquidity(DexAsset::BTC, 1. - borrowed_btc)
        .await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;

    let fee = (0.1 + borrowed_btc) * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::BTC, fee).await;

    user.assert_di_option_settled(100, false, 0.1 + borrowed_btc - fee)
        .await;

    user.create_mint_account(DexAsset::BTC).await;

    let option = user.di_collect_my_options(100).await;
    user.di_withdraw_settled(option[0].created)
        .await
        .assert_ok();
    user.assert_btc_balance(0.1 + borrowed_btc - fee).await;

    let option = user.di_collect_my_options(100).await;
    assert_eq!(option.len(), 0);
}

#[tokio::test]
async fn test_btc_call_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;

    market.assert_fee(DexAsset::USDC, 20.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 0.1 btc
    let mut now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    // Open size: 0.1 btc
    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_ok();
    user.assert_btc_balance(0.).await;

    // Check borrowed btc: size * premium_rate
    let borrowed_btc = 0.1 * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::BTC, 1. - borrowed_btc)
        .await;

    // Check borrowed usdc: size * strike_price * ( 1 + premium_rate )
    let borrowed_usdc = 0.1 * 25000. * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let options = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_call(
        options[0].created,
        500,
        btc(0.1),
        btc(borrowed_btc),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, btc(0.1)).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(26000.))
        .await
        .assert_ok();

    // Settle user's option
    admin
        .di_settle_with_invalid_user_mint_acc(
            &user.user.pubkey(),
            options[0].created,
            false,
            usdc(0.),
        )
        .await
        .assert_ok();

    admin.assert_di_settle_size(100, btc(0.1)).await;

    // Check liquidity
    market.assert_liquidity(DexAsset::BTC, 1. + 0.1).await;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let fee = borrowed_usdc * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::USDC, 20. + fee).await;
    user.assert_btc_balance(0.).await;

    user.assert_di_option_settled(100, true, borrowed_usdc - fee)
        .await;

    user.create_mint_account(DexAsset::USDC).await;

    let option = user.di_collect_my_options(100).await;
    user.di_withdraw_settled(option[0].created)
        .await
        .assert_ok();

    user.assert_usdc_balance(borrowed_usdc - fee).await;

    let option = user.di_collect_my_options(100).await;
    assert_eq!(option.len(), 0);
}

#[tokio::test]
async fn test_btc_put_not_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    market.add_liquidity_with_btc(1.).await;
    market.add_liquidity_with_eth(10.).await;
    market.add_liquidity_with_usdc(20000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;

    market.assert_fee(DexAsset::USDC, 20.).await;

    // Create put option: premium = 5%, strike = 18000, minimum size = 100 usdc
    let mut now = now();
    admin
        .di_create_btc_put(100, 500, now + 5, 18000., 100.)
        .await;

    // Open size: 180 usdc
    user.mint_usdc(180.).await;
    user.di_buy(100, 500, usdc(180.)).await.assert_ok();
    user.assert_usdc_balance(0.).await;

    // Check borrowed usdc: size * premium_rate
    let borrowed_usdc = 180. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    // Check borrowed btc: ( usdc_size / strike_price ) * ( 1 + premium_rate )
    let borrowed_btc = (180. / 18000.) * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::BTC, 1. - borrowed_btc)
        .await;

    let options = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_put(
        options[0].created,
        500,
        usdc(180.),
        btc(borrowed_btc),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, usdc(180.)).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(19000.))
        .await
        .assert_ok();

    // Close user's btc account
    user.close_mint_account(DexAsset::USDC).await;

    // Settle user's option
    admin
        .di_settle_with_invalid_user_mint_acc(
            &user.user.pubkey(),
            options[0].created,
            false,
            usdc(0.),
        )
        .await
        .assert_ok();

    admin.assert_di_settle_size(100, usdc(180.)).await;

    // Check liquidity
    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let fee = (180. + borrowed_usdc) * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::USDC, 20. + fee).await;

    user.assert_di_option_settled(100, false, 180. + borrowed_usdc - fee)
        .await;

    user.create_mint_account(DexAsset::USDC).await;

    let option = user.di_collect_my_options(100).await;
    user.di_withdraw_settled(option[0].created)
        .await
        .assert_ok();

    user.assert_usdc_balance(180. + borrowed_usdc - fee).await;

    let option = user.di_collect_my_options(100).await;
    assert_eq!(option.len(), 0);
}

#[tokio::test]
async fn test_btc_put_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    market.add_liquidity_with_btc(1.).await;
    market.add_liquidity_with_eth(10.).await;
    market.add_liquidity_with_usdc(20000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;

    market.assert_fee(DexAsset::USDC, 20.).await;

    // Create put option: premium = 5%, strike = 18000, minimum size = 100 usdc
    let mut now = now();
    admin
        .di_create_btc_put(100, 500, now + 5, 18000., 100.)
        .await;

    // Open size: 180 usdc
    user.mint_usdc(180.).await;
    user.di_buy(100, 500, usdc(180.)).await.assert_ok();
    user.assert_usdc_balance(0.).await;

    // Check borrowed usdc: size * premium_rate
    let borrowed_usdc = 180. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    // Check borrowed btc: ( usdc_size / strike_price ) * ( 1 + premium_rate )
    let borrowed_btc = (180. / 18000.) * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::BTC, 1. - borrowed_btc)
        .await;

    let options = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_put(
        options[0].created,
        500,
        usdc(180.),
        btc(borrowed_btc),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, usdc(180.)).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(17000.))
        .await
        .assert_ok();

    // Settle user's option
    admin
        .di_settle_with_invalid_user_mint_acc(
            &user.user.pubkey(),
            options[0].created,
            false,
            usdc(0.),
        )
        .await
        .assert_ok();

    admin.assert_di_settle_size(100, usdc(180.)).await;

    // Check liquidity
    market
        .assert_liquidity(DexAsset::BTC, 1. - borrowed_btc)
        .await;
    market.assert_liquidity(DexAsset::USDC, 19980. + 180.).await;

    let fee = borrowed_btc * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::BTC, fee).await;

    user.assert_di_option_settled(100, true, borrowed_btc - fee)
        .await;

    user.create_mint_account(DexAsset::BTC).await;

    let option = user.di_collect_my_options(100).await;
    user.di_withdraw_settled(option[0].created)
        .await
        .assert_ok();

    user.assert_btc_balance(borrowed_btc - fee).await;

    user.assert_usdc_balance(0.).await;

    let option = user.di_collect_my_options(100).await;
    assert_eq!(option.len(), 0);
}

#[tokio::test]
async fn test_sol_call_not_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;
    market.assert_liquidity(DexAsset::SOL, 997.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 10. sol
    let mut now = now();
    admin.di_create_sol_call(100, 500, now + 5, 25., 10.).await;

    // Open size: 10 sol
    user.di_buy(100, 500, sol(10.)).await.assert_ok();

    // Check borrowed sol: size * premium_rate
    let borrowed_sol = 10. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::SOL, 997. - borrowed_sol)
        .await;

    // Check borrowed usdc: size * strike_price * ( 1 + premium_rate )
    let borrowed_usdc = 10. * 25. * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let options = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_call(
        options[0].created,
        500,
        sol(10.),
        sol(borrowed_sol),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, sol(10.)).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin.di_set_settle_price(100, usdc(22.)).await.assert_ok();

    // Settle
    anyone
        .di_settle_with_invalid_user_mint_acc(
            &user.user.pubkey(),
            options[0].created,
            false,
            usdc(0.),
        )
        .await
        .assert_ok();

    // Check liquidity
    market
        .assert_liquidity(DexAsset::SOL, 997. - borrowed_sol)
        .await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;

    let fee = (10. + borrowed_sol) * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::SOL, fee).await;

    user.assert_di_option_settled(100, false, 10. + borrowed_sol - fee)
        .await;

    let user_sol_balance_before = user.balance().await;
    user.di_withdraw_settled(options[0].created)
        .await
        .assert_ok();
    let user_sol_balance_after = user.balance().await;

    assert!(user_sol_balance_after - user_sol_balance_before < sol(10. + borrowed_sol - fee));

    let option = user.di_collect_my_options(100).await;
    assert_eq!(option.len(), 0);
}

#[tokio::test]
async fn test_sol_call_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;
    market.assert_liquidity(DexAsset::SOL, 997.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 10. sol
    let mut now = now();
    admin.di_create_sol_call(100, 500, now + 5, 25., 10.).await;

    // Open size: 10 sol
    user.di_buy(100, 500, sol(10.)).await.assert_ok();

    // Check borrowed sol: size * premium_rate
    let borrowed_sol = 10. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::SOL, 997. - borrowed_sol)
        .await;

    // Check borrowed usdc: size * strike_price * ( 1 + premium_rate )
    let borrowed_usdc = 10. * 25. * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let options = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_call(
        options[0].created,
        500,
        sol(10.),
        sol(borrowed_sol),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, sol(10.)).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin.di_set_settle_price(100, usdc(26.)).await.assert_ok();

    // Settle
    anyone
        .di_settle_with_invalid_user_mint_acc(
            &user.user.pubkey(),
            options[0].created,
            false,
            usdc(0.),
        )
        .await
        .assert_ok();

    // Check liquidity
    market.assert_liquidity(DexAsset::SOL, 997. + 10.).await;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let fee = borrowed_usdc * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::USDC, 20. + fee).await;
    market.assert_fee(DexAsset::SOL, 0.).await;

    user.di_withdraw_settled(options[0].created)
        .await
        .assert_ok();

    user.assert_usdc_balance(borrowed_usdc - fee).await;

    let option = user.di_collect_my_options(100).await;
    assert_eq!(option.len(), 0);
}

#[tokio::test]
async fn test_sol_put_not_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;
    market.assert_liquidity(DexAsset::SOL, 997.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;

    // Create put option: premium = 5%, strike = 15, minimum size = 100. usdc
    let mut now = now();
    admin.di_create_sol_put(100, 500, now + 5, 15., 100.).await;

    // Open size: 150 usdc
    user.mint_usdc(150.).await;
    user.di_buy(100, 500, usdc(150.)).await.assert_ok();

    // Check borrowed sol: ( usdc_size / strike_price ) * ( 1 + premium_rate )
    let borrowed_sol = (150. / 15.) * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::SOL, 997. - borrowed_sol)
        .await;

    // Check borrowed usdc: size * premium_rate
    let borrowed_usdc = 150. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let options = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_put(
        options[0].created,
        500,
        usdc(150.),
        sol(borrowed_sol),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, usdc(150.)).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin.di_set_settle_price(100, usdc(26.)).await.assert_ok();

    user.close_mint_account(DexAsset::USDC).await;
    // Settle
    anyone
        .di_settle_with_invalid_user_mint_acc(
            &user.user.pubkey(),
            options[0].created,
            false,
            usdc(0.),
        )
        .await
        .assert_ok();

    // Check liquidity
    market.assert_liquidity(DexAsset::SOL, 997.).await;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let fee = (150. + borrowed_usdc) * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::USDC, 20. + fee).await;
    market.assert_fee(DexAsset::SOL, 0.).await;

    user.di_withdraw_settled(options[0].created)
        .await
        .assert_ok();

    user.assert_usdc_balance(150. + borrowed_usdc - fee).await;

    let option = user.di_collect_my_options(100).await;
    assert_eq!(option.len(), 0);
}

#[tokio::test]
async fn test_sol_put_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.feed_btc_price(20000.).await;
    market.feed_eth_price(2000.).await;
    market.feed_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;
    market.assert_liquidity(DexAsset::SOL, 997.).await;
    market.assert_fee(DexAsset::SOL, 0.).await;

    // Create put option: premium = 5%, strike = 15, minimum size = 100. usdc
    let mut now = now();
    admin.di_create_sol_put(100, 500, now + 5, 15., 100.).await;

    // Open size: 150 usdc
    user.mint_usdc(150.).await;
    user.di_buy(100, 500, usdc(150.)).await.assert_ok();

    // Check borrowed sol: ( usdc_size / strike_price ) * ( 1 + premium_rate )
    let borrowed_sol = (150. / 15.) * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::SOL, 997. - borrowed_sol)
        .await;

    // Check borrowed usdc: size * premium_rate
    let borrowed_usdc = 150. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - borrowed_usdc)
        .await;

    let options = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_put(
        options[0].created,
        500,
        usdc(150.),
        sol(borrowed_sol),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, usdc(150.)).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin.di_set_settle_price(100, usdc(14.)).await.assert_ok();

    // Settle
    anyone
        .di_settle_with_invalid_user_mint_acc(
            &user.user.pubkey(),
            options[0].created,
            false,
            usdc(0.),
        )
        .await
        .assert_ok();

    // Check liquidity
    market
        .assert_liquidity(DexAsset::SOL, 997. - borrowed_sol)
        .await;
    market.assert_liquidity(DexAsset::USDC, 19980. + 150.).await;

    let fee = borrowed_sol * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::SOL, fee).await;
    market.assert_fee(DexAsset::USDC, 20.).await;

    let user_sol_balance_before = user.balance().await;
    user.di_withdraw_settled(options[0].created)
        .await
        .assert_ok();
    let user_sol_balance_after = user.balance().await;

    assert!(user_sol_balance_after - user_sol_balance_before < sol(borrowed_sol - fee));

    let option = user.di_collect_my_options(100).await;
    assert_eq!(option.len(), 0);
}
