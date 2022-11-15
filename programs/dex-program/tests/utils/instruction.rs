#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]
use std::mem;

use anchor_client::{
    solana_sdk::{
        instruction::Instruction, signer::Signer, system_instruction, system_program, sysvar,
    },
    Program,
};
use anchor_lang::prelude::{AccountMeta, Pubkey};
use dex_program::{
    accounts::{
        AddAsset, AddLiquidity, AddMarket, CreateUserState, FeedMockOraclePrice, InitDex,
        InitMockOracle,
    },
    dex::Dex,
};
use solana_program_test::ProgramTestContext;

use {anchor_client::solana_sdk::signature::Keypair, spl_token};

pub async fn compose_init_dex_ixs(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Keypair,
    usdc_mint: &Keypair,
    event_queue: &Keypair,
    match_queue: &Keypair,
    user_list_entry_page: &Keypair,
    vlp_mint: Pubkey,
    vlp_mint_authority: Pubkey,
    vlp_decimals: u8,
    vlp_mint_nonce: u8,
) -> Vec<Instruction> {
    let rent = context.banks_client.get_rent().await.unwrap();
    let dex_account_size = 8 + mem::size_of::<Dex>();
    let event_queue_account_size = 16 * 1024;
    let match_queue_account_size = 16 * 1024;
    let user_list_entry_page_account_size = 4 * 1024;

    let init_dex_ixs = program
        .request()
        .instruction(system_instruction::create_account(
            &payer.pubkey(),
            &dex.pubkey(),
            rent.minimum_balance(dex_account_size),
            dex_account_size as u64,
            &program.id(),
        ))
        .instruction(system_instruction::create_account(
            &payer.pubkey(),
            &event_queue.pubkey(),
            rent.minimum_balance(event_queue_account_size),
            event_queue_account_size as u64,
            &program.id(),
        ))
        .instruction(system_instruction::create_account(
            &payer.pubkey(),
            &match_queue.pubkey(),
            rent.minimum_balance(match_queue_account_size),
            match_queue_account_size as u64,
            &program.id(),
        ))
        .instruction(system_instruction::create_account(
            &payer.pubkey(),
            &user_list_entry_page.pubkey(),
            rent.minimum_balance(user_list_entry_page_account_size),
            user_list_entry_page_account_size as u64,
            &program.id(),
        ))
        .accounts(InitDex {
            dex: dex.pubkey(),
            usdc_mint: usdc_mint.pubkey(),
            authority: payer.pubkey(),
            event_queue: event_queue.pubkey(),
            match_queue: match_queue.pubkey(),
            user_list_entry_page: user_list_entry_page.pubkey(),
            vlp_mint,
            vlp_mint_authority,
            system_program: system_program::id(),
            token_program: spl_token::id(),
            rent: sysvar::rent::id(),
        })
        .args(dex_program::instruction::InitDex {
            vlp_decimals,
            vlp_mint_nonce,
        })
        .instructions()
        .unwrap();

    init_dex_ixs
}

pub fn compose_init_mock_oracle_ix(
    program: &Program,
    payer: &Keypair,
    oracle: &Keypair,
    price: u64,
    expo: u8,
) -> Instruction {
    let init_mock_oracle_ix = program
        .request()
        .accounts(InitMockOracle {
            mock_oracle: oracle.pubkey(),
            authority: payer.pubkey(),
            system_program: system_program::id(),
        })
        .args(dex_program::instruction::InitMockOracle { price, expo })
        .instructions()
        .unwrap()
        .pop()
        .unwrap();

    init_mock_oracle_ix
}

pub fn compose_feed_mock_oracle_ix(
    program: &Program,
    payer: &Keypair,
    oracle: &Pubkey,
    price: u64,
) -> Instruction {
    let feed_mock_oracle_ix = program
        .request()
        .accounts(FeedMockOraclePrice {
            mock_oracle: *oracle,
            authority: payer.pubkey(),
            system_program: system_program::id(),
        })
        .args(dex_program::instruction::FeedMockOraclePrice { price })
        .instructions()
        .unwrap()
        .pop()
        .unwrap();

    feed_mock_oracle_ix
}

pub fn compose_add_asset_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Keypair,
    mint: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    symbol: String,
    decimals: u8,
    nonce: u8,
    oracle: &Pubkey,
    oracle_source: u8,
    borrow_fee_rate: u16,
    add_liquidity_fee_rate: u16,
    remove_liquidity_fee_rate: u16,
    target_weight: u16,
) -> Instruction {
    let add_dex_asset_ix = program
        .request()
        .accounts(AddAsset {
            dex: dex.pubkey(),
            mint: *mint,
            vault: *vault,
            program_signer: *program_signer,
            oracle: *oracle,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::AddAsset {
            symbol,
            decimals,
            nonce,
            oracle_source,
            borrow_fee_rate,
            add_liquidity_fee_rate,
            remove_liquidity_fee_rate,
            target_weight,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap();

    add_dex_asset_ix
}

pub async fn compose_add_market_ixs(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Keypair,
    long_order_book: &Pubkey,
    short_order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    oracle: &Pubkey,
    symbol: String,
    minimum_open_amount: u64,
    charge_borrow_fee_interval: u64,
    open_fee_rate: u16,
    close_fee_rate: u16,
    liquidate_fee_rate: u16,
    decimals: u8,
    oracle_source: u8,
    asset_index: u8,
    significant_decimals: u8,
) -> Vec<Instruction> {
    let rent = context.banks_client.get_rent().await.unwrap();
    let account_size = 128 * 1024; //128k
    let account_rent = rent.minimum_balance(account_size);

    let add_market_ixs = program
        .request()
        .instruction(system_instruction::create_account(
            &payer.pubkey(),
            long_order_book,
            account_rent,
            account_size as u64,
            &program.id(),
        ))
        .instruction(system_instruction::create_account(
            &payer.pubkey(),
            short_order_book,
            account_rent,
            account_size as u64,
            &program.id(),
        ))
        .instruction(system_instruction::create_account(
            &payer.pubkey(),
            order_pool_entry_page,
            account_rent,
            account_size as u64,
            &program.id(),
        ))
        .accounts(AddMarket {
            dex: dex.pubkey(),
            long_order_book: *long_order_book,
            short_order_book: *short_order_book,
            order_pool_entry_page: *order_pool_entry_page,
            oracle: *oracle,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::AddMarket {
            symbol,
            minimum_open_amount,
            charge_borrow_fee_interval,
            open_fee_rate,
            close_fee_rate,
            liquidate_fee_rate,
            decimals,
            oracle_source,
            asset_index,
            significant_decimals,
        })
        .instructions()
        .unwrap();

    add_market_ixs
}

pub async fn compose_init_user_state_ixs(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
) -> Vec<Instruction> {
    let order_slot_count: u8 = 32;
    let position_slot_count: u8 = 32;

    let init_user_state_ixs = program
        .request()
        .accounts(CreateUserState {
            user_state: *user_state,
            dex: *dex,
            authority: payer.pubkey(),
            system_program: system_program::id(),
        })
        .args(dex_program::instruction::CreateUserState {
            order_slot_count,
            position_slot_count,
        })
        .instructions()
        .unwrap();

    init_user_state_ixs
}

pub async fn compose_add_liquidity_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    user_mint_acc: &Pubkey,
    vlp_mint: &Pubkey,
    vlp_mint_authority: &Pubkey,
    user_vlp_account: &Pubkey,
    amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let add_liquidity_ix = program
        .request()
        .accounts(AddLiquidity {
            dex: *dex,
            mint: *mint,
            vault: *vault,
            program_signer: *program_signer,
            user_mint_acc: *user_mint_acc,
            vlp_mint: *vlp_mint,
            vlp_mint_authority: *vlp_mint_authority,
            user_vlp_account: *user_vlp_account,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::AddLiquidity { amount })
        .instructions()
        .unwrap()
        .pop()
        .unwrap();

    add_liquidity_ix
}
