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

use super::{compose_bid_ix, create_token_account};

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    in_mint: &Pubkey,
    in_mint_oracle: &Pubkey,
    in_mint_vault: &Pubkey,
    market_oracle: &Pubkey,
    market_mint: &Pubkey,
    market_mint_oracle: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    user_state: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    market: u8,
    long: bool,
    price: u64,
    amount: u64,
    leverage: u32,
) -> Result<(), TransportError> {
    let user_wsol_acc = Keypair::new();

    let user_mint_acc = if *in_mint == spl_token::native_mint::id() {
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

    let bid_ix = compose_bid_ix(
        program,
        user,
        dex,
        in_mint,
        in_mint_oracle,
        in_mint_vault,
        market_oracle,
        market_mint,
        market_mint_oracle,
        order_book,
        order_pool_entry_page,
        &user_mint_acc,
        user_state,
        remaining_accounts,
        market,
        long,
        price,
        amount,
        leverage,
    )
    .await;

    let mut instructions: Vec<Instruction> = vec![bid_ix];

    if *in_mint == spl_token::native_mint::id() {
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
