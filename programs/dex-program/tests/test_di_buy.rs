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
async fn test_not_exist() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    user.mint_btc(0.1).await;
    user.di_buy_direct(100, DexAsset::BTC, DexAsset::USDC, true, 500, btc(0.1))
        .await
        .assert_err();
    user.assert_btc_balance(0.1).await;
}

#[tokio::test]
async fn test_option_expired() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    dtc.advance_clock(now + 10).await;

    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_err();
    user.assert_btc_balance(0.1).await;
}

#[tokio::test]
async fn test_invalid_premium() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    user.mint_btc(0.1).await;
    user.di_buy(100, 600, btc(0.1)).await.assert_err();
    user.assert_btc_balance(0.1).await;
}

#[tokio::test]
async fn test_already_stopped() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    let now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    admin.di_update_option(100, 550, true).await.assert_ok();

    user.mint_btc(0.1).await;
    user.di_buy(100, 550, btc(0.1)).await.assert_err();
    user.assert_btc_balance(0.1).await;
}

#[tokio::test]
async fn test_base_price_gt_call_strike() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    market.mock_btc_price(20000.).await;

    let now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    market.mock_btc_price(26000.).await;

    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_err();
    user.assert_btc_balance(0.1).await;
}

#[tokio::test]
async fn test_base_price_lt_put_strike() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    market.mock_btc_price(20000.).await;

    let now = now();
    admin
        .di_create_btc_put(100, 500, now + 5, 18000., 100.)
        .await;

    market.mock_btc_price(17000.).await;

    user.mint_usdc(200.).await;
    user.di_buy(100, 500, usdc(200.)).await.assert_err();
    user.assert_usdc_balance(200.).await;
}

#[tokio::test]
async fn test_violate_call_minimum_size() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    market.mock_btc_price(20000.).await;

    let now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    user.mint_btc(0.09).await;
    user.di_buy(100, 500, btc(0.09)).await.assert_err();
    user.assert_btc_balance(0.09).await;
}

#[tokio::test]
async fn test_violate_put_minimum_size() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    market.mock_btc_price(20000.).await;

    let now = now();
    admin
        .di_create_btc_put(100, 500, now + 5, 18000., 100.)
        .await;

    user.mint_usdc(99.).await;
    user.di_buy(100, 500, usdc(99.)).await.assert_err();
    user.assert_usdc_balance(99.).await;
}

#[tokio::test]
async fn test_insufficient_liquidity_for_call() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    market.mock_btc_price(20000.).await;

    let now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    user.mint_btc(0.1).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_err();
    user.assert_btc_balance(0.1).await;
}

#[tokio::test]
async fn test_insufficient_liquidity_for_put() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let user = &dtc.user_context[1];
    let market = &dtc.user_context[2];
    dtc.di_set_admin(&admin.user.pubkey()).await;

    market.mock_btc_price(20000.).await;

    let now = now();
    admin
        .di_create_btc_put(100, 500, now + 5, 18000., 100.)
        .await;

    user.mint_usdc(100.).await;
    user.di_buy(100, 500, usdc(100.)).await.assert_err();
    user.assert_usdc_balance(100.).await;
}

#[tokio::test]
async fn test_btc_call_success() {
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
    let now = now();
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

    let option = user.di_collect_my_options(100).await;
    // Check user state
    user.assert_di_user_call(
        option[0].created,
        500,
        btc(0.1),
        btc(borrowed_btc),
        usdc(borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, btc(0.1)).await;
}

#[tokio::test]
async fn test_btc_put_success() {
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

    // Create put option: premium = 5%, strike = 18000, minimum size = 100 usdc
    let now = now();
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
}

#[tokio::test]
async fn test_btc_call_multiple_users() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];
    let market = &dtc.user_context[3];
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
    let now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    // Alice open size: 0.1 btc
    alice.mint_btc(0.1).await;
    alice.di_buy(100, 500, btc(0.1)).await.assert_ok();
    alice.assert_btc_balance(0.).await;

    // Check borrowed btc: size * premium_rate
    let alice_borrowed_btc = 0.1 * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::BTC, 1. - alice_borrowed_btc)
        .await;

    // Check borrowed usdc: size * strike_price * ( 1 + premium_rate )
    let alice_borrowed_usdc = 0.1 * 25000. * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::USDC, 19980. - alice_borrowed_usdc)
        .await;

    let options = alice.di_collect_my_options(100).await;
    // Check user state
    alice
        .assert_di_user_call(
            options[0].created,
            500,
            btc(0.1),
            btc(alice_borrowed_btc),
            usdc(alice_borrowed_usdc),
        )
        .await;

    // Check option volume
    admin.assert_di_option_volume(100, btc(0.1)).await;

    // Bob open size: 0.1 btc
    bob.mint_btc(0.1).await;
    bob.di_buy(100, 500, btc(0.1)).await.assert_ok();
    bob.assert_btc_balance(0.).await;

    // Check borrowed btc: size * premium_rate
    let bob_borrowed_btc = 0.1 * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::BTC, 1. - bob_borrowed_btc - alice_borrowed_btc)
        .await;

    // Check borrowed usdc: size * strike_price * ( 1 + premium_rate )
    let bob_borrowed_usdc = 0.1 * 25000. * (1. + 500. / 10000.);
    market
        .assert_liquidity(
            DexAsset::USDC,
            19980. - bob_borrowed_usdc - alice_borrowed_usdc,
        )
        .await;

    let options = bob.di_collect_my_options(100).await;
    // Check user state
    bob.assert_di_user_call(
        options[0].created,
        500,
        btc(0.1),
        btc(bob_borrowed_btc),
        usdc(bob_borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, btc(0.2)).await;
}

#[tokio::test]
async fn test_btc_put_multiple_users() {
    let dtc = DexTestContext::new().await;
    let admin = &dtc.user_context[0];
    let alice = &dtc.user_context[1];
    let bob = &dtc.user_context[2];
    let market = &dtc.user_context[3];
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

    // Create put option: premium = 5%, strike = 18000, minimum size = 100 usdc
    let now = now();
    admin
        .di_create_btc_put(100, 500, now + 5, 18000., 100.)
        .await;

    // Alice open size: 180 usdc
    alice.mint_usdc(180.).await;
    alice.di_buy(100, 500, usdc(180.)).await.assert_ok();
    alice.assert_usdc_balance(0.).await;

    // Check borrowed usdc: size * premium_rate
    let alice_borrowed_usdc = 180. * 500. / 10000.;
    market
        .assert_liquidity(DexAsset::USDC, 19980. - alice_borrowed_usdc)
        .await;

    // Check borrowed btc: ( usdc_size / strike_price ) * ( 1 + premium_rate )
    let alice_borrowed_btc = (180. / 18000.) * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::BTC, 1. - alice_borrowed_btc)
        .await;

    let options = alice.di_collect_my_options(100).await;
    // Check user state
    alice
        .assert_di_user_put(
            options[0].created,
            500,
            usdc(180.),
            btc(alice_borrowed_btc),
            usdc(alice_borrowed_usdc),
        )
        .await;

    // Check option volume
    admin.assert_di_option_volume(100, usdc(180.)).await;

    // Bob open size: 180 usdc
    bob.mint_usdc(180.).await;
    bob.di_buy(100, 500, usdc(180.)).await.assert_ok();
    bob.assert_usdc_balance(0.).await;

    // Check borrowed usdc: size * premium_rate
    let bob_borrowed_usdc = 180. * 500. / 10000.;
    market
        .assert_liquidity(
            DexAsset::USDC,
            19980. - bob_borrowed_usdc - alice_borrowed_usdc,
        )
        .await;

    // Check borrowed btc: ( usdc_size / strike_price ) * ( 1 + premium_rate )
    let bob_borrowed_btc = (180. / 18000.) * (1. + 500. / 10000.);
    market
        .assert_liquidity(DexAsset::BTC, 1. - bob_borrowed_btc - alice_borrowed_btc)
        .await;

    let options = bob.di_collect_my_options(100).await;
    // Check user state
    bob.assert_di_user_put(
        options[0].created,
        500,
        usdc(180.),
        btc(bob_borrowed_btc),
        usdc(bob_borrowed_usdc),
    )
    .await;

    // Check option volume
    admin.assert_di_option_volume(100, usdc(360.)).await;
}

#[tokio::test]
async fn test_single_user_multiple_btc_call() {
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
    let now = now();
    admin
        .di_create_btc_call(100, 500, now + 5, 25000., 0.1)
        .await;

    // Open 0.1 btc
    user.mint_btc(0.3).await;
    user.di_buy(100, 500, btc(0.1)).await.assert_ok();
    user.assert_btc_balance(0.2).await;

    admin.di_update_option(100, 1000, false).await.assert_ok();

    dtc.advance_clock(now + 1).await;

    // Another 0.1 btc
    user.di_buy(100, 1000, btc(0.2)).await.assert_ok();
    user.assert_btc_balance(0.).await;

    // Collect all options
    let options = user.di_collect_my_options(100).await;
    assert_eq!(options.len(), 2);

    assert_eq!(options[0].size, btc(0.1));
    assert_eq!(options[1].size, btc(0.2));

    assert_eq!(options[0].strike_price, usdc(25000.));
    assert_eq!(options[1].strike_price, usdc(25000.));

    assert_eq!(options[0].premium_rate, 500);
    assert_eq!(options[1].premium_rate, 1000);

    assert_eq!(options[0].expiry_date, now + 5);
    assert_eq!(options[1].expiry_date, now + 5);

    assert_eq!(options[0].base_asset_index, DexAsset::BTC as u8);
    assert_eq!(options[1].base_asset_index, DexAsset::BTC as u8);

    assert_eq!(options[0].quote_asset_index, DexAsset::USDC as u8);
    assert_eq!(options[1].quote_asset_index, DexAsset::USDC as u8);

    assert_eq!(options[0].borrowed_base_funds, btc(0.1 * 500. / 10000.));
    assert_eq!(options[1].borrowed_base_funds, btc(0.2 * 1000. / 10000.));

    assert_eq!(
        options[0].borrowed_quote_funds,
        usdc(0.1 * 25000. * (1. + 500. / 10000.))
    );
    assert_eq!(
        options[1].borrowed_quote_funds,
        usdc(0.2 * 25000. * (1. + 1000. / 10000.))
    );

    market
        .assert_liquidity(
            DexAsset::BTC,
            1. - 0.1 * 500. / 10000. - 0.2 * 1000. / 10000.,
        )
        .await;

    market
        .assert_liquidity(
            DexAsset::USDC,
            19980. - 0.1 * 25000. * (1. + 500. / 10000.) - 0.2 * 25000. * (1. + 1000. / 10000.),
        )
        .await;

    // Check option volume
    admin.assert_di_option_volume(100, btc(0.3)).await;
}

#[tokio::test]
async fn test_single_user_multiple_btc_put() {
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

    // Create put option: premium = 5%, strike = 18000, minimum size = 100 usdc
    let now = now();
    admin
        .di_create_btc_put(100, 500, now + 5, 18000., 100.)
        .await;

    // Open 180 usdc
    user.mint_usdc(540.).await;
    user.di_buy(100, 500, usdc(180.)).await.assert_ok();
    user.assert_usdc_balance(360.).await;

    admin.di_update_option(100, 1000, false).await.assert_ok();

    dtc.advance_clock(now + 1).await;

    // Another 360 usdc
    user.di_buy(100, 1000, usdc(360.)).await.assert_ok();
    user.assert_usdc_balance(0.).await;

    // Collect all options
    let options = user.di_collect_my_options(100).await;
    assert_eq!(options.len(), 2);

    assert_eq!(options[0].size, usdc(180.));
    assert_eq!(options[1].size, usdc(360.));

    assert_eq!(options[0].strike_price, usdc(18000.));
    assert_eq!(options[1].strike_price, usdc(18000.));

    assert_eq!(options[0].premium_rate, 500);
    assert_eq!(options[1].premium_rate, 1000);

    assert_eq!(options[0].expiry_date, now + 5);
    assert_eq!(options[1].expiry_date, now + 5);

    assert_eq!(options[0].base_asset_index, DexAsset::BTC as u8);
    assert_eq!(options[1].base_asset_index, DexAsset::BTC as u8);

    assert_eq!(options[0].quote_asset_index, DexAsset::USDC as u8);
    assert_eq!(options[1].quote_asset_index, DexAsset::USDC as u8);

    assert_eq!(
        options[0].borrowed_base_funds,
        btc((180. / 18000.) * (1. + 500. / 10000.))
    );
    assert_eq!(
        options[1].borrowed_base_funds,
        btc((360. / 18000.) * (1. + 1000. / 10000.))
    );

    assert_eq!(options[0].borrowed_quote_funds, usdc(180. * 500. / 10000.));
    assert_eq!(options[1].borrowed_quote_funds, usdc(360. * 1000. / 10000.));

    market
        .assert_liquidity(
            DexAsset::BTC,
            1. - (180. / 18000.) * (1. + 500. / 10000.) - (360. / 18000.) * (1. + 1000. / 10000.),
        )
        .await;

    market
        .assert_liquidity(
            DexAsset::USDC,
            19980. - 180. * 500. / 10000. - 360. * 1000. / 10000.,
        )
        .await;

    // Check option volume
    market.assert_di_option_volume(100, usdc(540.)).await;
}

#[tokio::test]
async fn test_sol_call_success() {
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
    market.add_liquidity_with_sol(1000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 20000.).await;
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Create call option: premium = 5%, strike = 25000, minimum size = 10. sol
    let now = now();
    admin.di_create_sol_call(100, 500, now + 5, 25., 10.).await;

    let sol_balance_before = user.balance().await;
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
    let sol_balance_after = user.balance().await;

    println!(
        "Sol balance before {} & after {}",
        sol_balance_before, sol_balance_after
    );

    // Check user balance
    assert!(sol_balance_before - sol_balance_after > sol(10.));
}

#[tokio::test]
async fn test_sol_put_success() {
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
    market.add_liquidity_with_sol(1000.).await;

    market.assert_liquidity(DexAsset::BTC, 1.).await;
    market.assert_liquidity(DexAsset::ETH, 10.).await;
    market.assert_liquidity(DexAsset::USDC, 20000.).await;
    market.assert_liquidity(DexAsset::SOL, 999.).await;
    market.assert_fee(DexAsset::SOL, 1.).await;

    // Create put option: premium = 5%, strike = 15, minimum size = 100. usdc
    let now = now();
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
}
