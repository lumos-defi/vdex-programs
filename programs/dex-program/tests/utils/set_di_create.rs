#![allow(dead_code)]
use anchor_client::{
    solana_sdk::{
        signature::Keypair, signer::Signer, transaction::Transaction, transport::TransportError,
    },
    Program,
};
use anchor_lang::prelude::Pubkey;
use solana_program_test::ProgramTestContext;

use super::compose_di_create_option_ix;

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    user: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    base_asset_oracle: &Pubkey,
    id: u64,
    is_call: bool,
    base_asset_index: u8,
    quote_asset_index: u8,
    premium_rate: u16,
    expiry_date: i64,
    strike_price: u64,
    minimum_open_size: u64,
) -> Result<(), TransportError> {
    let ix = compose_di_create_option_ix(
        program,
        user,
        dex,
        di_option,
        base_asset_oracle,
        id,
        is_call,
        base_asset_index,
        quote_asset_index,
        premium_rate,
        expiry_date,
        strike_price,
        minimum_open_size,
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &vec![ix],
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
