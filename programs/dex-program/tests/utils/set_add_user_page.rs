#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{
        instruction::Instruction, signature::Keypair, signer::Signer, transaction::Transaction,
        transport::TransportError,
    },
    Program,
};
use anchor_lang::prelude::Pubkey;
use solana_program_test::ProgramTestContext;

use super::compose_add_user_page_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    user_list_entry_page: &Pubkey,
    user_list_remaining_pages: &[Pubkey],
    new_page: &Pubkey,
) -> Result<(), TransportError> {
    let ix = compose_add_user_page_ix(
        program,
        user,
        dex,
        user_list_entry_page,
        user_list_remaining_pages,
        new_page,
    );

    let instructions: Vec<Instruction> = vec![ix];

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
