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

use super::{
    compose_di_withdraw_settled_ix, create_associated_token_account, create_token_account,
};

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
    let user_wsol_acc = Keypair::new();

    let user_mint_acc = if *mint == spl_token::native_mint::id() {
        create_token_account(context, payer, &user_wsol_acc, mint, &payer.pubkey(), 0)
            .await
            .unwrap();
        user_wsol_acc.pubkey()
    } else {
        let acc = get_associated_token_address(&payer.pubkey(), mint);
        if let Ok(None) = context.banks_client.get_account(acc).await {
            create_associated_token_account(context, payer, &payer.pubkey(), mint).await
        }

        acc
    };

    let di_withdraw_settled_ix = compose_di_withdraw_settled_ix(
        program,
        payer,
        dex,
        user_state,
        &user_mint_acc,
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
