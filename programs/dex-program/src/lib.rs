use anchor_lang::prelude::*;
use errors::*;

pub mod collections;
pub mod dex;
pub mod dual_invest;
pub mod errors;
pub mod order;
pub mod pool;
pub mod position;
pub mod user;
pub mod utils;

use dex::*;
use dual_invest::*;
use order::*;
use pool::*;
use position::*;
use user::*;

declare_id!("AzGjndwsJTbc1XRzPkmuFk11V88dNLMiQwGqkqkS1vBD");

#[program]
pub mod dex_program {
    use super::*;

    pub fn init_dex(ctx: Context<InitDex>, vlp_decimals: u8, di_fee_rate: u16) -> DexResult {
        dex::init_dex::handler(ctx, vlp_decimals, di_fee_rate)
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
        di_option_slot_count: u8,
    ) -> DexResult {
        user::create::handler(
            ctx,
            order_slot_count,
            position_slot_count,
            di_option_slot_count,
        )
    }

    pub fn add_asset(
        ctx: Context<AddAsset>,
        symbol: String,
        decimals: u8,
        nonce: u8,
        oracle_source: u8,
        borrow_fee_rate: u16,
        add_liquidity_fee_rate: u16,
        remove_liquidity_fee_rate: u16,
        swap_fee_rate: u16,
        target_weight: u16,
    ) -> DexResult {
        dex::add_asset::handler(
            ctx,
            symbol,
            decimals,
            nonce,
            oracle_source,
            borrow_fee_rate,
            add_liquidity_fee_rate,
            remove_liquidity_fee_rate,
            swap_fee_rate,
            target_weight,
        )
    }

    pub fn add_market(
        ctx: Context<AddMarket>,
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
    ) -> DexResult {
        dex::add_market::handler(
            ctx,
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
        )
    }

    pub fn add_liquidity(ctx: Context<AddLiquidity>, amount: u64) -> DexResult {
        pool::add::handler(ctx, amount)
    }

    pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, vlp_amount: u64) -> DexResult {
        pool::remove::handler(ctx, vlp_amount)
    }

    pub fn swap(ctx: Context<Swap>, amount: u64) -> DexResult {
        pool::swap::handler(ctx, amount)
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

    pub fn limit_bid(
        ctx: Context<LimitBid>,
        market: u8,
        long: bool,
        price: u64,
        amount: u64,
        leverage: u32,
    ) -> DexResult {
        order::bid::handler(ctx, market, long, price, amount, leverage)
    }

    pub fn limit_ask(
        ctx: Context<LimitAsk>,
        market: u8,
        long: bool,
        price: u64,
        size: u64,
    ) -> DexResult {
        order::ask::handler(ctx, market, long, price, size)
    }

    pub fn cancel_order(ctx: Context<CancelOrder>, user_order_slot: u8) -> DexResult {
        order::cancel::handler(ctx, user_order_slot)
    }

    pub fn cancel_all_orders<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelAllOrders<'info>>,
    ) -> DexResult {
        order::cancel_all::handler(ctx)
    }

    pub fn fill_order(ctx: Context<FillOrder>, market: u8) -> DexResult {
        order::fill::handler(ctx, market)
    }

    pub fn crank(ctx: Context<Crank>) -> DexResult {
        order::crank::handler(ctx)
    }

    pub fn withdraw_asset(ctx: Context<WithdrawAsset>, asset: u8) -> DexResult {
        order::withdraw::handler(ctx, asset)
    }

    // Dual investment
    pub fn di_set_admin(ctx: Context<DiSetAdmin>) -> DexResult {
        dual_invest::set_admin::handler(ctx)
    }

    pub fn di_set_fee_rate(ctx: Context<DiSetFeeRate>, fee_rate: u16) -> DexResult {
        dual_invest::set_fee_rate::handler(ctx, fee_rate)
    }

    pub fn di_create_option(
        ctx: Context<DiCreateOption>,
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
    ) -> DexResult {
        dual_invest::create::handler(
            ctx,
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
        )
    }

    pub fn di_set_settle_price(ctx: Context<DiSetSettlePrice>, id: u64, price: u64) -> DexResult {
        dual_invest::set_settle_price::handler(ctx, id, price)
    }

    pub fn di_update_option(
        ctx: Context<DiUpdateOption>,
        id: u64,
        premium_rate: u16,
        stop: bool,
    ) -> DexResult {
        dual_invest::update::handler(ctx, id, premium_rate, stop)
    }

    pub fn di_remove_option(ctx: Context<DiRemoveOption>, id: u64, force: bool) -> DexResult {
        dual_invest::remove::handler(ctx, id, force)
    }

    pub fn di_buy(ctx: Context<DiBuy>, id: u64, premium_rate: u16, size: u64) -> DexResult {
        dual_invest::buy::handler(ctx, id, premium_rate, size)
    }

    pub fn di_settle(
        ctx: Context<DiSettle>,
        created: u64,
        force: bool,
        settle_price: u64,
    ) -> DexResult {
        dual_invest::settle::handler(ctx, created, force, settle_price)
    }

    pub fn di_withdraw_settled(ctx: Context<DiWithdrawSettled>, created: u64) -> DexResult {
        dual_invest::withdraw_settled::handler(ctx, created)
    }

    pub fn init_price_feed(ctx: Context<InitPriceFeed>) -> DexResult {
        dex::init_price_feed::handler(ctx)
    }

    pub fn update_price(ctx: Context<UpdatePrice>, prices: [u64; 16]) -> DexResult {
        dex::update_price::handler(ctx, prices)
    }

    pub fn compound(ctx: Context<Compound>) -> DexResult {
        user::compound::handler(ctx)
    }

    pub fn stake_vdx(ctx: Context<StakeVdx>, amount: u64) -> DexResult {
        user::stake_vdx::handler(ctx, amount)
    }

    pub fn redeem_vdx(ctx: Context<RedeemVdx>, amount: u64) -> DexResult {
        user::redeem_vdx::handler(ctx, amount)
    }
}

#[derive(Accounts)]
pub struct CloseAllPositions<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}
