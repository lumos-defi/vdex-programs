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

use crate::utils::create_associated_token_account;

use super::compose_add_liquidity_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    admin: &Keypair,
    user: &Keypair,
    dex: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    vlp_mint: &Pubkey,
    vlp_mint_authority: &Pubkey,
    amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Result<(), TransportError> {
    let user_mint_acc = get_associated_token_address(&user.pubkey(), mint);
    let user_vlp_account = get_associated_token_address(&user.pubkey(), vlp_mint);

    //create user asset associated token account
    match context.banks_client.get_account(user_vlp_account).await {
        Ok(None) => create_associated_token_account(context, &user, &user.pubkey(), vlp_mint).await,
        Ok(Some(_)) => {} //if exists do nothing
        Err(_) => {}
    }

    println!("user vlp ata:{}", user_vlp_account);

    let add_liquidity_ix = compose_add_liquidity_ix(
        program,
        user,
        dex,
        mint,
        vault,
        program_signer,
        &user_mint_acc,
        vlp_mint,
        vlp_mint_authority,
        &user_vlp_account,
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
