use anchor_lang::prelude::*;

use crate::{dex::state::*, errors::DexResult};

#[derive(Accounts)]
pub struct SetDelegate<'info> {
    #[account(
        mut,
        has_one = authority, owner = *program_id
    )]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub delegate: AccountInfo<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<SetDelegate>) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;
    dex.delegate = ctx.accounts.delegate.key();

    Ok(())
}
