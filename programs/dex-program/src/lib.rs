#![cfg_attr(feature = "no-entrypoint", allow(dead_code))]

pub mod errors;

use anchor_lang::prelude::*;
use errors::*;

declare_id!("2aJZ6AufDU5NRzXLg5Ww4S4Nf2tx7xZDQD6he2gjsKyq");

#[program]
pub mod dex_program {

    use super::*;

    pub fn init_dex(_ctx: Context<Initialize>) -> DexResult {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}
