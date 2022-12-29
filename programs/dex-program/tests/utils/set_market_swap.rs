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

use super::{compose_market_swap_ix, create_associated_token_account, create_token_account};

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    in_mint: &Pubkey,
    in_mint_oracle: &Pubkey,
    in_vault: &Pubkey,
    out_mint: &Pubkey,
    out_mint_oracle: &Pubkey,
    out_vault: &Pubkey,
    out_vault_program_signer: &Pubkey,
    event_queue: &Pubkey,
    amount: u64,
) -> Result<(), TransportError> {
    let user_wsol_acc = Keypair::new();

    let user_in_mint_acc = if *in_mint == spl_token::native_mint::id() {
        create_token_account(
            context,
            user,
            &user_wsol_acc,
            in_mint,
            &user.pubkey(),
            amount,
        )
        .await
        .unwrap();
        user_wsol_acc.pubkey()
    } else {
        get_associated_token_address(&user.pubkey(), in_mint)
    };

    if let Ok(None) = context.banks_client.get_account(user_in_mint_acc).await {
        create_associated_token_account(context, &user, &user.pubkey(), in_mint).await
    }

    let user_out_mint_acc = if *out_mint == spl_token::native_mint::id() {
        create_token_account(context, user, &user_wsol_acc, out_mint, &user.pubkey(), 0)
            .await
            .unwrap();
        user_wsol_acc.pubkey()
    } else {
        get_associated_token_address(&user.pubkey(), out_mint)
    };

    if let Ok(None) = context.banks_client.get_account(user_out_mint_acc).await {
        create_associated_token_account(context, &user, &user.pubkey(), out_mint).await
    }

    let bid_ix = compose_market_swap_ix(
        program,
        user,
        dex,
        user_state,
        in_mint,
        in_mint_oracle,
        in_vault,
        &user_in_mint_acc,
        out_mint,
        out_mint_oracle,
        out_vault,
        out_vault_program_signer,
        &user_out_mint_acc,
        event_queue,
        amount,
    )
    .await;

    let mut instructions: Vec<Instruction> = vec![bid_ix];

    if *in_mint == spl_token::native_mint::id() {
        let close_wsol_account_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &user_in_mint_acc,
            &user.pubkey(),
            &user.pubkey(),
            &[&user.pubkey()],
        )
        .unwrap();

        instructions.push(close_wsol_account_ix);
    }

    if *out_mint == spl_token::native_mint::id() {
        let close_wsol_account_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &user_out_mint_acc,
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
