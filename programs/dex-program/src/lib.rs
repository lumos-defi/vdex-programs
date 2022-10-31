use anchor_lang::prelude::*;
use errors::*;

pub mod collections;
pub mod dex;
pub mod errors;
pub mod utils;

use dex::*;

declare_id!("2aJZ6AufDU5NRzXLg5Ww4S4Nf2tx7xZDQD6he2gjsKyq");

#[program]
pub mod dex_program {

    use super::*;

    pub fn init_dex(ctx: Context<InitDex>) -> DexResult {
        dex::init_dex::handler(ctx)
    }

    pub fn init_mock_oracle(ctx: Context<InitMockOracle>, price: u64, expo: u8) -> DexResult {
        dex::init_mock_oracle::handler(ctx, price, expo)
    }

    pub fn feed_mock_oracle_price(ctx: Context<FeedMockOraclePrice>, price: u64) -> DexResult {
        dex::feed_mock_oracle_price::handler(ctx, price)
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
        open_fee_rate: u16,
        close_fee_rate: u16,
        decimals: u8,
        oracle_source: u8,
        asset_index: u8,
        significant_decimals: u8,
    ) -> DexResult {
        dex::add_market::handler(
            ctx,
            symbol,
            open_fee_rate,
            close_fee_rate,
            decimals,
            oracle_source,
            asset_index,
            significant_decimals,
        )
    }

    pub fn add_liquidity(_ctx: Context<AddLiquidity>) -> DexResult {
        Ok(())
    }

    pub fn remove_liquidity(_ctx: Context<RemoveLiquidity>) -> DexResult {
        Ok(())
    }

    pub fn swap(_ctx: Context<Swap>) -> DexResult {
        Ok(())
    }

    pub fn open_position(_ctx: Context<OpenPosition>) -> DexResult {
        Ok(())
    }

    pub fn close_position(_ctx: Context<ClosePosition>) -> DexResult {
        Ok(())
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
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClosePosition<'info> {
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
