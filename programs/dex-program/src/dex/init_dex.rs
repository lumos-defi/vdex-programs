use anchor_lang::prelude::*;

use crate::{
    collections::{EventQueue, MountMode, PagedList, SingleEventQueue},
    dex::state::*,
    errors::{DexError, DexResult},
    utils::{DEX_MAGIC_NUMBER, USER_LIST_MAGIC_BYTE},
};

#[derive(Accounts)]
pub struct InitDex<'info> {
    #[account(zero)]
    pub dex: AccountLoader<'info, Dex>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= match_queue.owner == program_id)]
    pub match_queue: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= user_list_entry_page.owner == program_id)]
    pub user_list_entry_page: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<InitDex>) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_init()?;

    dex.magic = DEX_MAGIC_NUMBER;
    dex.authority = *ctx.accounts.authority.key;
    dex.event_queue = *ctx.accounts.event_queue.key;
    dex.match_queue = *ctx.accounts.match_queue.key;
    dex.user_list_entry_page = *ctx.accounts.user_list_entry_page.key;
    dex.user_list_remaining_pages_number = 0;
    dex.assets_number = 0;
    dex.markets_number = 0;

    EventQueue::mount(&mut ctx.accounts.event_queue, false)?.initialize(true)?;
    SingleEventQueue::<MatchEvent>::mount(&mut ctx.accounts.match_queue, false)?
        .initialize()
        .map_err(|_| DexError::FailedInitMatchQueue)?;

    PagedList::<UserListItem>::mount(
        &mut ctx.accounts.user_list_entry_page,
        &[],
        USER_LIST_MAGIC_BYTE,
        MountMode::Initialize,
    )
    .map_err(|_| DexError::FailedInitializeUserList)?;

    Ok(())
}
