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
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    quote_asset_oracle: &Pubkey,
    base_mint_vault: &Pubkey,
    quote_mint_vault: &Pubkey,
    base_asset_program_signer: &Pubkey,
    quote_asset_program_signer: &Pubkey,
    event_queue: &Pubkey,
    user_list_entry_page: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    id: u64,
    force: bool,
    settle_price: u64,
) -> Result<(), TransportError> {
    let user_wsol_acc = Keypair::new();

    let user_base_mint_acc = if *base_mint == spl_token::native_mint::id() {
        create_token_account(
            context,
            payer,
            &user_wsol_acc,
            base_mint,
            &payer.pubkey(),
            0,
        )
        .await
        .unwrap();
        user_wsol_acc.pubkey()
    } else {
        let acc = get_associated_token_address(user, base_mint);
        if let Ok(None) = context.banks_client.get_account(acc).await {
            create_associated_token_account(context, payer, user, base_mint).await
        }

        acc
    };

    let user_quote_mint_acc = get_associated_token_address(user, quote_mint);
    if let Ok(None) = context.banks_client.get_account(user_quote_mint_acc).await {
        create_associated_token_account(context, payer, user, quote_mint).await
    }

    let di_settle_ix = compose_di_settle_ix(
        program,
        payer,
        dex,
        di_option,
        user,
        user_state,
        &user_base_mint_acc,
        &user_quote_mint_acc,
        base_mint,
        quote_mint,
        quote_asset_oracle,
        base_mint_vault,
        quote_mint_vault,
        base_asset_program_signer,
        quote_asset_program_signer,
        event_queue,
        user_list_entry_page,
        remaining_accounts,
        id,
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
