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
use spl_associated_token_account::get_associated_token_address;

use super::compose_redeem_vdx_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    price_feed: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    vdx_mint: &Pubkey,
    vdx_program_signer: &Pubkey,
    vdx_vault: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    amount: u64,
) -> Result<(), TransportError> {
    let user_mint_acc = get_associated_token_address(&payer.pubkey(), vdx_mint);

    let redeem_vdx_ix = compose_redeem_vdx_ix(
        program,
        payer,
        dex,
        &user_mint_acc,
        price_feed,
        user_state,
        event_queue,
        vdx_mint,
        vdx_program_signer,
        vdx_vault,
        remaining_accounts,
        amount,
    )
    .await;

    let instructions: Vec<Instruction> = vec![redeem_vdx_ix];

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
