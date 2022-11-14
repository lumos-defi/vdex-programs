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
}
