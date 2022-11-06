use anchor_lang::prelude::*;
use errors::*;

pub mod collections;
pub mod dex;
pub mod errors;
pub mod pool;
pub mod position;
pub mod user;
pub mod utils;

use dex::*;
use pool::*;
use position::*;
use user::*;

declare_id!("2aJZ6AufDU5NRzXLg5Ww4S4Nf2tx7xZDQD6he2gjsKyq");

#[program]
pub mod dex_program {
    use super::*;

    pub fn init_dex(ctx: Context<InitDex>, vlp_decimals: u8, vlp_mint_nonce: u8) -> DexResult {
        dex::init_dex::handler(ctx, vlp_decimals, vlp_mint_nonce)
    }

    pub fn init_mock_oracle(ctx: Context<InitMockOracle>, price: u64, expo: u8) -> DexResult {
        dex::init_mock_oracle::handler(ctx, price, expo)
    }

    pub fn feed_mock_oracle_price(ctx: Context<FeedMockOraclePrice>, price: u64) -> DexResult {
        dex::feed_mock_oracle_price::handler(ctx, price)
    }

    pub fn create_user_state(
        ctx: Context<CreateUserState>,
        order_slot_count: u8,
        position_slot_count: u8,
    ) -> DexResult {
        user::create::handler(ctx, order_slot_count, position_slot_count)
    }

    pub fn add_asset(
        ctx: Context<AddAsset>,
        symbol: String,
        decimals: u8,
        nonce: u8,
        oracle_source: u8,
        borrowed_fee_rate: u16,
        add_liquidity_fee_rate: u16,
        remove_liquidity_fee_rate: u16,
        target_weight: u16,
    ) -> DexResult {
        dex::add_asset::handler(
            ctx,
            symbol,
            decimals,
            nonce,
            oracle_source,
            borrowed_fee_rate,
            add_liquidity_fee_rate,
            remove_liquidity_fee_rate,
            target_weight,
        )
    }

    pub fn add_market(
        ctx: Context<AddMarket>,
        symbol: String,
        minimum_open_amount: u64,
        charge_borrow_fee_interval: u64,
        open_fee_rate: u16,
        close_fee_rate: u16,
        borrow_fee_rate: u16,
        decimals: u8,
        oracle_source: u8,
        asset_index: u8,
        significant_decimals: u8,
    ) -> DexResult {
        dex::add_market::handler(
            ctx,
            symbol,
            minimum_open_amount,
            charge_borrow_fee_interval,
            open_fee_rate,
            close_fee_rate,
            borrow_fee_rate,
            decimals,
            oracle_source,
            asset_index,
            significant_decimals,
        )
    }

    pub fn add_liquidity(ctx: Context<AddLiquidity>, amount: u64) -> DexResult {
        pool::add_liquidity::handler(ctx, amount)
    }

    pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, amount: u64) -> DexResult {
        pool::remove_liquidity::handler(ctx, amount)
    }

    pub fn swap(_ctx: Context<Swap>) -> DexResult {
        Ok(())
    }

    pub fn open_position(
        ctx: Context<OpenPosition>,
        market: u8,
        long: bool,
        amount: u64,
        leverage: u32,
    ) -> DexResult {
        position::open::handler(ctx, market, long, amount, leverage)
    }

    pub fn close_position(
        ctx: Context<ClosePosition>,
        market: u8,
        long: bool,
        size: u64,
    ) -> DexResult {
        position::close::handler(ctx, market, long, size)
    }

    pub fn liquidate_position(
        ctx: Context<LiquidatePosition>,
        market: u8,
        long: bool,
    ) -> DexResult {
        position::liquidate::handler(ctx, market, long)
    }

    pub fn close_all_positions(_ctx: Context<CloseAllPositions>) -> DexResult {
        Ok(())
    }

    pub fn new_limit_order(_ctx: Context<NewLimitOrder>) -> DexResult {
        Ok(())
    }

    pub fn cancel_limit_order(_ctx: Context<CancelLimitOrder>) -> DexResult {
        Ok(())
    }

    pub fn cancel_all_limit_orders(_ctx: Context<CancelAllLimitOrders>) -> DexResult {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseAllPositions<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct NewLimitOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelLimitOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelAllLimitOrders<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}
