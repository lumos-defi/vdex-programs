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
    const DISCRIMINATOR: u8 = 1;
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
    const DISCRIMINATOR: u8 = 2;
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
    const DISCRIMINATOR: u8 = 3;
}

pub trait AppendEvent {
    #[allow(clippy::too_many_arguments)]
    fn fill_position(
        &mut self,
        user_state: [u8; 32],
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
}

impl AppendEvent for EventQueue<'_> {
    #[allow(clippy::too_many_arguments)]
    fn fill_position(
        &mut self,
        user_state: [u8; 32],
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
}
