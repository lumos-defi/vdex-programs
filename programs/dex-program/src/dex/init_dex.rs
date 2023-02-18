use anchor_lang::prelude::*;

use crate::{
    collections::{EventQueue, MountMode, PagedList, SingleEventQueue},
    dex::state::*,
    dual_invest::DI,
    errors::{DexError, DexResult},
    order::MatchEvent,
    utils::{DEX_MAGIC_NUMBER, USER_LIST_MAGIC_BYTE},
};

#[derive(Accounts)]
pub struct InitDex<'info> {
    #[account(zero)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub usdc_mint: AccountInfo<'info>,

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

    /// CHECK
    pub reward_mint: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<InitDex>, vlp_decimals: u8) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_init()?;

    dex.magic = DEX_MAGIC_NUMBER;
    dex.authority = ctx.accounts.authority.key();
    dex.event_queue = ctx.accounts.event_queue.key();
    dex.match_queue = ctx.accounts.match_queue.key();
    dex.usdc_mint = ctx.accounts.usdc_mint.key();
    dex.di_option = ctx.accounts.di_option.key();
    dex.user_list_entry_page = ctx.accounts.user_list_entry_page.key();
    dex.user_list_remaining_pages_number = 0;
    dex.assets_number = 0;
    dex.markets_number = 0;
    dex.usdc_asset_index = 0xff;
    dex.vlp_pool.init(
        // Dummy VLP token, could never mint
        Pubkey::default(),
        Pubkey::default(),
        Pubkey::default(),
        ctx.accounts.reward_mint.key(),
        u8::MAX,
        vlp_decimals,
        u8::MAX, // Will be updated when reward asset is added
    );

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

    DI::initialize(
        &mut ctx.accounts.di_option,
        64u8,
        ctx.accounts.authority.key(),
    )
}
