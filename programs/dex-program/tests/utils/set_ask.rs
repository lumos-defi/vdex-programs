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

use super::compose_ask_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    oracle: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    user_state: &Pubkey,
    price_feed: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    market: u8,
    long: bool,
    price: u64,
    size: u64,
) -> Result<(), TransportError> {
    let ask_ix = compose_ask_ix(
        program,
        user,
        dex,
        oracle,
        order_book,
        order_pool_entry_page,
        user_state,
        price_feed,
        remaining_accounts,
        market,
        long,
        price,
        size,
    )
    .await;

    let instructions: Vec<Instruction> = vec![ask_ix];

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
