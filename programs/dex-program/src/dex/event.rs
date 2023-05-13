use crate::{
    collections::{EventQueue, PackedEvent},
    errors::DexResult,
};
use anchor_lang::prelude::*;
#[cfg(feature = "client-support")]
use serde::Serialize;

#[repr(u8)]
pub enum PositionAct {
    Open = 0,
    Close = 1,
    Liquidate = 2,
}

impl PositionAct {
    fn decode(self) -> (u8, &'static str) {
        match self {
            PositionAct::Open => (0, "opened"),
            PositionAct::Close => (1, "closed"),
            PositionAct::Liquidate => (2, "liquidated"),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
#[cfg_attr(feature = "client-support", derive(Serialize))]
pub struct PositionFilled {
    pub user_state: [u8; 32],
    pub position_status: u8,
    pub price: u64,
    pub size: u64,
    pub collateral: u64,
    pub borrow: u64,
    pub market: u8,

    pub action: u8,
    pub long_or_short: u8,

    pub fee: u64,
    pub borrow_fee: u64,
    // Only for closing position
    pub pnl: i64,
}

impl PackedEvent for PositionFilled {
    const DISCRIMINATOR: u8 = 100;
}

#[derive(AnchorSerialize, AnchorDeserialize)]
#[cfg_attr(feature = "client-support", derive(Serialize))]
pub struct LiquidityMoved {
    pub user_state: [u8; 32],

    pub add: bool,
    pub asset: u8,
    pub asset_amount: u64,
    pub vlp_amount: u64,
    pub fee: u64,
}

impl PackedEvent for LiquidityMoved {
    const DISCRIMINATOR: u8 = 101;
}

#[derive(AnchorSerialize, AnchorDeserialize)]
#[cfg_attr(feature = "client-support", derive(Serialize))]
pub struct AssetSwapped {
    pub user_state: [u8; 32],

    pub in_mint: [u8; 32],
    pub out_mint: [u8; 32],
    pub in_amount: u64,
    pub out_amount: u64,
    pub fee: u64,
}

impl PackedEvent for AssetSwapped {
    const DISCRIMINATOR: u8 = 102;
}

#[derive(AnchorSerialize, AnchorDeserialize)]
#[cfg_attr(feature = "client-support", derive(Serialize))]
pub struct DIOptionSettled {
    pub user_state: [u8; 32],
    pub base_mint: [u8; 32],
    pub quote_mint: [u8; 32],
    pub option_id: u64,
    pub created: u64,
    pub expiry_date: i64,
    pub strike_price: u64,
    pub settle_price: u64,
    pub size: u64,
    pub premium_rate: u16,
    pub withdrawable: u64,
    pub fee: u64,
    pub is_call: bool,
    pub exercised: bool,
    pub position_status: u8,
}

impl PackedEvent for DIOptionSettled {
    const DISCRIMINATOR: u8 = 103;
}

#[derive(AnchorSerialize, AnchorDeserialize)]
#[cfg_attr(feature = "client-support", derive(Serialize))]
pub struct DIOptionRemoved {
    pub base_mint: [u8; 32],
    pub quote_mint: [u8; 32],
    pub expiry_date: i64,
    pub strike_price: u64,
    pub settle_price: u64,
    pub volume: u64,
    pub is_call: bool,
}

impl PackedEvent for DIOptionRemoved {
    const DISCRIMINATOR: u8 = 104;
}

pub trait AppendEvent {
    #[allow(clippy::too_many_arguments)]
    fn fill_position(
        &mut self,
        user_state: [u8; 32],
        position_status: u8,
        market: u8,
        action: PositionAct,
        long: bool,
        price: u64,
        size: u64,
        collateral: u64,
        borrow: u64,
        fee: u64,
        borrow_fee: u64,
        pnl: i64,
    ) -> DexResult;

    #[allow(clippy::too_many_arguments)]
    fn move_liquidity(
        &mut self,
        user_state: [u8; 32],
        add: bool,
        asset: u8,
        asset_amount: u64,
        vlp_amount: u64,
        fee: u64,
    ) -> DexResult;

    fn swap_asset(
        &mut self,
        user_state: [u8; 32],
        in_mint: [u8; 32],
        out_mint: [u8; 32],
        in_amount: u64,
        out_amount: u64,
        fee: u64,
    ) -> DexResult;

    #[allow(clippy::too_many_arguments)]
    fn settle_di_option(
        &mut self,
        option_id: u64,
        created: u64,
        user_state: [u8; 32],
        base_mint: [u8; 32],
        quote_mint: [u8; 32],
        expiry_date: i64,
        strike_price: u64,
        settle_price: u64,
        size: u64,
        premium_rate: u16,
        withdrawable: u64,
        fee: u64,
        is_call: bool,
        exercised: bool,
        position_status: u8,
    ) -> DexResult;

    #[allow(clippy::too_many_arguments)]
    fn remove_di_option(
        &mut self,
        base_mint: [u8; 32],
        quote_mint: [u8; 32],
        expiry_date: i64,
        strike_price: u64,
        settle_price: u64,
        volume: u64,
        is_call: bool,
    ) -> DexResult;
}

impl AppendEvent for EventQueue<'_> {
    #[allow(clippy::too_many_arguments)]
    fn fill_position(
        &mut self,
        user_state: [u8; 32],
        position_status: u8,
        market: u8,
        action: PositionAct,
        long: bool,
        price: u64,
        size: u64,
        collateral: u64,
        borrow: u64,
        fee: u64,
        borrow_fee: u64,
        pnl: i64,
    ) -> DexResult {
        let (code, text) = action.decode();

        let event = PositionFilled {
            user_state,
            position_status,
            price,
            size,
            collateral,
            borrow,
            market,
            action: code,
            long_or_short: if long { 0 } else { 1 },
            fee,
            borrow_fee,
            pnl,
        };

        let event_seq = self.append(event)?;

        msg!(
            "Position {}: {:?} {} {} {} {} {} {} {} {} {} {}",
            text,
            user_state,
            market,
            long,
            price,
            size,
            collateral,
            borrow,
            fee,
            borrow_fee,
            pnl,
            event_seq
        );

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn move_liquidity(
        &mut self,
        user_state: [u8; 32],
        add: bool,
        asset: u8,
        asset_amount: u64,
        vlp_amount: u64,
        fee: u64,
    ) -> DexResult {
        let event = LiquidityMoved {
            user_state,
            add,
            asset,
            asset_amount,
            vlp_amount,
            fee,
        };

        let event_seq = self.append(event)?;
        msg!(
            "Liquidity {}: {:?} {} {} {} {} {} {}",
            if add { "added" } else { "removed" },
            user_state,
            add,
            asset,
            asset_amount,
            vlp_amount,
            fee,
            event_seq
        );
        Ok(())
    }

    fn swap_asset(
        &mut self,
        user_state: [u8; 32],
        in_mint: [u8; 32],
        out_mint: [u8; 32],
        in_amount: u64,
        out_amount: u64,
        fee: u64,
    ) -> DexResult {
        let event = AssetSwapped {
            user_state,
            in_mint,
            out_mint,
            in_amount,
            out_amount,
            fee,
        };

        let event_seq = self.append(event)?;
        msg!(
            "Asset swapped: {:?} {:?} {:?} {} {} {} {}",
            user_state,
            in_mint,
            out_mint,
            in_amount,
            out_amount,
            fee,
            event_seq
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn settle_di_option(
        &mut self,
        option_id: u64,
        created: u64,
        user_state: [u8; 32],
        base_mint: [u8; 32],
        quote_mint: [u8; 32],
        expiry_date: i64,
        strike_price: u64,
        settle_price: u64,
        size: u64,
        premium_rate: u16,
        withdrawable: u64,
        fee: u64,
        is_call: bool,
        exercised: bool,
        position_status: u8,
    ) -> DexResult {
        let event = DIOptionSettled {
            user_state,
            position_status,
            base_mint,
            quote_mint,
            option_id,
            created,
            expiry_date,
            strike_price,
            settle_price,
            size,
            premium_rate,
            withdrawable,
            fee,
            is_call,
            exercised,
        };

        let event_seq = self.append(event)?;
        msg!(
            "DI option settled: {:?} {:?} {:?} {} {} {} {} {} {} {} {} {} {} {} {}",
            user_state,
            base_mint,
            quote_mint,
            option_id,
            created,
            expiry_date,
            strike_price,
            settle_price,
            size,
            premium_rate,
            withdrawable,
            fee,
            is_call,
            exercised,
            event_seq
        );

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn remove_di_option(
        &mut self,
        base_mint: [u8; 32],
        quote_mint: [u8; 32],
        expiry_date: i64,
        strike_price: u64,
        settle_price: u64,
        volume: u64,
        is_call: bool,
    ) -> DexResult {
        let event = DIOptionRemoved {
            base_mint,
            quote_mint,
            expiry_date,
            strike_price,
            settle_price,
            volume,
            is_call,
        };

        let event_seq = self.append(event)?;
        msg!(
            "DI option removed: {:?} {:?} {:?} {} {} {} {} {}",
            base_mint,
            quote_mint,
            expiry_date,
            strike_price,
            settle_price,
            volume,
            is_call,
            event_seq
        );

        Ok(())
    }
}
