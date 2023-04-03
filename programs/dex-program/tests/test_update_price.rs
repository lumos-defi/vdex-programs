#![cfg(test)]

mod context;
mod utils;

use context::DexTestContext;
use dex_program::utils::MAX_ASSET_COUNT;
use solana_program_test::tokio;

use crate::utils::now;

#[tokio::test]
async fn test_update_all_price() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    let mut prices = [0.0; MAX_ASSET_COUNT];
    let mut counts = [0u8; MAX_ASSET_COUNT];

    prices[0] = 1.1; //USDC
    prices[1] = 20_000.1; //BTC
    prices[2] = 2_000.1; //ETH
    prices[3] = 20.1; //SOL

    counts[0] = 1;
    counts[1] = 1;
    counts[2] = 1;
    counts[3] = 1;

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;
    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_all_price_two_round_in_one_second() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    let mut prices = [0.0; MAX_ASSET_COUNT];
    let mut counts = [0u8; MAX_ASSET_COUNT];

    prices[0] = 1.1; //USDC
    prices[1] = 20_000.1; //BTC
    prices[2] = 2_000.1; //ETH
    prices[3] = 20.1; //SOL

    alice.update_price(prices).await;

    prices[0] = 1.2; //USDC
    prices[1] = 20_000.2; //BTC
    prices[2] = 2_000.2; //ETH
    prices[3] = 20.2; //SOL

    counts[0] = 1;
    counts[1] = 1;
    counts[2] = 1;
    counts[3] = 1;

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;
    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_all_price_two_round_in_two_second() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    let mut prices = [0.0; MAX_ASSET_COUNT];
    let mut counts = [0u8; MAX_ASSET_COUNT];

    prices[0] = 1.1; //USDC
    prices[1] = 20_000.1; //BTC
    prices[2] = 2_000.1; //ETH
    prices[3] = 20.1; //SOL

    alice.update_price(prices).await;

    prices[0] = 1.2; //USDC
    prices[1] = 20_000.2; //BTC
    prices[2] = 2_000.2; //ETH
    prices[3] = 20.2; //SOL

    let mut now = now();
    now += 1;
    dtc.advance_clock(now).await;

    alice.update_price(prices).await;

    counts[0] = 2;
    counts[1] = 2;
    counts[2] = 2;
    counts[3] = 2;

    alice.assert_latest_price(prices).await;
    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_usdc_price_two_round_in_two_second() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];
    let mut counts = [0u8; MAX_ASSET_COUNT];
    counts[0] = 2;

    alice.update_usdc_price(1.01).await;

    let mut now = now();
    now += 1;
    dtc.advance_clock(now).await;

    alice.update_usdc_price(1.02).await;

    alice.assert_usdc_price(1.02).await;
    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_btc_price_two_round_in_two_second() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];
    let mut counts = [0u8; MAX_ASSET_COUNT];
    counts[1] = 2;

    alice.update_btc_price(20_001.01).await;

    let mut now = now();
    now += 1;
    dtc.advance_clock(now).await;

    alice.update_btc_price(20_001.02).await;

    alice.assert_btc_price(20_001.02).await;
    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_eth_price_two_round_in_two_second() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];
    let mut counts = [0u8; MAX_ASSET_COUNT];
    counts[2] = 2;

    alice.update_eth_price(2_001.01).await;

    let mut now = now();
    now += 1;
    dtc.advance_clock(now).await;

    alice.update_eth_price(2_001.02).await;

    alice.assert_eth_price(2_001.02).await;
    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_sol_price_two_round_in_two_second() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];
    let mut counts = [0u8; MAX_ASSET_COUNT];
    counts[3] = 2;

    alice.update_sol_price(21.01).await;

    let mut now = now();
    now += 1;
    dtc.advance_clock(now).await;

    alice.update_sol_price(21.02).await;

    alice.assert_sol_price(21.02).await;
    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_usdc_price_seven_round() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];
    let mut counts = [0u8; MAX_ASSET_COUNT];
    counts[0] = 5;

    //round 1
    alice.update_usdc_price(1.01).await;
    alice.assert_usdc_price(1.01).await;

    let mut now = now();
    now += 2;
    dtc.advance_clock(now).await;

    //round 2
    alice.update_usdc_price(1.02).await;
    alice.assert_usdc_price(1.02).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 3
    alice.update_usdc_price(1.03).await;
    alice.assert_usdc_price(1.03).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 4
    alice.update_usdc_price(1.04).await;
    alice.assert_usdc_price(1.04).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 5
    alice.update_usdc_price(1.05).await;
    alice.assert_usdc_price(1.05).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 6
    alice.update_usdc_price(1.06).await;
    alice.assert_usdc_price(1.06).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 7
    alice.update_usdc_price(1.07).await;
    alice.assert_usdc_price(1.07).await;

    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_every_price_seven_round() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];
    let mut counts = [0u8; MAX_ASSET_COUNT];
    counts[0] = 5;
    counts[1] = 5;
    counts[2] = 5;
    counts[3] = 5;

    //round 1
    alice.update_usdc_price(1.01).await;
    alice.update_btc_price(20_001.01).await;
    alice.update_eth_price(2_001.01).await;
    alice.update_sol_price(21.01).await;

    let mut now = now();
    now += 2;
    dtc.advance_clock(now).await;

    //round 2
    alice.update_usdc_price(1.02).await;
    alice.update_btc_price(20_001.02).await;
    alice.update_eth_price(2_001.02).await;
    alice.update_sol_price(21.02).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 3
    alice.update_usdc_price(1.03).await;
    alice.update_btc_price(20_001.03).await;
    alice.update_eth_price(2_001.03).await;
    alice.update_sol_price(21.03).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 4
    alice.update_usdc_price(1.04).await;
    alice.update_btc_price(20_001.04).await;
    alice.update_eth_price(2_001.04).await;
    alice.update_sol_price(21.04).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 5
    alice.update_usdc_price(1.05).await;
    alice.update_btc_price(20_001.05).await;
    alice.update_eth_price(2_001.05).await;
    alice.update_sol_price(21.05).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 6
    alice.update_usdc_price(1.06).await;
    alice.update_btc_price(20_001.06).await;
    alice.update_eth_price(2_001.06).await;
    alice.update_sol_price(21.06).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 7
    alice.update_usdc_price(1.07).await;
    alice.update_btc_price(20_001.07).await;
    alice.update_eth_price(2_001.07).await;
    alice.update_sol_price(21.07).await;

    alice.assert_usdc_price(1.07).await;
    alice.assert_btc_price(20_001.07).await;
    alice.assert_eth_price(2_001.07).await;
    alice.assert_sol_price(21.07).await;

    alice.assert_valid_price_count(counts).await;
}

#[tokio::test]
async fn test_update_all_price_seven_round() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    let mut prices = [0.0; MAX_ASSET_COUNT];
    let mut counts = [0u8; MAX_ASSET_COUNT];

    counts[0] = 5;
    counts[1] = 5;
    counts[2] = 5;
    counts[3] = 5;

    //round 1
    prices[0] = 1.1; //USDC
    prices[1] = 20_000.1; //BTC
    prices[2] = 2_000.1; //ETH
    prices[3] = 20.1; //SOL

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;

    let mut now = now();
    now += 2;
    dtc.advance_clock(now).await;

    //round 2
    prices[0] = 1.2;
    prices[1] = 20_000.2;
    prices[2] = 2_000.2;
    prices[3] = 20.2;

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 3
    prices[0] = 1.3;
    prices[1] = 20_000.3;
    prices[2] = 2_000.3;
    prices[3] = 20.3;

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 4
    prices[0] = 1.4;
    prices[1] = 20_000.4;
    prices[2] = 2_000.4;
    prices[3] = 20.4;

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 5
    prices[0] = 1.5;
    prices[1] = 20_000.5;
    prices[2] = 2_000.5;
    prices[3] = 20.5;

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 6
    prices[0] = 1.6;
    prices[1] = 20_000.6;
    prices[2] = 2_000.6;
    prices[3] = 20.6;

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;

    now += 2;
    dtc.advance_clock(now).await;

    //round 7
    prices[0] = 1.7;
    prices[1] = 20_000.7;
    prices[2] = 2_000.7;
    prices[3] = 20.7;

    alice.update_price(prices).await;

    alice.assert_latest_price(prices).await;

    alice.assert_valid_price_count(counts).await;
}
