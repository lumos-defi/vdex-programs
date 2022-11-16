#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction},
    Program,
};
use anchor_lang::prelude::Pubkey;
use solana_program_test::ProgramTestContext;

use super::compose_init_user_state_ixs;

pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
) -> Pubkey {
    //get user state
    let (user_state, _) = Pubkey::find_program_address(
        &[&dex.to_bytes(), &payer.pubkey().to_bytes()],
        &program.id(),
    );

    let init_user_state_ixs = compose_init_user_state_ixs(program, payer, dex, &user_state).await;
    let transaction = Transaction::new_signed_with_payer(
        &init_user_state_ixs,
        Some(&payer.pubkey()),
        &[payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    user_state
}
