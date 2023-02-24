#![cfg(test)]
use anchor_client::solana_sdk::{
    instruction::Instruction, signer::Signer, transaction::Transaction,
};

mod context;
mod utils;

use crate::utils::DexAsset;
use solana_program_test::{tokio, ProgramTestContext};

use context::DexTestContext;
use spl_associated_token_account::get_associated_token_address;

use utils::helper::create_associated_token_account;

#[tokio::test]
async fn test_close_associate_account_created_by_others() {
    let dtc = DexTestContext::new().await;
    let payer = &dtc.user_context[0];
    let user = &dtc.user_context[1];

    let ai = payer.dex_info.borrow().assets[DexAsset::USDC as usize];
    let context: &mut ProgramTestContext = &mut payer.context.borrow_mut();

    let user_pubkey = user.user.pubkey();
    let user_mint_acc = get_associated_token_address(&user_pubkey, &ai.mint);
    if let Ok(None) = context.banks_client.get_account(user_mint_acc).await {
        create_associated_token_account(context, &payer.user, &user_pubkey, &ai.mint).await
    } else {
        assert!(false);
    }

    let close_account_ix = spl_token::instruction::close_account(
        &spl_token::id(),
        &user_mint_acc,
        &user_pubkey,
        &user_pubkey,
        &[&user_pubkey],
    )
    .unwrap();

    let instructions: Vec<Instruction> = vec![close_account_ix];

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&user_pubkey),
        &[&user.user],
        context.banks_client.get_latest_blockhash().await.unwrap(),
    );

    let res = context
        .banks_client
        .process_transaction_with_preflight(transaction)
        .await;

    assert!(res.is_ok());
}
