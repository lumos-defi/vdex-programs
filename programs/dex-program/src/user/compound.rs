use anchor_lang::prelude::*;

use crate::{dex::Dex, errors::DexError, errors::DexResult, user::UserState};

#[derive(Accounts)]
pub struct Compound<'info> {
    #[account(mut)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    /// CHECK
    pub authority: Signer<'info>,
}

// Remaining accounts layout:
// dex.assets.map({
//   asset index price oracle account
// })
pub fn handler(ctx: Context<Compound>) -> DexResult {
    let mut dex = &mut ctx.accounts.dex.load_mut()?;
    let us = UserState::mount(&ctx.accounts.user_state, true)?;

    let assets_oracles_len = dex.assets.iter().filter(|a| a.valid).count();

    let reward_asset_debt =
        dex.update_staking_pool(&ctx.remaining_accounts[0..assets_oracles_len])?;
    require!(reward_asset_debt == 0, DexError::InsufficientSolLiquidity);

    us.borrow_mut().compound_es_vdx(&mut dex)?;

    Ok(())
}
