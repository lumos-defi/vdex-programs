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
async fn test_btc_call_not_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    // Settle user's option
    admin
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
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
    user.assert_btc_balance(0.1 + borrowed_btc - fee).await;
}

#[tokio::test]
async fn test_btc_call_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
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

    user.assert_usdc_balance(borrowed_usdc - fee).await;
}

#[tokio::test]
async fn test_btc_put_not_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    // Settle user's option
    admin
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
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

    user.assert_usdc_balance(180. + borrowed_usdc - fee).await;
}

#[tokio::test]
async fn test_btc_put_exercised() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
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
    user.assert_btc_balance(borrowed_btc - fee).await;

    user.assert_usdc_balance(0.).await;
}

#[tokio::test]
async fn test_anyone_can_settle_without_forcing_settle_price() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    // Anyone can settle user's option if not providing settle price.
    anyone
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
        .await
        .assert_ok();
}

#[tokio::test]
async fn test_only_admin_can_force_settle_price() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    // Only admin can settle user's option if providing settle price.
    anyone
        .di_settle(&user.user.pubkey(), options[0].created, true, usdc(17500.))
        .await
        .assert_err();

    admin
        .di_settle(&user.user.pubkey(), options[0].created, true, usdc(17500.))
        .await
        .assert_ok();
}

#[tokio::test]
async fn test_settle_user_multiple_options() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    dtc.advance_clock(now + 1).await;

    // Another position
    user.mint_usdc(360.).await;
    user.di_buy(100, 500, usdc(360.)).await.assert_ok();
    user.assert_usdc_balance(0.).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    let options = user.di_collect_my_options(100).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(17000.))
        .await
        .assert_ok();

    // Settle the first one
    anyone
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
        .await
        .assert_ok();

    admin.assert_di_settle_size(100, usdc(180.)).await;

    // Settle the second one
    anyone
        .di_settle(&user.user.pubkey(), options[1].created, false, usdc(0.))
        .await
        .assert_ok();

    admin.assert_di_settle_size(100, usdc(540.)).await;
    user.assert_di_user_option_count(100, 0).await
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
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;
    market.add_liquidity_with_sol(1000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 20000.).await;
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 10. sol
    let mut now = now();
    admin.di_create_sol_call(100, 500, now + 5, 25., 10.).await;

    // Open size: 10 sol
    user.di_buy(100, 500, sol(10.)).await.assert_ok();

    // Check borrowed sol: size * premium_rate
    let borrowed_sol = 10. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::SOL, 999. - borrowed_sol)
        .await;

    // Check borrowed usdc: size * strike_price * ( 1 + premium_rate )
    let borrowed_usdc = 10. * 25. * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::USDC, 20000. - borrowed_usdc)
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

    let user_sol_balance_before = user.balance().await;
    // Settle
    anyone
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
        .await
        .assert_ok();

    // Check liquidity
    market
        .assert_liquidity(DexAsset::SOL, 999. - borrowed_sol)
        .await;
    market.assert_liquidity(DexAsset::USDC, 20000.).await;

    let fee = (10. + borrowed_sol) * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::SOL, 1.0 + fee).await;

    let user_sol_balance_after = user.balance().await;

    assert_eq!(
        user_sol_balance_after - user_sol_balance_before,
        sol(10. + borrowed_sol - fee)
    )
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
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;
    market.add_liquidity_with_sol(1000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 20000.).await;
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 10. sol
    let mut now = now();
    admin.di_create_sol_call(100, 500, now + 5, 25., 10.).await;

    // Open size: 10 sol
    user.di_buy(100, 500, sol(10.)).await.assert_ok();

    // Check borrowed sol: size * premium_rate
    let borrowed_sol = 10. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::SOL, 999. - borrowed_sol)
        .await;

    // Check borrowed usdc: size * strike_price * ( 1 + premium_rate )
    let borrowed_usdc = 10. * 25. * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::USDC, 20000. - borrowed_usdc)
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

    let user_sol_balance_before = user.balance().await;

    // Settle
    anyone
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
        .await
        .assert_ok();

    // Check liquidity
    market.assert_liquidity(DexAsset::SOL, 999. + 10.).await;
    market
        .assert_liquidity(DexAsset::USDC, 20000. - borrowed_usdc)
        .await;

    let fee = borrowed_usdc * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::USDC, fee).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    let user_sol_balance_after = user.balance().await;
    assert_eq!(user_sol_balance_after - user_sol_balance_before, sol(0.));

    user.assert_usdc_balance(borrowed_usdc - fee).await;
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
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;
    market.add_liquidity_with_sol(1000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 20000.).await;
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Create put option: premium = 5%, strike = 15, minimum size = 100. usdc
    let mut now = now();
    admin.di_create_sol_put(100, 500, now + 5, 15., 100.).await;

    // Open size: 150 usdc
    user.mint_usdc(150.).await;
    user.di_buy(100, 500, usdc(150.)).await.assert_ok();

    // Check borrowed sol: ( usdc_size / strike_price ) * ( 1 + premium_rate )
    let borrowed_sol = (150. / 15.) * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::SOL, 999. - borrowed_sol)
        .await;

    // Check borrowed usdc: size * premium_rate
    let borrowed_usdc = 150. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::USDC, 20000. - borrowed_usdc)
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

    let user_sol_balance_before = user.balance().await;

    // Settle
    anyone
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
        .await
        .assert_ok();

    // Check liquidity
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market
        .assert_liquidity(DexAsset::USDC, 20000. - borrowed_usdc)
        .await;

    let fee = (150. + borrowed_usdc) * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::USDC, fee).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    let user_sol_balance_after = user.balance().await;
    assert_eq!(user_sol_balance_after - user_sol_balance_before, sol(0.));

    user.assert_usdc_balance(150. + borrowed_usdc - fee).await;
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
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;
    market.add_liquidity_with_sol(1000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 20000.).await;
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Create put option: premium = 5%, strike = 15, minimum size = 100. usdc
    let mut now = now();
    admin.di_create_sol_put(100, 500, now + 5, 15., 100.).await;

    // Open size: 150 usdc
    user.mint_usdc(150.).await;
    user.di_buy(100, 500, usdc(150.)).await.assert_ok();

    // Check borrowed sol: ( usdc_size / strike_price ) * ( 1 + premium_rate )
    let borrowed_sol = (150. / 15.) * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::SOL, 999. - borrowed_sol)
        .await;

    // Check borrowed usdc: size * premium_rate
    let borrowed_usdc = 150. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::USDC, 20000. - borrowed_usdc)
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

    let user_sol_balance_before = user.balance().await;

    // Settle
    anyone
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
        .await
        .assert_ok();

    // Check liquidity
    market
        .assert_liquidity(DexAsset::SOL, 999. - borrowed_sol)
        .await;
    market.assert_liquidity(DexAsset::USDC, 20000. + 150.).await;

    let fee = borrowed_sol * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::SOL, 1.0 + fee).await;
    market.assert_fee(DexAsset::USDC, 0.).await;

    let user_sol_balance_after = user.balance().await;
    assert_eq!(
        user_sol_balance_after - user_sol_balance_before,
        sol(borrowed_sol - fee)
    );

    user.assert_usdc_balance(0.).await;
}

#[tokio::test]
async fn test_can_not_settle_removed_option() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    let anyone = &dtc.user_context[3];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

    market.add_liquidity_with_btc(1.0).await;
    market.add_liquidity_with_eth(10.0).await;
    market.add_liquidity_with_usdc(20000.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 0.1 btc
    let mut now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    // Open size: 0.1 btc
    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_ok();
    user.assert_btc_balance(0.).await;

    // Mock expiration
    now += 10;
    dtc.advance_clock(now).await;

    // Set settle price
    admin
        .di_set_settle_price(100, usdc(22000.))
        .await
        .assert_ok();

    admin.di_remove(100, true).await.assert_ok();

    let options = user.di_collect_my_options(100).await;
    // Fail to settle
    anyone
        .di_settle(&user.user.pubkey(), options[0].created, false, usdc(0.))
        .await
        .assert_err();
}

#[tokio::test]
async fn test_admin_force_to_settle_removed_option() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    // Prepare liquidity
    market.mock_btc_price(20000.).await;
    market.mock_eth_price(2000.).await;
    market.mock_sol_price(20.).await;

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

    admin.di_remove(100, true).await.assert_ok();

    // Admin force to settle user's option
    admin
        .di_settle(&user.user.pubkey(), options[0].created, true, usdc(22000.))
        .await
        .assert_ok();

    // Check liquidity
    market
        .assert_liquidity(DexAsset::BTC, 1. - borrowed_btc)
        .await;
    market.assert_liquidity(DexAsset::USDC, 19980.).await;

    let fee = (0.1 + borrowed_btc) * TEST_DI_FEE_RATE as f64 / 10000.;
    market.assert_fee(DexAsset::BTC, fee).await;
    user.assert_btc_balance(0.1 + borrowed_btc - fee).await;
}
