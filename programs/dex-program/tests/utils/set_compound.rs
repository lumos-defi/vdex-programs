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

use super::compose_compound_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    price_feed: &Pubkey,
    user_state: &Pubkey,
    vdx_mint: &Pubkey,
    vdx_program_signer: &Pubkey,
    vdx_vault: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
) -> Result<(), TransportError> {
    let compound_ix = compose_compound_ix(
        program,
        payer,
        dex,
        price_feed,
        user_state,
        vdx_mint,
        vdx_program_signer,
        vdx_vault,
        remaining_accounts,
    )
    .await;

    let instructions: Vec<Instruction> = vec![compound_ix];

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
