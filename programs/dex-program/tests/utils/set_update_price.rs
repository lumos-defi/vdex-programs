#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{
        signature::Keypair, signer::Signer, transaction::Transaction, transport::TransportError,
    },
    Program,
};
use anchor_lang::prelude::Pubkey;
use dex_program::utils::MAX_ASSET_COUNT;
use solana_program_test::ProgramTestContext;

use super::compose_update_price_ix;

pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    price_feed: &Pubkey,
    prices: [u64; MAX_ASSET_COUNT],
) -> Result<(), TransportError> {
    let update_price_ix = compose_update_price_ix(program, payer, dex, price_feed, prices).await;

    let transaction = Transaction::new_signed_with_payer(
        &[update_price_ix],
        Some(&payer.pubkey()),
        &[payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}
