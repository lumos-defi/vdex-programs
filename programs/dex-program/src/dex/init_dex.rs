use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    collections::{EventQueue, MountMode, PagedList, SingleEventQueue},
    dex::state::*,
    errors::{DexError, DexResult},
    order::MatchEvent,
    utils::{DEX_MAGIC_NUMBER, USER_LIST_MAGIC_BYTE},
};

#[derive(Accounts)]
#[instruction(vlp_decimal: u8)]
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

    #[account(
        init,
        seeds = [
            dex.key().as_ref(),
            b"vlp".as_ref(),
        ],
        bump,
        payer = authority,
        mint::decimals = vlp_decimal,
        mint::authority = vlp_program_signer,
        mint::freeze_authority = vlp_program_signer,
    )]
    pub vlp_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = vlp_mint,
        associated_token::authority = vlp_program_signer,
    )]
    pub vlp_vault: Account<'info, TokenAccount>,

    /// CHECK
    pub reward_mint: UncheckedAccount<'info>,

    /// CHECK
    pub vlp_program_signer: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    ///CHECK
    pub rent: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<InitDex>, vlp_decimals: u8, vlp_mint_nonce: u8) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_init()?;

    dex.magic = DEX_MAGIC_NUMBER;
    dex.authority = ctx.accounts.authority.key();
    dex.event_queue = ctx.accounts.event_queue.key();
    dex.match_queue = ctx.accounts.match_queue.key();
    dex.usdc_mint = ctx.accounts.usdc_mint.key();
    dex.user_list_entry_page = ctx.accounts.user_list_entry_page.key();
    dex.user_list_remaining_pages_number = 0;
    dex.assets_number = 0;
    dex.markets_number = 0;
    dex.usdc_asset_index = 0xff;
    dex.vlp_pool.init(
        ctx.accounts.vlp_mint.key(),
        ctx.accounts.vlp_vault.key(),
        ctx.accounts.vlp_program_signer.key(),
        ctx.accounts.reward_mint.key(),
        vlp_mint_nonce,
        vlp_decimals,
        u8::MAX,
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

    Ok(())
}
