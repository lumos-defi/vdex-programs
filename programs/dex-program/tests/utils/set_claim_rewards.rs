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

use super::{compose_claim_rewards_ix, create_token_account};

#[allow(clippy::too_many_arguments)]
pub async fn setup(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    reward_vault: &Pubkey,
    reward_vault_program_signer: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    price_feed: &Pubkey,
    vdx_program_signer: &Pubkey,
    vdx_mint: &Pubkey,
    vdx_vault: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    amount: u64,
) -> Result<(), TransportError> {
    let user_wsol_acc = Keypair::new();
    create_token_account(
        context,
        payer,
        &user_wsol_acc,
        &spl_token::native_mint::id(),
        &payer.pubkey(),
        0,
    )
    .await
    .unwrap();

    let claim_rewards_ix = compose_claim_rewards_ix(
        program,
        payer,
        dex,
        reward_vault,
        reward_vault_program_signer,
        &user_wsol_acc.pubkey(),
        user_state,
        event_queue,
        price_feed,
        vdx_program_signer,
        vdx_mint,
        vdx_vault,
        remaining_accounts,
        amount,
    )
    .await;

    let instructions: Vec<Instruction> = vec![claim_rewards_ix];

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
