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

use super::compose_cancel_all_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    close_wsol_account: bool,
    user_wsol_account: &Pubkey,
) -> Result<(), TransportError> {
    let cancel_all_ix =
        compose_cancel_all_ix(program, user, dex, user_state, remaining_accounts).await;

    let mut instructions: Vec<Instruction> = vec![cancel_all_ix];
    if close_wsol_account {
        let close_wsol_account_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            user_wsol_account,
            &user.pubkey(),
            &user.pubkey(),
            &[&user.pubkey()],
        )
        .unwrap();

        instructions.push(close_wsol_account_ix);
    }

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
