#![cfg(test)]

mod context;
mod utils;

use crate::utils::{
    sol, vlp, DexAsset, DexMarket, TestResult, TEST_BTC_ORACLE_PRICE, TEST_ETH_ORACLE_PRICE,
    TEST_SOL_ORACLE_PRICE,
};
use context::DexTestContext;
use dex_program::utils::{
    REWARD_PERCENTAGE_FOR_VDX_POOL, REWARD_SHARE_POW_DECIMALS, UPDATE_REWARDS_PERIOD,
};
use solana_program_test::tokio;

fn calc_rewards_per_share(total: u64, total_shares: u64) -> u64 {
    (total as u128 * REWARD_SHARE_POW_DECIMALS as u128 / total_shares as u128) as u64
}

fn calc_rewards(rewards_per_share: u64, shares: u64) -> u64 {
    ((rewards_per_share as u128 * shares as u128) / REWARD_SHARE_POW_DECIMALS as u128) as u64
}

#[tokio::test]
async fn test_claim_rewards_same_share() {
    let dtc = DexTestContext::new_with_no_liquidity().await;
    let anonymous = &dtc.user_context[5];

    // Four user have the same amount of VLP staked, they will have the same rewards.
    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.add_liquidity_with_sol(100.0).await;
        user.add_liquidity_with_usdc(20000.).await;
    }

    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.assert_vlp(40000.).await;
    }

    //  Alice open short position, rewards generated.
    let alice = &dtc.user_context[4];
    alice.mint_usdc(2000.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;

    let expected_open_fee = 58.252427;
    alice.assert_fee(DexAsset::USDC, expected_open_fee).await;
    dtc.assert_total_rewards(0).await;

    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.assert_rewards(0.).await;
    }

    // Collect rewards
    dtc.after(UPDATE_REWARDS_PERIOD).await;
    anonymous.compound().await.assert_ok();

    let expect_total_rewards = expected_open_fee / TEST_SOL_ORACLE_PRICE;
    let expect_vdx_pool_rewards =
        expect_total_rewards * REWARD_PERCENTAGE_FOR_VDX_POOL as f64 / 100.;
    let expect_vlp_pool_rewards = expect_total_rewards - expect_vdx_pool_rewards;

    dtc.assert_total_rewards(sol(expect_total_rewards)).await;
    dtc.assert_vdx_pool_rewards(sol(expect_vdx_pool_rewards))
        .await;
    dtc.assert_vlp_pool_rewards(sol(expect_vlp_pool_rewards))
        .await;

    let mut total_share = 0;
    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.assert_vlp(40000.).await;
        total_share += vlp(40000.);
    }

    // Check each user's pending rewards
    let expect_rewards_per_share =
        calc_rewards_per_share(sol(expect_vlp_pool_rewards), total_share);
    let expect_rewards_of_each_user = calc_rewards(expect_rewards_per_share, vlp(40000.));
    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.assert_pending_rewards(expect_rewards_of_each_user)
            .await;
    }

    // User[0] claims rewards
    let user_0 = &dtc.user_context[0];
    user_0
        .claim_rewards(expect_rewards_of_each_user)
        .await
        .assert_ok();

    // User[0] has no pending rewards
    user_0.assert_pending_rewards(0).await;

    // Check total rewards
    dtc.assert_vlp_pool_rewards(sol(expect_vlp_pool_rewards) - expect_rewards_of_each_user)
        .await;

    // User[1..4] hold pending rewards
    for i in 1..4 {
        let user = &dtc.user_context[i];
        user.assert_pending_rewards(expect_rewards_of_each_user)
            .await;
    }

    // User[1..4] claim rewards
    for i in 1..4 {
        let user = &dtc.user_context[i];
        user.claim_rewards(expect_rewards_of_each_user)
            .await
            .assert_ok();
        dtc.advance_second().await;
    }

    for i in 1..4 {
        let user = &dtc.user_context[i];
        user.assert_pending_rewards(0).await;
    }

    // Pool has some dust rewards left, not zero
    dtc.assert_vlp_pool_rewards(sol(expect_vlp_pool_rewards) - expect_rewards_of_each_user * 4)
        .await;
}

#[tokio::test]
async fn test_claim_rewards_different_share() {
    let dtc = DexTestContext::new_with_no_liquidity().await;
    let anonymous = &dtc.user_context[5];

    // Four user have the 10 / 20 / 30 / 40 percent of VLP staked, check their rewards.
    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.add_liquidity_with_sol(100.0 * (i + 1) as f64).await;
        user.add_liquidity_with_usdc(10000. * (i + 1) as f64).await;
    }

    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.assert_vlp(30000. * (i + 1) as f64).await;
    }

    //  Alice open short position, rewards generated.
    let alice = &dtc.user_context[4];
    alice.mint_usdc(2000.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;
    alice.assert_usdc_balance(0.).await;

    let expected_open_fee = 58.252427;
    alice.assert_fee(DexAsset::USDC, expected_open_fee).await;
    dtc.assert_total_rewards(0).await;

    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.assert_rewards(0.).await;
    }

    // Collect rewards
    dtc.after(UPDATE_REWARDS_PERIOD).await;
    anonymous.compound().await.assert_ok();

    let expect_total_rewards = expected_open_fee / TEST_SOL_ORACLE_PRICE;
    let expect_vdx_pool_rewards =
        expect_total_rewards * REWARD_PERCENTAGE_FOR_VDX_POOL as f64 / 100.;
    let expect_vlp_pool_rewards = expect_total_rewards - expect_vdx_pool_rewards;

    dtc.assert_total_rewards(sol(expect_total_rewards)).await;
    dtc.assert_vdx_pool_rewards(sol(expect_vdx_pool_rewards))
        .await;
    dtc.assert_vlp_pool_rewards(sol(expect_vlp_pool_rewards))
        .await;

    let mut total_share = 0;
    for i in 0..4 {
        let user = &dtc.user_context[i];
        let user_vlp = 30000. * (i + 1) as f64;
        user.assert_vlp(user_vlp).await;
        total_share += vlp(user_vlp);
    }

    // Check each user's pending rewards
    let expect_rewards_per_share =
        calc_rewards_per_share(sol(expect_vlp_pool_rewards), total_share);

    for i in 0..4 {
        let user = &dtc.user_context[i];
        let user_vlp = 30000. * (i + 1) as f64;
        let user_expect_rewards = calc_rewards(expect_rewards_per_share, vlp(user_vlp));

        user.assert_pending_rewards(user_expect_rewards).await;
    }

    // User[0] claims rewards
    let user_0 = &dtc.user_context[0];
    let user_0_expect_rewards = calc_rewards(expect_rewards_per_share, vlp(30000.));
    user_0
        .claim_rewards(user_0_expect_rewards)
        .await
        .assert_ok();

    // User[0] has no pending rewards
    user_0.assert_pending_rewards(0).await;

    // Check total rewards
    dtc.assert_vlp_pool_rewards(sol(expect_vlp_pool_rewards) - user_0_expect_rewards)
        .await;

    // User[1..4] hold pending rewards
    for i in 1..4 {
        let user = &dtc.user_context[i];
        let user_vlp = 30000. * (i + 1) as f64;
        let user_expect_rewards = calc_rewards(expect_rewards_per_share, vlp(user_vlp));
        user.assert_pending_rewards(user_expect_rewards).await;
    }
}

#[tokio::test]
async fn test_increment_rewards() {
    let dtc = DexTestContext::new_with_no_liquidity().await;
    let anonymous = &dtc.user_context[5];
    let alice = &dtc.user_context[4];

    // Four user have the 10 / 20 / 30 / 40 percent of VLP staked, check their rewards.
    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.add_liquidity_with_sol(100.0 * (i + 1) as f64).await;
        user.add_liquidity_with_usdc(10000. * (i + 1) as f64).await;
        user.add_liquidity_with_eth(5. * (i + 1) as f64).await;
        user.add_liquidity_with_btc(0.5 * (i + 1) as f64).await;
        dtc.advance_second().await;
    }

    let mut total_share = 0;
    for i in 0..4 {
        let user = &dtc.user_context[i];
        let user_vlp = 50000. * (i + 1) as f64;
        user.assert_vlp(user_vlp).await;
        total_share += vlp(user_vlp);
    }

    let mut expect_total_rewards = 0f64;
    let mut expect_vdx_pool_rewards = 0f64;
    let mut expect_vlp_pool_rewards = 0f64;

    //  Alice open short btc position, rewards increased.
    alice.mint_usdc(2000.).await;
    alice
        .assert_open(DexAsset::USDC, DexMarket::BTC, false, 2000., 10 * 1000)
        .await;

    let expected_open_fee = 58.252427; // In usdc
    let increase_rewards = expected_open_fee / TEST_SOL_ORACLE_PRICE;

    dtc.after(UPDATE_REWARDS_PERIOD).await;
    anonymous.compound().await.assert_ok(); // Collect rewards

    expect_total_rewards += increase_rewards;
    expect_vdx_pool_rewards += increase_rewards * REWARD_PERCENTAGE_FOR_VDX_POOL as f64 / 100.;
    expect_vlp_pool_rewards += increase_rewards - expect_vdx_pool_rewards;

    dtc.assert_total_rewards(sol(expect_total_rewards)).await;
    dtc.assert_vdx_pool_rewards(sol(expect_vdx_pool_rewards))
        .await;
    dtc.assert_vlp_pool_rewards(sol(expect_vlp_pool_rewards))
        .await;

    // Check each user's pending rewards
    let mut expect_rewards_per_share =
        calc_rewards_per_share(sol(expect_vlp_pool_rewards), total_share);
    println!("expect rewards per share: {}", expect_rewards_per_share);
    for i in 0..4 {
        let user = &dtc.user_context[i];
        let user_vlp = 50000. * (i + 1) as f64;
        let user_expect_rewards = calc_rewards(expect_rewards_per_share, vlp(user_vlp));

        user.assert_pending_rewards(user_expect_rewards).await;
    }

    //  Alice open long btc position, rewards increased.
    alice.mint_btc(0.1).await;
    alice
        .assert_open(DexAsset::BTC, DexMarket::BTC, true, 0.1, 10 * 1000)
        .await;
    alice.assert_btc_balance(0.).await;

    let expected_open_fee = 0.002912621;
    let increase_rewards = expected_open_fee * TEST_BTC_ORACLE_PRICE / TEST_SOL_ORACLE_PRICE;
    println!("increase rewards: {}", increase_rewards);
    anonymous.assert_fee(DexAsset::BTC, expected_open_fee).await;

    dtc.after(UPDATE_REWARDS_PERIOD).await;
    anonymous.compound().await.assert_ok(); // Collect rewards

    expect_total_rewards += increase_rewards;
    let increase_vdx_pool_rewards = increase_rewards * REWARD_PERCENTAGE_FOR_VDX_POOL as f64 / 100.;
    expect_vdx_pool_rewards += increase_vdx_pool_rewards;
    expect_vlp_pool_rewards += increase_rewards - increase_vdx_pool_rewards;

    dtc.assert_total_rewards(sol(expect_total_rewards)).await;
    dtc.assert_vdx_pool_rewards(sol(expect_vdx_pool_rewards))
        .await;
    dtc.assert_vlp_pool_rewards(sol(expect_vlp_pool_rewards))
        .await;

    // Check each user's pending rewards
    let increase_vlp_rewards = increase_rewards - increase_vdx_pool_rewards;
    expect_rewards_per_share += calc_rewards_per_share(sol(increase_vlp_rewards), total_share);
    println!("expect rewards per share: {}", expect_rewards_per_share);
    for i in 0..4 {
        let user = &dtc.user_context[i];
        let user_vlp = 50000. * (i + 1) as f64;
        let user_expect_rewards = calc_rewards(expect_rewards_per_share, vlp(user_vlp));

        user.assert_pending_rewards(user_expect_rewards).await;
    }

    //  Alice open long eth position, rewards increased.
    alice.mint_eth(1.).await;
    alice
        .assert_open(DexAsset::ETH, DexMarket::ETH, true, 1., 10 * 1000)
        .await;
    alice.assert_eth_balance(0.).await;

    let expected_open_fee = 0.029126; //0.02912621359
    let increase_rewards = expected_open_fee * TEST_ETH_ORACLE_PRICE / TEST_SOL_ORACLE_PRICE;
    println!("increase rewards: {}", increase_rewards);
    anonymous.assert_fee(DexAsset::ETH, expected_open_fee).await;

    dtc.after(UPDATE_REWARDS_PERIOD).await;
    anonymous.compound().await.assert_ok(); // Collect rewards

    expect_total_rewards += increase_rewards;
    let increase_vdx_pool_rewards = increase_rewards * REWARD_PERCENTAGE_FOR_VDX_POOL as f64 / 100.;
    expect_vdx_pool_rewards += increase_vdx_pool_rewards;
    expect_vlp_pool_rewards += increase_rewards - increase_vdx_pool_rewards;

    dtc.assert_total_rewards(sol(expect_total_rewards)).await;
    dtc.assert_vdx_pool_rewards(sol(expect_vdx_pool_rewards))
        .await;
    dtc.assert_vlp_pool_rewards(sol(expect_vlp_pool_rewards))
        .await;

    // Check each user's pending rewards
    let increase_vlp_rewards = increase_rewards - increase_vdx_pool_rewards;
    expect_rewards_per_share += calc_rewards_per_share(sol(increase_vlp_rewards), total_share);
    println!("expect rewards per share: {}", expect_rewards_per_share);

    let mut total_pending_rewards = 0;
    for i in 0..4 {
        let user = &dtc.user_context[i];
        total_pending_rewards += user.pending_rewards().await;
    }

    println!(
        "User total pending vlp rewards {}, vlp pool total rewards {}",
        total_pending_rewards,
        sol(expect_vlp_pool_rewards)
    );
    assert!(total_pending_rewards <= sol(expect_vlp_pool_rewards));
}
