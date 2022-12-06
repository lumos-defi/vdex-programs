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

use super::{compose_crank_ix, create_token_account};

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user: &Pubkey,
    user_state: &Pubkey,
    in_mint_oracle: &Pubkey,
    market_mint: &Pubkey,
    market_mint_oracle: &Pubkey,
    market_mint_vault: &Pubkey,
    market_mint_program_signer: &Pubkey,
    out_mint: &Pubkey,
    match_queue: &Pubkey,
    event_queue: &Pubkey,
    user_list_entry_page: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
) -> Result<(), TransportError> {
    let user_wsol_acc = Keypair::new();

    let user_mint_acc = if *out_mint == spl_token::native_mint::id() {
        create_token_account(context, payer, &user_wsol_acc, out_mint, &payer.pubkey(), 0)
            .await
            .unwrap();
        user_wsol_acc.pubkey()
    } else {
        get_associated_token_address(user, out_mint)
    };

    let crank_ix = compose_crank_ix(
        program,
        payer,
        dex,
        user,
        user_state,
        &user_mint_acc,
        in_mint_oracle,
        market_mint,
        market_mint_oracle,
        market_mint_vault,
        market_mint_program_signer,
        match_queue,
        event_queue,
        user_list_entry_page,
        remaining_accounts,
    )
    .await;

    let instructions: Vec<Instruction> = vec![crank_ix];

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
