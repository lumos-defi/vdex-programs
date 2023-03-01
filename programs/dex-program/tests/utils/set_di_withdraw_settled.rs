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
use spl_associated_token_account::get_associated_token_address;

use super::compose_di_withdraw_settled_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    created: u64,
) -> Result<(), TransportError> {
    let user_mint_acc = get_associated_token_address(&payer.pubkey(), mint);

    let di_withdraw_settled_ix = compose_di_withdraw_settled_ix(
        program,
        payer,
        dex,
        user_state,
        &user_mint_acc,
        mint,
        vault,
        program_signer,
        created,
    )
    .await;

    let instructions: Vec<Instruction> = vec![di_withdraw_settled_ix];

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[payer],
        context.banks_client.get_latest_blockhash().await.unwrap(),
    );

    context
        .banks_client
        .process_transaction_with_preflight(transaction)
        .await
        .map_err(|e| e.into())
}
