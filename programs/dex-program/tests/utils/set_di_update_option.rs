#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{
        signature::Keypair, signer::Signer, transaction::Transaction, transport::TransportError,
    },
    Program,
};
use anchor_lang::prelude::Pubkey;
use solana_program_test::ProgramTestContext;

use super::compose_di_update_option_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    id: u64,
    premium_rate: u16,
    stop: bool,
) -> Result<(), TransportError> {
    let ix =
        compose_di_update_option_ix(program, user, dex, di_option, id, premium_rate, stop).await;

    let transaction = Transaction::new_signed_with_payer(
        &vec![ix],
        Some(&user.pubkey()),
        &[user],
        context.banks_client.get_latest_blockhash().await.unwrap(),
    );

    context
        .banks_client
        .process_transaction_with_preflight(transaction)
        .await
        .map_err(|e| e.into())
}
