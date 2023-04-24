#![cfg(test)]

mod context;
mod utils;

use crate::utils::{es_vdx, TestResult, DAY};
use context::DexTestContext;
use dex_program::utils::{
    ES_VDX_PERCENTAGE_FOR_VDX_POOL, ES_VDX_PER_SECOND, UPDATE_REWARDS_PERIOD, VESTING_PERIOD,
};
use solana_program_test::tokio;

const ES_VDX_PS_F: f64 = ES_VDX_PER_SECOND as f64;

#[tokio::test]
async fn test_es_vdx_minter() {
    let dtc = DexTestContext::new().await;
    let alice = &dtc.user_context[0];

    // After one hour
    dtc.after(3600).await;
    alice.compound().await.assert_ok();

    let es_vdx_total = dtc.pending_es_vdx_total().await;
    assert_eq!(es_vdx_total, es_vdx(ES_VDX_PS_F * 3600.));

    // After one day
    dtc.after(3600 * 24).await;
    alice.compound().await.assert_ok();

    let es_vdx_total = dtc.pending_es_vdx_total().await;
    assert_eq!(
        es_vdx_total,
        es_vdx(ES_VDX_PS_F * 3600. + ES_VDX_PS_F * 3600. * 24.)
    );

    // After one year
    dtc.after(3600 * 24 * 365).await;
    alice.compound().await.assert_ok();

    let es_vdx_total = dtc.pending_es_vdx_total().await;
    assert_eq!(
        es_vdx_total,
        es_vdx(ES_VDX_PS_F * 3600. + ES_VDX_PS_F * 3600. * 24. + ES_VDX_PS_F * 3600. * 24. * 365.)
    );
}

fn es_vdx_for_vlp_pool(amount: u64) -> u64 {
    amount * (100 - ES_VDX_PERCENTAGE_FOR_VDX_POOL as u64) / 100
}

#[tokio::test]
async fn test_four_user_single_compound() {
    let dtc = DexTestContext::new_with_no_liquidity().await;

    // Four user have the same amount of VLP staked, they will have the same es-vdx amount as reward too.
    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.add_liquidity_with_sol(1000.0).await;
    }

    for i in 0..4 {
        let user = &dtc.user_context[i];
        user.assert_vlp(200000.).await;
    }

    let es_vdx_in_vlp_pool = dtc.pending_es_vdx_for_vlp_pool().await;
    assert_eq!(es_vdx_in_vlp_pool, 0);

    // One period later
    dtc.after(UPDATE_REWARDS_PERIOD).await;

    // Each user compounds
    for i in 0..4 {
        let user = &dtc.user_context[i];

        user.compound().await.assert_ok();
        let user_es_vdx = user.pending_es_vdx().await.assert_unwrap();
        assert_eq!(
            user_es_vdx,
            es_vdx_for_vlp_pool(es_vdx(ES_VDX_PS_F * UPDATE_REWARDS_PERIOD as f64)) / 4
        );
    }

    // No es-vdx left in vlp pool
    let es_vdx_in_vlp_pool = dtc.pending_es_vdx_for_vlp_pool().await;
    assert_eq!(es_vdx_in_vlp_pool, 0);

    let alice = &dtc.user_context[0];
    let alice_vesting_es_vdx = alice.pending_es_vdx().await.assert_unwrap();

    let time = dtc.after(DAY).await;
    alice.compound().await.assert_ok();

    let vested_vdx = alice.staked_vdx(time).await.assert_unwrap();
    println!(
        "vested vdx {}, expected {}",
        vested_vdx,
        alice_vesting_es_vdx / VESTING_PERIOD as u64
    );
    assert_eq!(vested_vdx, alice_vesting_es_vdx / VESTING_PERIOD as u64);

    alice.redeem_vdx(vested_vdx).await.assert_ok();
    alice.assert_vdx_balance(vested_vdx).await;
    let vdx = alice.staked_vdx(time).await.assert_unwrap();
    assert_eq!(vdx, 0);

    alice.stake_vdx(vested_vdx).await.assert_ok();
    let current_vdx = alice.staked_vdx(time).await.assert_unwrap();
    assert_eq!(current_vdx, vested_vdx);
}
