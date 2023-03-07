use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    collections::EventQueue,
    dex::{event::AppendEvent, Dex},
    errors::DexError,
    errors::DexResult,
    utils::SafeMath,
};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    pub in_mint: AccountInfo<'info>,

    /// CHECK
    pub in_mint_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub in_vault: AccountInfo<'info>,

    #[account(
         mut,
         constraint = (user_in_mint_acc.owner == *authority.key && user_in_mint_acc.mint == *in_mint.key)
     )]
    pub user_in_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    pub out_mint: AccountInfo<'info>,

    /// CHECK
    pub out_mint_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub out_vault: AccountInfo<'info>,

    /// CHECK
    pub out_vault_program_signer: AccountInfo<'info>,

    #[account(
         mut,
         constraint = (user_out_mint_acc.owner == *authority.key && user_out_mint_acc.mint == *out_mint.key)
     )]
    pub user_out_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    #[account(seeds = [dex.key().as_ref(), authority.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    /// CHECK
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Swap>, amount: u64) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    let (ain, aii) = dex.find_asset_by_mint(ctx.accounts.in_mint.key())?;
    require!(
        aii.mint == ctx.accounts.in_mint.key()
            && aii.oracle == ctx.accounts.in_mint_oracle.key()
            && aii.vault == ctx.accounts.in_vault.key(),
        DexError::InvalidMint
    );

    let (aout, aoi) = dex.find_asset_by_mint(ctx.accounts.out_mint.key())?;
    require!(
        aoi.mint == ctx.accounts.out_mint.key()
            && aoi.oracle == ctx.accounts.out_mint_oracle.key()
            && aoi.vault == ctx.accounts.out_vault.key()
            && aoi.program_signer == ctx.accounts.out_vault_program_signer.key(),
        DexError::InvalidMint
    );

    require!(aii.mint != aoi.mint, DexError::InvalidMint);

    let seeds = &[
        ctx.accounts.out_mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[aoi.nonce],
    ];
    let signer = &[&seeds[..]];

    let oracles = &vec![&ctx.accounts.in_mint_oracle, &ctx.accounts.out_mint_oracle];
    let (out, fee) = dex.swap(ain, aout, amount, true, &oracles)?;

    dex.swap_in(ain, amount.safe_sub(fee)?, fee)?;
    dex.swap_out(aout, out)?;

    //Swap in assets
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_in_mint_acc.to_account_info(),
        to: ctx.accounts.in_vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    //Swap out assets
    let cpi_accounts = Transfer {
        from: ctx.accounts.out_vault.to_account_info(),
        to: ctx.accounts.user_out_mint_acc.to_account_info(),
        authority: ctx.accounts.out_vault_program_signer.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer,
    );
    token::transfer(cpi_ctx, out)?;

    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidEventQueue
    );

    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;
    event_queue.swap_asset(
        ctx.accounts.user_state.key().to_bytes(),
        ctx.accounts.in_mint.key().to_bytes(),
        ctx.accounts.out_mint.key().to_bytes(),
        amount,
        out,
        fee,
    )
}
