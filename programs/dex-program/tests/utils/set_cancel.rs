#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{
        instruction::Instruction, signature::Keypair, signer::Signer, transaction::Transaction,
        transport::TransportError,
    },
    Program,
};
use anchor_lang::prelude::{AccountMeta, Pubkey};
use solana_program_test::ProgramTestContext;

use super::compose_cancel_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    user_order_slot: u8,
) -> Result<(), TransportError> {
    let cancel_ix = compose_cancel_ix(
        program,
        user,
        dex,
        user_state,
        order_book,
        order_pool_entry_page,
        remaining_accounts,
        user_order_slot,
    )
    .await;

    let instructions: Vec<Instruction> = vec![cancel_ix];

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
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
