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

use super::{compose_cancel_ix, create_token_account};

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    user_order_slot: u8,
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

    let cancel_ix = compose_cancel_ix(
        program,
        user,
        dex,
        user_state,
        order_book,
        order_pool_entry_page,
        mint,
        vault,
        program_signer,
        &user_mint_acc,
        remaining_accounts,
        user_order_slot,
    )
    .await;

    let mut instructions: Vec<Instruction> = vec![cancel_ix];
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
