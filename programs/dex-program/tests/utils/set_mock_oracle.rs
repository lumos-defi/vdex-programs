#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{
        signature::Keypair, signer::Signer, transaction::Transaction, transport::TransportError,
    },
    Program,
};
use solana_program_test::ProgramTestContext;

use super::compose_init_mock_oracle_ix;

pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    mock_oracle: &Keypair,
    init_price: u64,
    price_expo: u8,
) -> Result<(), TransportError> {
    let init_mock_oracle_ix =
        compose_init_mock_oracle_ix(program, payer, mock_oracle, init_price, price_expo);

    let transaction = Transaction::new_signed_with_payer(
        &[init_mock_oracle_ix],
        Some(&payer.pubkey()),
        &[payer, mock_oracle],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}
