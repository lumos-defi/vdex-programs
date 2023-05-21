#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

use anchor_client::{
    solana_sdk::{instruction::Instruction, signer::Signer, system_instruction, system_program},
    Program,
};
use anchor_lang::prelude::{AccountMeta, Pubkey};
use dex_program::{
    accounts::{
        AddAsset, AddLiquidity, AddMarket, CancelAllOrders, CancelOrder, ClaimRewards,
        ClosePosition, Compound, Crank, CreateUserState, DiBuy, DiCreateOption, DiRemoveOption,
        DiSetAdmin, DiSetFeeRate, DiSetSettlePrice, DiSettle, DiUpdateOption, DiWithdrawSettled,
        FeedMockOraclePrice, FillOrder, InitDex, InitMockOracle, LimitAsk, LimitBid, OpenPosition,
        RedeemVdx, RemoveLiquidity, SetLiquidityFeeRate, StakeVdx, Swap, UpdatePrice,
        WithdrawAsset,
    },
    utils::MAX_ASSET_COUNT,
};
use solana_program_test::ProgramTestContext;

use {anchor_client::solana_sdk::signature::Keypair, spl_token};

pub async fn compose_init_dex_ixs(
    program: &Program,
    payer: &Keypair,
    dex: &Keypair,
    usdc_mint: &Keypair,
    event_queue: &Keypair,
    match_queue: &Keypair,
    di_option: &Keypair,
    price_feed: &Keypair,
    reward_mint: &Pubkey,
    vdx_mint: &Pubkey,
    vdx_vault: &Pubkey,
    vdx_program_signer: &Pubkey,
    vdx_nonce: u8,
    di_fee_rate: u16,
) -> Vec<Instruction> {
    let init_dex_ixs = program
        .request()
        .accounts(InitDex {
            dex: dex.pubkey(),
            usdc_mint: usdc_mint.pubkey(),
            authority: payer.pubkey(),
            event_queue: event_queue.pubkey(),
            match_queue: match_queue.pubkey(),
            vdx_program_signer: *vdx_program_signer,
            vdx_mint: *vdx_mint,
            vdx_vault: *vdx_vault,
            reward_mint: *reward_mint,
            di_option: di_option.pubkey(),
            price_feed: price_feed.pubkey(),
        })
        .args(dex_program::instruction::InitDex {
            vdx_nonce,
            di_fee_rate,
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
    minimum_collateral: u64,
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
            minimum_collateral,
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
    let order_slot_count: u8 = 8;
    let position_slot_count: u8 = 8;
    let di_option_slot_count: u8 = 8;

    program
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
            di_option_slot_count,
        })
        .instructions()
        .unwrap()
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
    price_feed: &Pubkey,
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
            price_feed: *price_feed,
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
    price_feed: &Pubkey,
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
            price_feed: *price_feed,
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
    price_feed: &Pubkey,
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
            token_program: spl_token::id(),
            price_feed: *price_feed,
        })
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
    oracle: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    user_mint_acc: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    price_feed: &Pubkey,
    market: u8,
    long: bool,
    size: u64,
) -> Instruction {
    program
        .request()
        .accounts(ClosePosition {
            dex: *dex,
            oracle: *oracle,
            vault: *vault,
            program_signer: *program_signer,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            authority: payer.pubkey(),
            event_queue: *event_queue,
            token_program: spl_token::id(),
            price_feed: *price_feed,
        })
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
    price_feed: &Pubkey,
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
            price_feed: *price_feed,
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
    price_feed: &Pubkey,
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
            price_feed: *price_feed,
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
    price_feed: &Pubkey,
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
            price_feed: *price_feed,
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
    in_mint: &Pubkey,
    in_mint_vault: &Pubkey,
    in_mint_oracle: &Pubkey,
    in_mint_program_signer: &Pubkey,
    market_mint: &Pubkey,
    market_mint_oracle: &Pubkey,
    market_mint_vault: &Pubkey,
    market_mint_program_signer: &Pubkey,
    match_queue: &Pubkey,
    event_queue: &Pubkey,
    price_feed: &Pubkey,
) -> Instruction {
    program
        .request()
        .accounts(Crank {
            dex: *dex,
            user: *user,
            user_state: *user_state,
            user_mint_acc: *user_mint_acc,
            in_mint: *in_mint,
            in_mint_vault: *in_mint_vault,
            in_mint_oracle: *in_mint_oracle,
            in_mint_program_signer: *in_mint_program_signer,
            market_mint: *market_mint,
            market_mint_oracle: *market_mint_oracle,
            market_mint_vault: *market_mint_vault,
            market_mint_program_signer: *market_mint_program_signer,
            match_queue: *match_queue,
            event_queue: *event_queue,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
            system_program: system_program::id(),
            price_feed: *price_feed,
        })
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
    vault: &Pubkey,
    program_signer: &Pubkey,
    user_mint_acc: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    user_order_slot: u8,
) -> Instruction {
    program
        .request()
        .accounts(CancelOrder {
            dex: *dex,
            order_book: *order_book,
            order_pool_entry_page: *order_pool_entry_page,
            vault: *vault,
            program_signer: *program_signer,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
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
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::CancelAllOrders {})
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_market_swap_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    in_mint: &Pubkey,
    in_mint_oracle: &Pubkey,
    in_vault: &Pubkey,
    user_in_mint_acc: &Pubkey,
    out_mint: &Pubkey,
    out_mint_oracle: &Pubkey,
    out_vault: &Pubkey,
    out_vault_program_signer: &Pubkey,
    user_out_mint_acc: &Pubkey,
    event_queue: &Pubkey,
    price_feed: &Pubkey,
    amount: u64,
) -> Instruction {
    program
        .request()
        .accounts(Swap {
            dex: *dex,
            in_mint: *in_mint,
            in_mint_oracle: *in_mint_oracle,
            in_vault: *in_vault,
            user_in_mint_acc: *user_in_mint_acc,
            out_mint: *out_mint,
            out_mint_oracle: *out_mint_oracle,
            out_vault: *out_vault,
            out_vault_program_signer: *out_vault_program_signer,
            user_out_mint_acc: *user_out_mint_acc,
            event_queue: *event_queue,
            user_state: *user_state,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
            price_feed: *price_feed,
        })
        .args(dex_program::instruction::Swap { amount })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_set_fee_rate_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    fee_rate: u16,
) -> Instruction {
    program
        .request()
        .accounts(DiSetFeeRate {
            dex: *dex,
            di_option: *di_option,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::DiSetFeeRate { fee_rate })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_set_admin_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    program
        .request()
        .accounts(DiSetAdmin {
            dex: *dex,
            di_option: *di_option,
            admin: *admin,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::DiSetAdmin {})
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_create_option_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    base_asset_oracle: &Pubkey,
    price_feed: &Pubkey,
    id: u64,
    is_call: bool,
    base_asset_index: u8,
    quote_asset_index: u8,
    premium_rate: u16,
    expiry_date: i64,
    strike_price: u64,
    minimum_open_size: u64,
    maximum_open_size: u64,
    stop_before_expiry: u64,
) -> Instruction {
    program
        .request()
        .accounts(DiCreateOption {
            dex: *dex,
            di_option: *di_option,
            base_asset_oracle: *base_asset_oracle,
            authority: payer.pubkey(),
            price_feed: *price_feed,
        })
        .args(dex_program::instruction::DiCreateOption {
            id,
            is_call,
            base_asset_index,
            quote_asset_index,
            premium_rate,
            expiry_date,
            strike_price,
            minimum_open_size,
            maximum_open_size,
            stop_before_expiry,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_set_settle_price_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    id: u64,
    price: u64,
) -> Instruction {
    program
        .request()
        .accounts(DiSetSettlePrice {
            dex: *dex,
            di_option: *di_option,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::DiSetSettlePrice { id, price })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_update_option_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    id: u64,
    premium_rate: u16,
    stop: bool,
) -> Instruction {
    program
        .request()
        .accounts(DiUpdateOption {
            dex: *dex,
            di_option: *di_option,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::DiUpdateOption {
            id,
            premium_rate,
            stop,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_remove_option_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    event_queue: &Pubkey,
    id: u64,
    force: bool,
) -> Instruction {
    program
        .request()
        .accounts(DiRemoveOption {
            dex: *dex,
            di_option: *di_option,
            event_queue: *event_queue,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::DiRemoveOption { id, force })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_buy_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    base_asset_oracle: &Pubkey,
    in_mint_vault: &Pubkey,
    user_mint_acc: &Pubkey,
    user_state: &Pubkey,
    price_feed: &Pubkey,
    id: u64,
    premium_rate: u16,
    size: u64,
) -> Instruction {
    program
        .request()
        .accounts(DiBuy {
            dex: *dex,
            di_option: *di_option,
            base_asset_oracle: *base_asset_oracle,
            in_mint_vault: *in_mint_vault,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
            price_feed: *price_feed,
        })
        .args(dex_program::instruction::DiBuy {
            id,
            premium_rate,
            size,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_settle_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    di_option: &Pubkey,
    user: &Pubkey,
    user_state: &Pubkey,
    user_mint_acc: &Pubkey,
    quote_asset_oracle: &Pubkey,
    mint_vault: &Pubkey,
    asset_program_signer: &Pubkey,
    event_queue: &Pubkey,
    price_feed: &Pubkey,
    created: u64,
    force: bool,
    settle_price: u64,
) -> Instruction {
    program
        .request()
        .accounts(DiSettle {
            dex: *dex,
            di_option: *di_option,
            user: *user,
            user_state: *user_state,
            user_mint_acc: *user_mint_acc,
            quote_asset_oracle: *quote_asset_oracle,
            mint_vault: *mint_vault,
            asset_program_signer: *asset_program_signer,
            event_queue: *event_queue,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
            system_program: system_program::id(),
            price_feed: *price_feed,
        })
        .args(dex_program::instruction::DiSettle {
            created,
            force,
            settle_price,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_di_withdraw_settled_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    user_mint_acc: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    created: u64,
) -> Instruction {
    program
        .request()
        .accounts(DiWithdrawSettled {
            dex: *dex,
            mint_vault: *vault,
            asset_program_signer: *program_signer,
            user_state: *user_state,
            user_mint_acc: *user_mint_acc,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
        })
        .args(dex_program::instruction::DiWithdrawSettled { created })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_withdraw_asset_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_state: &Pubkey,
    user_mint_acc: &Pubkey,
    vault: &Pubkey,
    program_signer: &Pubkey,
    asset: u8,
) -> Instruction {
    program
        .request()
        .accounts(WithdrawAsset {
            dex: *dex,
            mint_vault: *vault,
            asset_program_signer: *program_signer,
            user_state: *user_state,
            user_mint_acc: *user_mint_acc,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
        })
        .args(dex_program::instruction::WithdrawAsset { asset })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_update_price_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    price_feed: &Pubkey,
    prices: [u64; MAX_ASSET_COUNT],
) -> Instruction {
    program
        .request()
        .accounts(UpdatePrice {
            dex: *dex,
            price_feed: *price_feed,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::UpdatePrice { prices })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_compound_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    price_feed: &Pubkey,
    user_state: &Pubkey,
    vdx_mint: &Pubkey,
    vdx_program_signer: &Pubkey,
    vdx_vault: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    program
        .request()
        .accounts(Compound {
            dex: *dex,
            user_state: *user_state,
            price_feed: *price_feed,
            vdx_program_signer: *vdx_program_signer,
            vdx_mint: *vdx_mint,
            vdx_vault: *vdx_vault,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::Compound {})
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_stake_vdx_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_mint_acc: &Pubkey,
    price_feed: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    vdx_mint: &Pubkey,
    vdx_program_signer: &Pubkey,
    vdx_vault: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    amount: u64,
) -> Instruction {
    program
        .request()
        .accounts(StakeVdx {
            dex: *dex,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            event_queue: *event_queue,
            price_feed: *price_feed,
            vdx_program_signer: *vdx_program_signer,
            vdx_mint: *vdx_mint,
            vdx_vault: *vdx_vault,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::StakeVdx { amount })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_redeem_vdx_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    user_mint_acc: &Pubkey,
    price_feed: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    vdx_mint: &Pubkey,
    vdx_program_signer: &Pubkey,
    vdx_vault: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    amount: u64,
) -> Instruction {
    program
        .request()
        .accounts(RedeemVdx {
            dex: *dex,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            event_queue: *event_queue,
            price_feed: *price_feed,
            vdx_program_signer: *vdx_program_signer,
            vdx_mint: *vdx_mint,
            vdx_vault: *vdx_vault,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::RedeemVdx { amount })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_claim_rewards_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    reward_vault: &Pubkey,
    reward_vault_program_signer: &Pubkey,
    user_mint_acc: &Pubkey,
    user_state: &Pubkey,
    event_queue: &Pubkey,
    price_feed: &Pubkey,
    vdx_program_signer: &Pubkey,
    vdx_mint: &Pubkey,
    vdx_vault: &Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    amount: u64,
) -> Instruction {
    program
        .request()
        .accounts(ClaimRewards {
            dex: *dex,
            reward_vault: *reward_vault,
            reward_vault_program_signer: *reward_vault_program_signer,
            user_mint_acc: *user_mint_acc,
            user_state: *user_state,
            event_queue: *event_queue,
            price_feed: *price_feed,
            vdx_program_signer: *vdx_program_signer,
            vdx_mint: *vdx_mint,
            vdx_vault: *vdx_vault,
            authority: payer.pubkey(),
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(dex_program::instruction::ClaimRewards { amount })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}

pub async fn compose_set_liquidity_fee_rate_ix(
    program: &Program,
    payer: &Keypair,
    dex: &Pubkey,
    index: u8,
    add_fee_rate: u16,
    remove_fee_rate: u16,
) -> Instruction {
    program
        .request()
        .accounts(SetLiquidityFeeRate {
            dex: *dex,
            authority: payer.pubkey(),
        })
        .args(dex_program::instruction::SetLiquidityFeeRate {
            index,
            add_fee_rate,
            remove_fee_rate,
        })
        .instructions()
        .unwrap()
        .pop()
        .unwrap()
}
