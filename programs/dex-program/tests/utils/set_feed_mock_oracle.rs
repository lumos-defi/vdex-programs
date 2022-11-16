#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{
        signature::Keypair, signer::Signer, transaction::Transaction, transport::TransportError,
    },
    Program,
};
use anchor_lang::prelude::Pubkey;
use solana_program_test::ProgramTestContext;

use super::compose_feed_mock_oracle_ix;

pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    mock_oracle: &Pubkey,
    price: u64,
) -> Result<(), TransportError> {
    let feed_mock_oracle_ix = compose_feed_mock_oracle_ix(program, payer, mock_oracle, price);

    let transaction = Transaction::new_signed_with_payer(
        &[feed_mock_oracle_ix],
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
