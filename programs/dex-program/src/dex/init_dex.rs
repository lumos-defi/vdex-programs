use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::{dex::state::*, errors::DexResult, utils::DEX_MAGIC_NUMBER};

#[derive(Accounts)]
#[instruction(vlp_decimal: u8)]
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

    #[account(
        init,
        seeds = [
            dex.key().as_ref(),
            b"vlp".as_ref(),
        ],
        bump,
        payer = authority,
        mint::decimals = vlp_decimal,
        mint::authority = vlp_mint_authority,
        mint::freeze_authority = vlp_mint_authority,
    )]
    pub vlp_mint: Account<'info, Mint>,

    /// CHECK
    pub vlp_mint_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    ///CHECK
    pub rent: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<InitDex>, _vlp_decimal: u8, vlp_mint_nonce: u8) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_init()?;

    dex.magic = DEX_MAGIC_NUMBER;
    dex.authority = *ctx.accounts.authority.key;
    dex.event_queue = *ctx.accounts.event_queue.key;
    dex.match_queue = *ctx.accounts.match_queue.key;
    dex.vlp_mint = ctx.accounts.vlp_mint.key();
    dex.vlp_mint_authority = *ctx.accounts.vlp_mint_authority.key;
    dex.user_list_entry_page = *ctx.accounts.user_list_entry_page.key;
    dex.user_list_remaining_pages_number = 0;
    dex.assets_number = 0;
    dex.markets_number = 0;
    dex.vlp_mint_nonce = vlp_mint_nonce;

    Ok(())
}
