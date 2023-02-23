use anchor_lang::prelude::*;

use crate::{
    collections::EventQueue,
    dex::{event::AppendEvent, Dex},
    dual_invest::DI,
    errors::{DexError, DexResult},
};

#[derive(Accounts)]
pub struct DiRemoveOption<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<DiRemoveOption>, id: u64, force: bool) -> DexResult {
    let dex = &mut ctx.accounts.dex.load()?;
    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );
    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidUserListEntryPage
    );

    let di = DI::mount(&ctx.accounts.di_option, true)?;
    if force {
        require!(
            di.borrow().meta.admin == ctx.accounts.authority.key(),
            DexError::InvalidDIAdmin
        );
    }

    // Save to event queue
    let option = di.borrow().get_di_option(id)?;

    let base_ai = dex.asset_as_ref(option.base_asset_index)?;
    let quote_ai = dex.asset_as_ref(option.quote_asset_index)?;

    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    event_queue.remove_di_option(
        base_ai.mint.to_bytes(),
        quote_ai.mint.to_bytes(),
        option.expiry_date,
        option.strike_price,
        option.settle_price,
        option.volume,
        option.is_call,
    )?;

    di.borrow_mut().remove(id, force)?;

    Ok(())
}
