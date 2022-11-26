#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{
        signature::Keypair, signer::Signer, transaction::Transaction, transport::TransportError,
    },
    Program,
};
use anchor_lang::prelude::{AccountMeta, Pubkey};
use solana_program_test::ProgramTestContext;
use spl_associated_token_account::get_associated_token_address;

use super::compose_add_liquidity_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    event_queue: &Pubkey,
    user_state: &Pubkey,
    amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Result<(), TransportError> {
    let user_mint_acc = get_associated_token_address(&user.pubkey(), mint);

    let add_liquidity_ix = compose_add_liquidity_ix(
        program,
        user,
        dex,
        mint,
        vault,
        &user_mint_acc,
        event_queue,
        &user_state,
        amount,
        remaining_accounts,
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &[add_liquidity_ix],
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
