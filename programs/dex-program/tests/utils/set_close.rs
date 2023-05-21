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

use super::{compose_close_market_position_ix, create_token_account};

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    mint: &Pubkey,
    oracle: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    price_feed: &Pubkey,
    market: u8,
    long: bool,
    size: u64,
) -> Result<(), TransportError> {
    let user_wsol_acc = Keypair::new();

    let user_mint_acc = if *mint == spl_token::native_mint::id() {
        create_token_account(context, user, &user_wsol_acc, mint, &user.pubkey(), 0)
            .await
            .unwrap();
        user_wsol_acc.pubkey()
    } else {
        get_associated_token_address(&user.pubkey(), mint)
    };

    let close_ix = compose_close_market_position_ix(
        program,
        user,
        dex,
        oracle,
        vault,
        program_signer,
        &user_mint_acc,
        user_state,
        event_queue,
        price_feed,
        market,
        long,
        size,
    )
    .await;

    let mut instructions: Vec<Instruction> = vec![close_ix];

    if *mint == spl_token::native_mint::id() {
        let close_wsol_account_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &user_mint_acc,
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
