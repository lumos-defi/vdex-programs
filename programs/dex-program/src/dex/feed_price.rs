use std::convert::TryFrom;

use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::{
    dex::{state::*, OracleSource},
    errors::{DexError, DexResult},
};

#[derive(Accounts)]
pub struct FeedPrice<'info> {
    #[account(
        mut,
        has_one = authority,
    )]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub feed_price: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<FeedPrice>, prices: [u64; 16]) -> DexResult {
    Ok(())
}
