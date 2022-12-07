#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]
use std::mem;

use anchor_client::{
    solana_sdk::{instruction::Instruction, signer::Signer, system_instruction, system_program},
    Program,
};
use anchor_lang::prelude::{AccountMeta, Pubkey};
use dex_program::{
    accounts::{
        AddAsset, AddLiquidity, AddMarket, CancelAllOrders, CancelOrder, ClosePosition, Crank,
        CreateUserState, FeedMockOraclePrice, FillOrder, InitDex, InitMockOracle, LimitAsk,
        LimitBid, OpenPosition, RemoveLiquidity,
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
    reward_mint: &Pubkey,
    vlp_decimals: u8,
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
            reward_mint: *reward_mint,
        })
        .args(dex_program::instruction::InitDex { vlp_decimals })
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
    swap_fee_rate: u16,
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
            swap_fee_rate,
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
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    oracle: &Pubkey,
    symbol: String,
    minimum_position_value: u64,
    charge_borrow_fee_interval: u64,
    open_fee_rate: u16,
    close_fee_rate: u16,
    liquidate_fee_rate: u16,
    max_leverage: u32,
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
            order_book,
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
            order_book: *order_book,
            order_pool_entry_page: *order_pool_entry_page,
            oracle: *oracle,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::AddMarket {
            symbol,
            minimum_position_value,
            charge_borrow_fee_interval,
            open_fee_rate,
            close_fee_rate,
            liquidate_fee_rate,
            max_leverage,
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
    user_mint_acc: &Pubkey,
    event_queue: &Pubkey,
    user_state: &Pubkey,
    amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let add_liquidity_ix = program
        .request()
        .accounts(AddLiquidity {
            dex: *dex,
            mint: *mint,
            vault: *vault,
            user_mint_acc: *user_mint_acc,
            event_queue: *event_queue,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
            user_state: *user_state,
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::AddLiquidity { amount })
        .instructions()
        .unwrap()
        .pop()
        .unwrap();

    add_liquidity_ix
}

pub async fn compose_remove_liquidity_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    user_mint_acc: &Pubkey,
    event_queue: &Pubkey,
    user_state: &Pubkey,
    amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let remove_liquidity_ix = program
        .request()
        .accounts(RemoveLiquidity {
            dex: *dex,
            mint: *mint,
            vault: *vault,
            program_signer: *program_signer,
            user_mint_acc: *user_mint_acc,
            event_queue: *event_queue,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
            user_state: *user_state,
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::RemoveLiquidity { vlp_amount: amount })
        .instructions()
        .unwrap()
        .pop()
        .unwrap();

    remove_liquidity_ix
}

pub async fn compose_open_market_position_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    in_mint: &Pubkey,
    in_mint_oracle: &Pubkey,
    in_mint_vault: &Pubkey,
    market_mint: &Pubkey,
    market_mint_oracle: &Pubkey,
    market_mint_vault: &Pubkey,
    market_oracle: &Pubkey,
    user_mint_acc: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    user_list_entry_page: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    market: u8,
    long: bool,
    amount: u64,
    leverage: u32,
) -> Instruction {
    program
        .request()
        .accounts(OpenPosition {
            dex: *dex,
            in_mint: *in_mint,
            in_mint_oracle: *in_mint_oracle,
            in_mint_vault: *in_mint_vault,
            market_mint: *market_mint,
            market_mint_oracle: *market_mint_oracle,
            market_mint_vault: *market_mint_vault,
            market_oracle: *market_oracle,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            authority: payer.pubkey(),
            event_queue: *event_queue,
            user_list_entry_page: *user_list_entry_page,
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::OpenPosition {
            market,
            long,
            amount,
            leverage,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_close_market_position_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    mint: &Pubkey,
    oracle: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    user_mint_acc: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    user_list_entry_page: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    market: u8,
    long: bool,
    size: u64,
) -> Instruction {
    program
        .request()
        .accounts(ClosePosition {
            dex: *dex,
            mint: *mint,
            oracle: *oracle,
            vault: *vault,
            program_signer: *program_signer,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            authority: payer.pubkey(),
            event_queue: *event_queue,
            user_list_entry_page: *user_list_entry_page,
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::ClosePosition { market, long, size })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_bid_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    in_mint: &Pubkey,
    in_mint_oracle: &Pubkey,
    in_mint_vault: &Pubkey,
    market_oracle: &Pubkey,
    market_mint: &Pubkey,
    market_mint_oracle: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    user_mint_acc: &Pubkey,
    user_state: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    market: u8,
    long: bool,
    price: u64,
    amount: u64,
    leverage: u32,
) -> Instruction {
    program
        .request()
        .accounts(LimitBid {
            dex: *dex,
            in_mint: *in_mint,
            in_mint_oracle: *in_mint_oracle,
            in_mint_vault: *in_mint_vault,
            market_oracle: *market_oracle,
            market_mint: *market_mint,
            market_mint_oracle: *market_mint_oracle,
            order_book: *order_book,
            order_pool_entry_page: *order_pool_entry_page,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::LimitBid {
            market,
            long,
            price,
            amount,
            leverage,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_ask_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    oracle: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    user_state: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    market: u8,
    long: bool,
    price: u64,
    size: u64,
) -> Instruction {
    program
        .request()
        .accounts(LimitAsk {
            dex: *dex,
            oracle: *oracle,
            order_book: *order_book,
            order_pool_entry_page: *order_pool_entry_page,
            user_state: *user_state,
            authority: payer.pubkey(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::LimitAsk {
            market,
            long,
            price,
            size,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_fill_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    oracle: &Pubkey,
    match_queue: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    market: u8,
) -> Instruction {
    program
        .request()
        .accounts(FillOrder {
            dex: *dex,
            oracle: *oracle,
            match_queue: *match_queue,
            order_book: *order_book,
            order_pool_entry_page: *order_pool_entry_page,
            authority: payer.pubkey(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::FillOrder { market })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_crank_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user: &Pubkey,
    user_state: &Pubkey,
    user_mint_acc: &Pubkey,
    in_mint_oracle: &Pubkey,
    market_mint: &Pubkey,
    market_mint_oracle: &Pubkey,
    market_mint_vault: &Pubkey,
    market_mint_program_signer: &Pubkey,
    match_queue: &Pubkey,
    event_queue: &Pubkey,
    user_list_entry_page: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    program
        .request()
        .accounts(Crank {
            dex: *dex,
            user: *user,
            user_state: *user_state,
            user_mint_acc: *user_mint_acc,
            in_mint_oracle: *in_mint_oracle,
            market_mint: *market_mint,
            market_mint_oracle: *market_mint_oracle,
            market_mint_vault: *market_mint_vault,
            market_mint_program_signer: *market_mint_program_signer,
            match_queue: *match_queue,
            event_queue: *event_queue,
            user_list_entry_page: *user_list_entry_page,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
            system_program: system_program::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::Crank {})
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_cancel_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    order_book: &Pubkey,
    order_pool_entry_page: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    user_order_slot: u8,
) -> Instruction {
    program
        .request()
        .accounts(CancelOrder {
            dex: *dex,
            order_book: *order_book,
            order_pool_entry_page: *order_pool_entry_page,
            user_state: *user_state,
            authority: payer.pubkey(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::CancelOrder { user_order_slot })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_cancel_all_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    program
        .request()
        .accounts(CancelAllOrders {
            dex: *dex,
            user_state: *user_state,
            authority: payer.pubkey(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::CancelAllOrders {})
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}
