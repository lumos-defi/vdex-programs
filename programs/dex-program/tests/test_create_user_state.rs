#![cfg(test)]

mod utils;

use anchor_client::solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use anchor_lang::prelude::{AccountInfo, Pubkey};
use dex_program::user::state::UserState;
use solana_program_test::tokio;
use utils::{helper::*, instruction::compose_init_user_state_ixs};

#[tokio::test]
async fn test_init_user_state() {
    let dex = Keypair::new();

    let (mut context, program) = get_context_and_program().await;

    //get alice
    let alice = get_keypair_from_file(&mut context, "tests/fixtures/alice.json").await;

    //get user state
    let (user_state, _) = Pubkey::find_program_address(
        &[&dex.pubkey().to_bytes(), &alice.pubkey().to_bytes()],
        &program.id(),
    );

    println!("user state:{:?}", user_state);
    let init_user_state_ixs =
        compose_init_user_state_ixs(&program, &alice, &dex.pubkey(), &user_state).await;
    let transaction = Transaction::new_signed_with_payer(
        &init_user_state_ixs,
        Some(&alice.pubkey()),
        &[&alice],
        context.last_blockhash,
    );

    let banks_client = &mut context.banks_client;
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok());

    let mut user_state_account = context
        .banks_client
        .get_account(user_state)
        .await
        .unwrap()
        .unwrap();

    let owner = alice.pubkey();
    let user_state_account_info: AccountInfo = (&owner, true, &mut user_state_account).into();
    let us = UserState::mount(&user_state_account_info, true).unwrap();

    assert_eq!(us.borrow().meta.order_slot_count, 8);
    assert_eq!(us.borrow().meta.position_slot_count, 8);
    assert_eq!(us.borrow().meta.vlp.staked, 0);
    assert_eq!(us.borrow().meta.vlp.reward_debt, 0);
    assert_eq!(us.borrow().meta.vlp.reward_accumulated, 0);
}
