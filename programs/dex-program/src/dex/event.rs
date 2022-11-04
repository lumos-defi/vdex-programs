use crate::collections::PackedEvent;
use anchor_lang::prelude::*;
#[cfg(feature = "client-support")]
use serde::Serialize;

#[derive(AnchorSerialize, AnchorDeserialize)]
#[cfg_attr(feature = "client-support", derive(Serialize))]
pub struct PositionFilled {
    pub user_state: [u8; 32],

    pub price: u64,
    pub size: u64,
    pub collateral: u64,
    pub borrow: u64,
    pub market: u8,

    pub open_or_close: u8,
    pub long_or_short: u8,

    pub fee: u64,
    // Only for closing position
    pub pnl: i64,
}

impl PackedEvent for PositionFilled {
    const DISCRIMINATOR: u8 = 1;
}
