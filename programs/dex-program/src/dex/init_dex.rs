use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption::Some as CSome;
use anchor_spl::token::{Mint, TokenAccount};

use crate::{
    collections::{EventQueue, MountMode, PagedList, SingleEventQueue},
    dex::state::*,
    dual_invest::DI,
    errors::{DexError, DexResult},
    order::MatchEvent,
    utils::{get_timestamp, DEX_MAGIC_NUMBER, USER_LIST_MAGIC_BYTE},
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

    /// CHECK:
    pub vdx_program_signer: AccountInfo<'info>,

    /// CHECK:
    #[account(constraint= vdx_mint.mint_authority == CSome(vdx_program_signer.key()) && vdx_mint.freeze_authority == CSome(authority.key()))]
    vdx_mint: Box<Account<'info, Mint>>,

    /// CHECK: Vault for locking asset
    #[account(constraint = vdx_vault.mint == vdx_mint.key() && vdx_vault.owner == vdx_program_signer.key()  @DexError::InvalidMint)]
    vdx_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK
    pub reward_mint: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<InitDex>, vdx_nonce: u8, di_fee_rate: u16) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_init()?;

    dex.magic = DEX_MAGIC_NUMBER;
    dex.authority = ctx.accounts.authority.key();
    dex.event_queue = ctx.accounts.event_queue.key();
    dex.match_queue = ctx.accounts.match_queue.key();
    dex.usdc_mint = ctx.accounts.usdc_mint.key();
    dex.di_option = ctx.accounts.di_option.key();
    dex.user_list_entry_page = ctx.accounts.user_list_entry_page.key();
    dex.mint_es_vdx_last_timestamp = get_timestamp()?;
    dex.user_list_remaining_pages_number = 0;
    dex.assets_number = 0;
    dex.markets_number = 0;
    dex.usdc_asset_index = 0xff;
    dex.vlp_pool.init(
        // Dummy VLP token, never mint
        Pubkey::default(),
        Pubkey::default(),
        Pubkey::default(),
        ctx.accounts.reward_mint.key(),
        u8::MAX,
        6,
        u8::MAX, // Will be updated when reward asset is added
    );

    let (program_signer, program_signer_nonce) = Pubkey::find_program_address(
        &[
            &ctx.accounts.vdx_mint.key().to_bytes(),
            &ctx.accounts.dex.to_account_info().key.to_bytes(),
        ],
        ctx.program_id,
    );

    require!(
        vdx_nonce == program_signer_nonce
            && ctx.accounts.vdx_program_signer.key() == program_signer,
        DexError::InvalidProgramSigner
    );

    dex.vdx_pool.init(
        ctx.accounts.vdx_mint.key(),
        ctx.accounts.vdx_vault.key(),
        ctx.accounts.vdx_program_signer.key(),
        ctx.accounts.reward_mint.key(),
        vdx_nonce,
        ctx.accounts.vdx_mint.decimals,
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
        di_fee_rate,
    )
}
