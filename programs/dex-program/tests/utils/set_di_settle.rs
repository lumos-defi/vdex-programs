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

use super::{compose_di_settle_ix, create_associated_token_account, create_token_account};

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    user: &Pubkey,
    user_state: &Pubkey,
    mint: &Pubkey,
    quote_asset_oracle: &Pubkey,
    mint_vault: &Pubkey,
    asset_program_signer: &Pubkey,
    event_queue: &Pubkey,
    price_feed: &Pubkey,
    created: u64,
    force: bool,
    settle_price: u64,
    create_user_mint_acc: bool,
) -> Result<(), TransportError> {
    let user_wsol_acc = Keypair::new();

    let mut user_mint_acc = if *mint == spl_token::native_mint::id() {
        if create_user_mint_acc {
            create_token_account(context, payer, &user_wsol_acc, mint, &payer.pubkey(), 0)
                .await
                .unwrap();
        }
        user_wsol_acc.pubkey()
    } else {
        let acc = get_associated_token_address(user, mint);
        if let Ok(None) = context.banks_client.get_account(acc).await {
            if create_user_mint_acc {
                create_associated_token_account(context, payer, user, mint).await
            }
        }

        acc
    };

    if !create_user_mint_acc {
        user_mint_acc = Keypair::new().pubkey();
    }

    let di_settle_ix = compose_di_settle_ix(
        program,
        payer,
        dex,
        di_option,
        user,
        user_state,
        &user_mint_acc,
        quote_asset_oracle,
        mint_vault,
        asset_program_signer,
        event_queue,
        price_feed,
        created,
        force,
        settle_price,
    )
    .await;

    let instructions: Vec<Instruction> = vec![di_settle_ix];

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
