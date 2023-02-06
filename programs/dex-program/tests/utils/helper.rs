#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    ops::{Div, Mul},
};

use anchor_client::{
    solana_sdk::{
        hash::Hash, program_pack::Pack, signature::read_keypair_file, signer::Signer,
        system_instruction, transaction::Transaction, transport::TransportError,
    },
    Program,
};
use anchor_lang::prelude::{AccountMeta, Pubkey};
use dex_program::{
    dex::{Dex, MockOracle},
    utils::USDC_POW_DECIMALS,
};
use solana_program_test::{BanksClient, ProgramTest, ProgramTestContext};

use {
    anchor_client::{
        solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair},
        Client, Cluster,
    },
    spl_token,
    std::rc::Rc,
};

pub fn get_dex_program_id() -> Pubkey {
    dex_program::id()
}

pub async fn get_banks_client(program_id: &Pubkey) -> (BanksClient, Keypair, Hash, Program) {
    let pt = ProgramTest::new("dex_program", *program_id, None);
    let (banks_client, payer, recent_blockhash) = pt.start().await;
    let client = Client::new_with_options(
        Cluster::Debug,
        Rc::new(Keypair::new()),
        CommitmentConfig::processed(),
    );
    let program = client.program(*program_id);
    (banks_client, payer, recent_blockhash, program)
}

pub async fn get_context_and_program() -> (ProgramTestContext, Program) {
    let dex_program_id = get_dex_program_id();
    let pt = ProgramTest::new("dex_program", dex_program_id, None);
    let context = pt.start_with_context().await;

    let client = Client::new_with_options(
        Cluster::Debug,
        Rc::new(Keypair::new()),
        CommitmentConfig::processed(),
    );
    let program = client.program(dex_program_id);

    (context, program)
}

pub async fn get_program() -> Program {
    let dex_program_id = get_dex_program_id();

    let client = Client::new_with_options(
        Cluster::Debug,
        Rc::new(Keypair::new()),
        CommitmentConfig::processed(),
    );
    let program = client.program(dex_program_id);
    program
}

pub async fn get_context() -> Rc<RefCell<ProgramTestContext>> {
    let dex_program_id = get_dex_program_id();
    let pt = ProgramTest::new("dex_program", dex_program_id, None);
    let context = pt.start_with_context().await;

    Rc::new(RefCell::new(context))
}

pub async fn get_keypair(file_path: &str) -> Keypair {
    let keypair = read_keypair_file(file_path).unwrap();

    keypair
}

pub async fn get_keypair_from_file(context: &mut ProgramTestContext, file_path: &str) -> Keypair {
    let keypair = read_keypair_file(file_path).unwrap();

    transfer(context, &keypair.pubkey(), 100_000_000_000).await;

    keypair
}

pub async fn create_mint(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
    owner: &Pubkey,
) -> Result<(), TransportError> {
    let rent = context.banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                owner,
                None,
                decimals,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[payer, mint], context.last_blockhash);
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn create_token_account(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    account: &Keypair,
    mint: &Pubkey,
    manager: &Pubkey,
    extra_lamports: u64,
) -> Result<(), TransportError> {
    let rent = context.banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent + extra_lamports,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], context.last_blockhash);

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn mint_tokens(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token_2022::instruction::mint_to(
            &spl_token::id(),
            mint,
            account,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn transfer(context: &mut ProgramTestContext, recipient: &Pubkey, amount: u64) {
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &context.payer.pubkey(),
            recipient,
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.banks_client.get_latest_blockhash().await.unwrap(),
    );

    context
        .banks_client
        .process_transaction_with_preflight(transaction)
        .await
        .unwrap();
}

pub async fn transfer_spl_tokens(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    source: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::transfer(
            &spl_token::id(),
            source,
            destination,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client.get_account(*token).await.unwrap().unwrap();
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(token_account.data.as_slice()).unwrap();
    account_info.amount
}

pub async fn create_associated_token_account(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
) {
    let mut transaction = Transaction::new_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                wallet_address,
                token_mint_address,
                &spl_token::id(),
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer], context.last_blockhash);

    context
        .borrow_mut()
        .banks_client
        //.process_transaction_with_preflight(transaction)
        .process_transaction(transaction)
        .await
        .unwrap();
}

pub async fn get_dex_info(banks_client: &mut BanksClient, dex: Pubkey) -> RefCell<Dex> {
    let dex_account = banks_client.get_account(dex).await.unwrap().unwrap();

    let data_ptr = dex_account.data.as_ptr();
    let dex_info = unsafe { data_ptr.add(8).cast::<Dex>().as_ref() }.unwrap();
    RefCell::new(*dex_info)
}

pub async fn get_mock_oracle_price(context: &mut ProgramTestContext, oracle: Pubkey) -> u64 {
    let oracle_account = context
        .banks_client
        .get_account(oracle)
        .await
        .unwrap()
        .unwrap();

    let data_ptr = oracle_account.data.as_ptr();

    let oracle_price = unsafe { data_ptr.add(8).cast::<MockOracle>().as_ref() }.unwrap();

    oracle_price
        .price
        .div(10u64.pow(oracle_price.expo as u32))
        .mul(USDC_POW_DECIMALS)
}

pub async fn get_oracles_remaining_accounts(dex_info: &Dex) -> Vec<AccountMeta> {
    let mut remaining_accounts: Vec<AccountMeta> = Vec::new();

    //process dex asset oracle account
    for asset in &dex_info.assets {
        if asset.valid {
            remaining_accounts.append(&mut vec![AccountMeta::new(asset.oracle, false)])
        }
    }

    //process perp market oracle account
    for market in &dex_info.markets {
        if market.valid {
            remaining_accounts.append(&mut vec![AccountMeta::new(market.oracle, false)])
        }
    }

    remaining_accounts
}

pub fn convert_to_big_number(number: f64, decimals: u8) -> u64 {
    (number * 10u64.pow(decimals as u32) as f64) as u64
}

pub fn convert_to_big_number_i(number: f64, decimals: u8) -> i64 {
    (number * 10i64.pow(decimals as u32) as f64) as i64
}

pub fn collateral_to_size(collateral: f64, leverage: f64, price: f64, decimals: u8) -> f64 {
    let temp = collateral * leverage / price;
    let decimals_powf = 10u64.pow(decimals as u32) as f64;

    (((temp * decimals_powf) as u64) as f64) / decimals_powf
}

pub fn assert_eq_with_dust(expect: u64, real: u64) {
    let difference = expect as i64 - real as i64;
    if difference.abs() > 1 {
        assert_eq!(expect, real);
    }
}
