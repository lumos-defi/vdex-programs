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

use super::compose_fill_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    oracle: &Pubkey,
    match_queue: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    price_feed: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    market: u8,
) -> Result<(), TransportError> {
    let fill_ix = compose_fill_ix(
        program,
        user,
        dex,
        oracle,
        match_queue,
        order_book,
        order_pool_entry_page,
        price_feed,
        remaining_accounts,
        market,
    )
    .await;

    let instructions: Vec<Instruction> = vec![fill_ix];

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
