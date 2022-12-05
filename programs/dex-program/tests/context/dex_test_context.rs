use std::{cell::RefCell, rc::Rc};

use crate::utils::{
    compose_add_asset_ix, compose_add_market_ixs, compose_init_dex_ixs,
    constant::{
        INIT_ADD_SOL_AMOUNT, TEST_BTC_ADD_LIQUIDITY_FEE_RATE, TEST_BTC_ASSET_INDEX,
        TEST_BTC_BORROW_FEE_RATE, TEST_BTC_CHARGE_BORROW_FEE_INTERVAL, TEST_BTC_CLOSE_FEE_RATE,
        TEST_BTC_DECIMALS, TEST_BTC_LIQUIDATE_FEE_RATE, TEST_BTC_MARKET_DECIMALS,
        TEST_BTC_MARKET_SYMBOL, TEST_BTC_MINIMUM_POSITION_VALUE, TEST_BTC_OPEN_FEE_RATE,
        TEST_BTC_ORACLE_EXPO, TEST_BTC_ORACLE_PRICE, TEST_BTC_ORACLE_SOURCE,
        TEST_BTC_REMOVE_LIQUIDITY_FEE_RATE, TEST_BTC_SIGNIFICANT_DECIMALS, TEST_BTC_SWAP_FEE_RATE,
        TEST_BTC_SYMBOL, TEST_BTC_TARGET_WEIGHT, TEST_ETH_ADD_LIQUIDITY_FEE_RATE,
        TEST_ETH_ASSET_INDEX, TEST_ETH_BORROW_FEE_RATE, TEST_ETH_CHARGE_BORROW_FEE_INTERVAL,
        TEST_ETH_CLOSE_FEE_RATE, TEST_ETH_DECIMALS, TEST_ETH_LIQUIDATE_FEE_RATE,
        TEST_ETH_MARKET_DECIMALS, TEST_ETH_MARKET_SYMBOL, TEST_ETH_MINIMUM_POSITION_VALUE,
        TEST_ETH_OPEN_FEE_RATE, TEST_ETH_ORACLE_EXPO, TEST_ETH_ORACLE_PRICE,
        TEST_ETH_ORACLE_SOURCE, TEST_ETH_REMOVE_LIQUIDITY_FEE_RATE, TEST_ETH_SIGNIFICANT_DECIMALS,
        TEST_ETH_SWAP_FEE_RATE, TEST_ETH_SYMBOL, TEST_ETH_TARGET_WEIGHT,
        TEST_SOL_ADD_LIQUIDITY_FEE_RATE, TEST_SOL_ASSET_INDEX, TEST_SOL_BORROW_FEE_RATE,
        TEST_SOL_CHARGE_BORROW_FEE_INTERVAL, TEST_SOL_CLOSE_FEE_RATE, TEST_SOL_DECIMALS,
        TEST_SOL_LIQUIDATE_FEE_RATE, TEST_SOL_MARKET_DECIMALS, TEST_SOL_MARKET_SYMBOL,
        TEST_SOL_MINIMUM_POSITION_VALUE, TEST_SOL_OPEN_FEE_RATE, TEST_SOL_ORACLE_EXPO,
        TEST_SOL_ORACLE_PRICE, TEST_SOL_ORACLE_SOURCE, TEST_SOL_REMOVE_LIQUIDITY_FEE_RATE,
        TEST_SOL_SIGNIFICANT_DECIMALS, TEST_SOL_SWAP_FEE_RATE, TEST_SOL_SYMBOL,
        TEST_SOL_TARGET_WEIGHT, TEST_USDC_ADD_LIQUIDITY_FEE_RATE, TEST_USDC_BORROW_FEE_RATE,
        TEST_USDC_DECIMALS, TEST_USDC_ORACLE_EXPO, TEST_USDC_ORACLE_PRICE,
        TEST_USDC_REMOVE_LIQUIDITY_FEE_RATE, TEST_USDC_SWAP_FEE_RATE, TEST_USDC_SYMBOL,
        TEST_USDC_TARGET_WEIGHT, TEST_VLP_DECIMALS,
    },
    convert_to_big_number, create_mint, create_token_account, get_context, get_dex_info,
    get_keypair_from_file, get_program, set_mock_oracle,
};

use anchor_client::{
    solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction},
    Program,
};
use anchor_lang::prelude::Pubkey;
use dex_program::dex::Dex;
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
            let symbol: &str = TEST_USDC_SYMBOL;
            let decimals: u8 = TEST_USDC_DECIMALS;
            let oracle_price: f64 = TEST_USDC_ORACLE_PRICE;
            let oracle_expo: u8 = TEST_USDC_ORACLE_EXPO;

            let borrow_fee_rate: u16 = TEST_USDC_BORROW_FEE_RATE;
            let add_liquidity_fee_rate: u16 = TEST_USDC_ADD_LIQUIDITY_FEE_RATE;
            let remove_liquidity_fee_rate: u16 = TEST_USDC_REMOVE_LIQUIDITY_FEE_RATE;
            let swap_fee_rate: u16 = TEST_USDC_SWAP_FEE_RATE;
            let target_weight: u16 = TEST_USDC_TARGET_WEIGHT;

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
                swap_fee_rate,
                target_weight,
            )
            .await;
        }

        //3. add BTC asset
        {
            let symbol: &str = TEST_BTC_SYMBOL;
            let decimals: u8 = TEST_BTC_DECIMALS;
            let oracle_price: f64 = TEST_BTC_ORACLE_PRICE;
            let oracle_expo: u8 = TEST_BTC_ORACLE_EXPO;

            let mint = Keypair::new();
            let borrow_fee_rate: u16 = TEST_BTC_BORROW_FEE_RATE;
            let add_liquidity_fee_rate: u16 = TEST_BTC_ADD_LIQUIDITY_FEE_RATE;
            let remove_liquidity_fee_rate: u16 = TEST_BTC_REMOVE_LIQUIDITY_FEE_RATE;
            let swap_fee_rate: u16 = TEST_BTC_SWAP_FEE_RATE;
            let target_weight: u16 = TEST_BTC_TARGET_WEIGHT;

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
                swap_fee_rate,
                target_weight,
            )
            .await;
        }

        //4. add ETH asset
        {
            let symbol: &str = TEST_ETH_SYMBOL;
            let decimals: u8 = TEST_ETH_DECIMALS;
            let oracle_price: f64 = TEST_ETH_ORACLE_PRICE;
            let oracle_expo: u8 = TEST_ETH_ORACLE_EXPO;

            let mint = Keypair::new();
            let borrow_fee_rate: u16 = TEST_ETH_BORROW_FEE_RATE;
            let add_liquidity_fee_rate: u16 = TEST_ETH_ADD_LIQUIDITY_FEE_RATE;
            let remove_liquidity_fee_rate: u16 = TEST_ETH_REMOVE_LIQUIDITY_FEE_RATE;
            let swap_fee_rate: u16 = TEST_ETH_SWAP_FEE_RATE;
            let target_weight: u16 = TEST_ETH_TARGET_WEIGHT;

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
                swap_fee_rate,
                target_weight,
            )
            .await;
        }

        //5. add SOL asset
        {
            let symbol: &str = TEST_SOL_SYMBOL;
            let decimals: u8 = TEST_SOL_DECIMALS;
            let oracle_price: f64 = TEST_SOL_ORACLE_PRICE;
            let oracle_expo: u8 = TEST_SOL_ORACLE_EXPO;

            let borrow_fee_rate: u16 = TEST_SOL_BORROW_FEE_RATE;
            let add_liquidity_fee_rate: u16 = TEST_SOL_ADD_LIQUIDITY_FEE_RATE;
            let remove_liquidity_fee_rate: u16 = TEST_SOL_REMOVE_LIQUIDITY_FEE_RATE;
            let swap_fee_rate: u16 = TEST_SOL_SWAP_FEE_RATE;
            let target_weight: u16 = TEST_SOL_TARGET_WEIGHT;

            add_asset(
                &mut context.borrow_mut(),
                &program,
                &admin,
                &dex,
                &Keypair::new(),
                symbol,
                decimals,
                convert_to_big_number(oracle_price.into(), oracle_expo),
                oracle_expo,
                &sol_mock_oracle,
                borrow_fee_rate,
                add_liquidity_fee_rate,
                remove_liquidity_fee_rate,
                swap_fee_rate,
                target_weight,
            )
            .await;
        }

        //6. add BTC market
        {
            let symbol: &str = TEST_BTC_MARKET_SYMBOL;
            let minimum_position_value: u64 =
                convert_to_big_number(TEST_BTC_MINIMUM_POSITION_VALUE as f64, 6);
            let charge_borrow_fee_interval: u64 = TEST_BTC_CHARGE_BORROW_FEE_INTERVAL;
            let open_fee_rate: u16 = TEST_BTC_OPEN_FEE_RATE;
            let close_fee_rate: u16 = TEST_BTC_CLOSE_FEE_RATE;
            let liquidate_fee_rate: u16 = TEST_BTC_LIQUIDATE_FEE_RATE;
            let decimals: u8 = TEST_BTC_MARKET_DECIMALS;
            let oracle_source: u8 = TEST_BTC_ORACLE_SOURCE;
            let asset_index: u8 = TEST_BTC_ASSET_INDEX;
            let significant_decimals: u8 = TEST_BTC_SIGNIFICANT_DECIMALS;

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
            let symbol: &str = TEST_ETH_MARKET_SYMBOL;
            let minimum_position_value: u64 =
                convert_to_big_number(TEST_ETH_MINIMUM_POSITION_VALUE as f64, 6);
            let charge_borrow_fee_interval: u64 = TEST_ETH_CHARGE_BORROW_FEE_INTERVAL;
            let open_fee_rate: u16 = TEST_ETH_OPEN_FEE_RATE;
            let close_fee_rate: u16 = TEST_ETH_CLOSE_FEE_RATE;
            let liquidate_fee_rate: u16 = TEST_ETH_LIQUIDATE_FEE_RATE;
            let decimals: u8 = TEST_ETH_MARKET_DECIMALS;
            let oracle_source: u8 = TEST_ETH_ORACLE_SOURCE;
            let asset_index: u8 = TEST_ETH_ASSET_INDEX;
            let significant_decimals: u8 = TEST_ETH_SIGNIFICANT_DECIMALS;

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
            let symbol: &str = TEST_SOL_MARKET_SYMBOL;
            let minimum_position_value: u64 =
                convert_to_big_number(TEST_SOL_MINIMUM_POSITION_VALUE as f64, 6);
            let charge_borrow_fee_interval: u64 = TEST_SOL_CHARGE_BORROW_FEE_INTERVAL;
            let open_fee_rate: u16 = TEST_SOL_OPEN_FEE_RATE; // 0.3% (30 / 10000)
            let close_fee_rate: u16 = TEST_SOL_CLOSE_FEE_RATE; // 0.5%   (50 /  10000)
            let liquidate_fee_rate: u16 = TEST_SOL_LIQUIDATE_FEE_RATE; // 0.8%   (80 /  10000)
            let decimals: u8 = TEST_SOL_MARKET_DECIMALS;
            let oracle_source: u8 = TEST_SOL_ORACLE_SOURCE; // 0: mock,1: pyth
            let asset_index: u8 = TEST_SOL_ASSET_INDEX; // 0:usdc, 1:btc, 2:eth, 3:sol
            let significant_decimals: u8 = TEST_SOL_SIGNIFICANT_DECIMALS;

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
        for _ in 0..5 {
            let user = UserTestContext::new(context.clone(), dex.pubkey()).await;
            users.push(user);
        }

        //init reward asset
        {
            let user = UserTestContext::new(context.clone(), dex.pubkey()).await;
            user.add_liquidity_with_sol(INIT_ADD_SOL_AMOUNT).await;
            users.push(user);
        }

        DexTestContext {
            context,
            program,
            admin,
            dex: dex.pubkey(),
            dex_info,
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
    let order_book = Keypair::new();
    let order_pool_entry_page = Keypair::new();

    //add market
    let add_market_ixs = compose_add_market_ixs(
        context,
        program,
        payer,
        dex,
        &order_book.pubkey(),
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
        &[payer, &order_book, &order_pool_entry_page],
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
    swap_fee_rate: u16,
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

    println!("init mock oralce {}", mock_oracle.pubkey());
    let mint_pubkey = if symbol == "SOL" {
        spl_token::native_mint::id()
    } else {
        //create mint
        create_mint(context, payer, &mint, decimals, &payer.pubkey())
            .await
            .unwrap();
        mint.pubkey()
    };

    println!("symbol: {}, mint account:{:?}", symbol, &mint_pubkey);

    //get program signer
    let (program_signer, nonce) = Pubkey::find_program_address(
        &[&mint_pubkey.to_bytes(), &dex.pubkey().to_bytes()],
        &program.id(),
    );

    println!("program signer:{:?}, nonce:{:?}", program_signer, nonce);

    //create vault
    create_token_account(
        context,
        payer,
        &asset_vault,
        &mint_pubkey,
        &program_signer,
        0,
    )
    .await
    .unwrap();

    println!("asset vault:{:?}", asset_vault.pubkey());

    //add dex asset
    let add_dex_asset_ix = compose_add_asset_ix(
        program,
        payer,
        dex,
        &mint_pubkey,
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
        swap_fee_rate,
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
    let reward_mint = spl_token::native_mint::id();

    let vlp_decimals = TEST_VLP_DECIMALS;

    let init_dex_ixs = compose_init_dex_ixs(
        context,
        program,
        payer,
        &dex,
        &usdc_mint,
        &event_queue,
        &match_queue,
        &user_list_entry_page,
        &reward_mint,
        vlp_decimals,
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
