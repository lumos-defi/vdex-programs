use std::{cell::RefCell, convert::TryInto, rc::Rc};

use crate::utils::{
    compose_add_asset_ix, compose_add_market_ixs, compose_init_dex_ixs, convert_to_big_number,
    create_mint, create_token_account, get_context, get_dex_info, get_keypair_from_file,
    get_program, set_mock_oracle,
};

use anchor_client::{
    solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction},
    Program,
};
use anchor_lang::prelude::Pubkey;
use dex_program::{
    dex::{oracle, Dex},
    pool::{add_liquidity, remove_liquidity},
};
use solana_program_test::ProgramTestContext;

use crate::context::UserTestContext;

pub struct DexTestContext {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub program: Program,

    pub admin: Keypair,
    pub dex: Pubkey,

    pub dex_info: RefCell<Dex>,
    pub user_context: Vec<UserTestContext>,
}
#[allow(dead_code)]
impl DexTestContext {
    // DexTestContext::new()
    //1. init dex
    //2. add USDC asset to dex
    //3. add BTC asset to dex
    //4. add ETH asset to dex
    //5. add SOL asset to dex
    //6. add BTC market to dex
    //7. add ETH market to dex
    //8. add SOL market to dex
    //9. init users
    pub async fn new() -> DexTestContext {
        let context = get_context().await;
        let program = get_program().await;

        let admin =
            get_keypair_from_file(&mut context.borrow_mut(), "tests/fixtures/admin.json").await;

        let dex = Keypair::new();
        let usdc_mint = Keypair::new();

        //oracle
        let usdc_mock_oracle = Keypair::new();
        let btc_mock_oracle = Keypair::new();
        let eth_mock_oracle = Keypair::new();
        let sol_mock_oracle = Keypair::new();

        //1.init dex
        init_dex(
            &mut context.borrow_mut(),
            &program,
            &admin,
            &dex,
            &usdc_mint,
        )
        .await;

        //2. add USDC asset
        {
            let symbol: &str = "USDC";
            let decimals: u8 = 6;
            let oracle_price: f64 = 1.0;
            let oracle_expo: u8 = 5;

            let borrow_fee_rate: u16 = 10; //1-10_000  0.1%
            let add_liquidity_fee_rate: u16 = 10; //0.1%
            let remove_liquidity_fee_rate: u16 = 10; //0.1%
            let target_weight: u16 = 400; //1-1000 //40%

            add_asset(
                &mut context.borrow_mut(),
                &program,
                &admin,
                &dex,
                &usdc_mint,
                symbol,
                decimals,
                convert_to_big_number(oracle_price.into(), oracle_expo),
                oracle_expo,
                &usdc_mock_oracle,
                borrow_fee_rate,
                add_liquidity_fee_rate,
                remove_liquidity_fee_rate,
                target_weight,
            )
            .await;
        }

        //3. add BTC asset
        {
            let symbol: &str = "BTC";
            let decimals: u8 = 9;
            let oracle_price: f64 = 20_000.0;
            let oracle_expo: u8 = 8;

            let mint = Keypair::new();
            let borrow_fee_rate: u16 = 10; //1-10_000  0.1%
            let add_liquidity_fee_rate: u16 = 10; //0.1%
            let remove_liquidity_fee_rate: u16 = 10; //0.1%
            let target_weight: u16 = 300; //1-1000 //30%

            add_asset(
                &mut context.borrow_mut(),
                &program,
                &admin,
                &dex,
                &mint,
                symbol,
                decimals,
                convert_to_big_number(oracle_price.into(), oracle_expo),
                oracle_expo,
                &btc_mock_oracle,
                borrow_fee_rate,
                add_liquidity_fee_rate,
                remove_liquidity_fee_rate,
                target_weight,
            )
            .await;
        }

        //4. add ETH asset
        {
            let symbol: &str = "ETH";
            let decimals: u8 = 9;
            let oracle_price: f64 = 2_000.0;
            let oracle_expo: u8 = 8;

            let mint = Keypair::new();
            let borrow_fee_rate: u16 = 10; //1-10_000  0.1%
            let add_liquidity_fee_rate: u16 = 10; //0.1%
            let remove_liquidity_fee_rate: u16 = 10; //0.1%
            let target_weight: u16 = 200; //1-1000 //20%

            add_asset(
                &mut context.borrow_mut(),
                &program,
                &admin,
                &dex,
                &mint,
                symbol,
                decimals,
                convert_to_big_number(oracle_price.into(), oracle_expo),
                oracle_expo,
                &eth_mock_oracle,
                borrow_fee_rate,
                add_liquidity_fee_rate,
                remove_liquidity_fee_rate,
                target_weight,
            )
            .await;
        }

        //5. add SOL asset
        {
            let symbol: &str = "SOL";
            let decimals: u8 = 9;
            let oracle_price: f64 = 20.0;
            let oracle_expo: u8 = 8;

            let mint = Keypair::new();
            let borrow_fee_rate: u16 = 10; //1-10_000  0.1%
            let add_liquidity_fee_rate: u16 = 10; //0.1%
            let remove_liquidity_fee_rate: u16 = 10; //0.1%
            let target_weight: u16 = 200; //1-1000 //20%

            add_asset(
                &mut context.borrow_mut(),
                &program,
                &admin,
                &dex,
                &mint,
                symbol,
                decimals,
                convert_to_big_number(oracle_price.into(), oracle_expo),
                oracle_expo,
                &sol_mock_oracle,
                borrow_fee_rate,
                add_liquidity_fee_rate,
                remove_liquidity_fee_rate,
                target_weight,
            )
            .await;
        }

        //6. add BTC market
        {
            let symbol: &str = "BTC";
            let minimum_position_value: u64 = 10000;
            let charge_borrow_fee_interval: u64 = 3600;
            let open_fee_rate: u16 = 30; // 0.3% (30 / 10000)
            let close_fee_rate: u16 = 50; // 0.5%   (50 /  10000)
            let liquidate_fee_rate: u16 = 80; // 0.8%   (80 /  10000)
            let decimals: u8 = 9;
            let oracle_source: u8 = 0; // 0: mock,1: pyth
            let asset_index: u8 = 1; // 0:usdc, 1:btc, 2:eth, 3:sol
            let significant_decimals: u8 = 2;

            add_market(
                &mut context.borrow_mut(),
                &program,
                &admin,
                &dex,
                &btc_mock_oracle,
                symbol.to_string(),
                minimum_position_value,
                charge_borrow_fee_interval,
                open_fee_rate,
                close_fee_rate,
                liquidate_fee_rate,
                decimals,
                oracle_source,
                asset_index,
                significant_decimals,
            )
            .await;
        }

        //7. add ETH market
        {
            let symbol: &str = "ETH";
            let minimum_position_value: u64 = 10000;
            let charge_borrow_fee_interval: u64 = 3600;
            let open_fee_rate: u16 = 30; // 0.3% (30 / 10000)
            let close_fee_rate: u16 = 50; // 0.5%   (50 /  10000)
            let liquidate_fee_rate: u16 = 80; // 0.8%   (80 /  10000)
            let decimals: u8 = 9;
            let oracle_source: u8 = 0; // 0: mock,1: pyth
            let asset_index: u8 = 2; // 0:usdc, 1:btc, 2:eth, 3:sol
            let significant_decimals: u8 = 2;

            add_market(
                &mut context.borrow_mut(),
                &program,
                &admin,
                &dex,
                &eth_mock_oracle,
                symbol.to_string(),
                minimum_position_value,
                charge_borrow_fee_interval,
                open_fee_rate,
                close_fee_rate,
                liquidate_fee_rate,
                decimals,
                oracle_source,
                asset_index,
                significant_decimals,
            )
            .await;
        }

        //8. add SOL market
        {
            let symbol: &str = "SOL";
            let minimum_position_value: u64 = 10000;
            let charge_borrow_fee_interval: u64 = 3600;
            let open_fee_rate: u16 = 30; // 0.3% (30 / 10000)
            let close_fee_rate: u16 = 50; // 0.5%   (50 /  10000)
            let liquidate_fee_rate: u16 = 80; // 0.8%   (80 /  10000)
            let decimals: u8 = 9;
            let oracle_source: u8 = 0; // 0: mock,1: pyth
            let asset_index: u8 = 3; // 0:usdc, 1:btc, 2:eth, 3:sol
            let significant_decimals: u8 = 2;

            add_market(
                &mut context.borrow_mut(),
                &program,
                &admin,
                &dex,
                &sol_mock_oracle,
                symbol.to_string(),
                minimum_position_value,
                charge_borrow_fee_interval,
                open_fee_rate,
                close_fee_rate,
                liquidate_fee_rate,
                decimals,
                oracle_source,
                asset_index,
                significant_decimals,
            )
            .await;
        }

        let dex_info = get_dex_info(&mut context.borrow_mut().banks_client, dex.pubkey()).await;

        let mut users: Vec<UserTestContext> = vec![];
        for _ in 0..64 {
            let user = UserTestContext::new(context.clone(), dex.pubkey()).await;
            users.push(user);
        }

        DexTestContext {
            context: context,
            program,
            admin,
            dex: dex.pubkey(),
            dex_info: dex_info,
            user_context: users,
        }
    }
}

pub async fn add_market(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Keypair,
    mock_oracle: &Keypair,
    symbol: String,
    minimum_position_value: u64,
    charge_borrow_fee_interval: u64,
    open_fee_rate: u16,
    close_fee_rate: u16,
    liquidate_fee_rate: u16,
    decimals: u8,
    oracle_source: u8,
    asset_index: u8,
    significant_decimals: u8,
) {
    let long_order_book = Keypair::new();
    let short_order_book = Keypair::new();
    let order_pool_entry_page = Keypair::new();

    //add market
    let add_market_ixs = compose_add_market_ixs(
        context,
        program,
        payer,
        dex,
        &long_order_book.pubkey(),
        &short_order_book.pubkey(),
        &order_pool_entry_page.pubkey(),
        &mock_oracle.pubkey(),
        symbol.to_string(),
        minimum_position_value,
        charge_borrow_fee_interval,
        open_fee_rate,
        close_fee_rate,
        liquidate_fee_rate,
        decimals,
        oracle_source,
        asset_index,
        significant_decimals,
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &add_market_ixs,
        Some(&payer.pubkey()),
        &[
            payer,
            &long_order_book,
            &short_order_book,
            &order_pool_entry_page,
        ],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

pub async fn add_asset(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Keypair,
    mint: &Keypair,
    symbol: &str,
    decimals: u8,
    init_oracle_price: u64,
    oracle_expo: u8,
    mock_oracle: &Keypair,
    borrow_fee_rate: u16,
    add_liquidity_fee_rate: u16,
    remove_liquidity_fee_rate: u16,
    target_weight: u16,
) {
    let asset_vault = Keypair::new();

    //set mock oracle
    set_mock_oracle::setup(
        context,
        program,
        payer,
        &mock_oracle,
        init_oracle_price,
        oracle_expo,
    )
    .await
    .unwrap();

    //create mint
    create_mint(context, payer, &mint, decimals, &payer.pubkey())
        .await
        .unwrap();

    println!("mint account:{:?}", &mint.pubkey());

    //get program signer
    let (program_signer, nonce) = Pubkey::find_program_address(
        &[&mint.pubkey().to_bytes(), &dex.pubkey().to_bytes()],
        &program.id(),
    );

    println!("program signer:{:?}, nonce:{:?}", program_signer, nonce);

    //create vault
    create_token_account(
        context,
        payer,
        &asset_vault,
        &mint.pubkey(),
        &program_signer,
    )
    .await
    .unwrap();

    println!("asset vault:{:?}", asset_vault.pubkey());

    //add dex asset
    let add_dex_asset_ix = compose_add_asset_ix(
        program,
        payer,
        dex,
        &mint.pubkey(),
        &asset_vault.pubkey(),
        &program_signer,
        symbol.to_string(),
        decimals,
        nonce,
        &mock_oracle.pubkey(),
        0, //default mock
        borrow_fee_rate,
        add_liquidity_fee_rate,
        remove_liquidity_fee_rate,
        target_weight,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[add_dex_asset_ix],
        Some(&payer.pubkey()),
        &[payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

pub async fn init_dex(
    context: &mut ProgramTestContext,
    program: &Program,
    payer: &Keypair,
    dex: &Keypair,
    usdc_mint: &Keypair,
) {
    let event_queue = Keypair::new();
    let match_queue = Keypair::new();
    let user_list_entry_page = Keypair::new();

    let vlp_decimals = 8;
    let (vlp_mint, vlp_mint_nonce) =
        Pubkey::find_program_address(&[&dex.pubkey().to_bytes(), b"vlp"], &program.id());

    println!("vlp mint account:{:?}", &vlp_mint);

    //get vlp_authority
    let (vlp_mint_authority, nonce) = Pubkey::find_program_address(
        &[&vlp_mint.to_bytes(), &dex.pubkey().to_bytes()],
        &program.id(),
    );

    println!(
        "vlp mint authority:{:?}, nonce:{:?}",
        vlp_mint_authority, nonce
    );

    let init_dex_ixs = compose_init_dex_ixs(
        context,
        program,
        payer,
        &dex,
        &usdc_mint,
        &event_queue,
        &match_queue,
        &user_list_entry_page,
        vlp_mint,
        vlp_mint_authority,
        vlp_decimals,
        vlp_mint_nonce,
    )
    .await;

    let mut signers: Vec<&Keypair> = vec![];
    signers.push(payer);
    signers.push(dex);
    signers.push(&event_queue);
    signers.push(&match_queue);
    signers.push(&user_list_entry_page);

    let transaction = Transaction::new_signed_with_payer(
        &init_dex_ixs,
        Some(&payer.pubkey()),
        &signers,
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
