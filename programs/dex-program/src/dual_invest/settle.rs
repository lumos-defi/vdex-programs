use crate::{
    collections::{EventQueue, MountMode, PagedList},
    dex::{event::AppendEvent, get_oracle_price, Dex, UserListItem},
    dual_invest::DI,
    errors::{DexError, DexResult},
    position::update_user_serial_number,
    user::state::*,
    utils::{get_timestamp, swap, SafeMath, FEE_RATE_BASE, USER_LIST_MAGIC_BYTE},
};

use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, CloseAccount, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct DiSettle<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= di_option.owner == program_id)]
    pub di_option: UncheckedAccount<'info>,

    /// CHECK
    pub user: AccountInfo<'info>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), user.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = (user_base_mint_acc.mint == *base_mint.key)
    )]
    pub user_base_mint_acc: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = (user_quote_mint_acc.owner == *user.key && user_quote_mint_acc.mint == *quote_mint.key)
    )]
    pub user_quote_mint_acc: Box<Account<'info, TokenAccount>>,

    /// CHECK
    pub base_mint: AccountInfo<'info>,

    /// CHECK
    pub quote_mint: AccountInfo<'info>,

    /// CHECK
    pub quote_asset_oracle: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub base_mint_vault: AccountInfo<'info>,

    /// CHECK
    #[account(mut)]
    pub quote_mint_vault: AccountInfo<'info>,

    /// CHECK
    pub base_asset_program_signer: AccountInfo<'info>,

    /// CHECK
    pub quote_asset_program_signer: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(mut, constraint= event_queue.owner == program_id)]
    pub event_queue: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= user_list_entry_page.owner == program_id)]
    pub user_list_entry_page: UncheckedAccount<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,

    /// CHECK
    #[account(executable, constraint = (system_program.key == &system_program::ID))]
    pub system_program: AccountInfo<'info>,
}

fn withdraw(
    ctx: &Context<DiSettle>,
    is_base_mint: bool,
    seeds: &[&[u8]; 3],
    amount: u64,
) -> DexResult {
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = if is_base_mint {
        Transfer {
            from: ctx.accounts.base_mint_vault.to_account_info(),
            to: ctx.accounts.user_base_mint_acc.to_account_info(),
            authority: ctx.accounts.base_asset_program_signer.to_account_info(),
        }
    } else {
        Transfer {
            from: ctx.accounts.quote_mint_vault.to_account_info(),
            to: ctx.accounts.user_quote_mint_acc.to_account_info(),
            authority: ctx.accounts.quote_asset_program_signer.to_account_info(),
        }
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.clone(),
        cpi_accounts,
        signer_seeds,
    );

    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

fn relay_native_mint_to_user(ctx: &Context<DiSettle>, lamports: u64) -> DexResult {
    let cpi_close = CloseAccount {
        account: ctx.accounts.user_base_mint_acc.to_account_info(),
        destination: ctx.accounts.authority.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.clone(), cpi_close);
    token::close_account(cpi_ctx)?;

    let cpi_sys_transfer = system_program::Transfer {
        from: ctx.accounts.authority.to_account_info(),
        to: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.system_program.clone(), cpi_sys_transfer);

    system_program::transfer(cpi_ctx, lamports)
}

// Layout of remaining accounts:
//  offset 0 ~ n: user_list remaining pages
pub fn handler(ctx: Context<DiSettle>, id: u64, force: bool, settle_price: u64) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;
    require!(
        dex.user_list_entry_page == ctx.accounts.user_list_entry_page.key(),
        DexError::InvalidUserListEntryPage
    );
    require!(
        dex.event_queue == ctx.accounts.event_queue.key(),
        DexError::InvalidUserListEntryPage
    );

    require!(
        dex.di_option == ctx.accounts.di_option.key(),
        DexError::InvalidDIOptionAccount
    );
    let di = DI::mount(&ctx.accounts.di_option, true)?;
    if force {
        // Force to use the provided settle price
        require!(
            di.borrow().meta.admin == ctx.accounts.authority.key(),
            DexError::InvalidDIAdmin
        );
    }

    // Get strike price
    let actual_settle_price = if let Ok(option) = di.borrow().get_di_option(id) {
        let now = get_timestamp()?;
        require!(now >= option.expiry_date, DexError::DIOptionNotExpired);
        require!(option.settle_price != 0, DexError::DIOptionNoSettlePrice);

        option.settle_price
    } else {
        require!(force && settle_price != 0, DexError::DIOptionNoSettlePrice);
        settle_price
    };

    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let (option_slot, option) = us.borrow().get_di_option(id)?;

    let base_ai = dex.asset_as_ref(option.base_asset_index)?;
    let quote_ai = dex.asset_as_ref(option.quote_asset_index)?;

    require!(
        quote_ai.oracle == ctx.accounts.quote_asset_oracle.key(),
        DexError::InvalidOracle
    );
    require!(
        base_ai.mint == ctx.accounts.base_mint.key()
            && quote_ai.mint == ctx.accounts.quote_mint.key(),
        DexError::InvalidMint
    );
    require!(
        base_ai.vault == ctx.accounts.base_mint_vault.key()
            && quote_ai.vault == ctx.accounts.quote_mint_vault.key(),
        DexError::InvalidVault
    );
    require!(
        base_ai.program_signer == ctx.accounts.base_asset_program_signer.key()
            && quote_ai.program_signer == ctx.accounts.quote_asset_program_signer.key(),
        DexError::InvalidProgramSigner
    );

    if ctx.accounts.base_mint.key() == token::spl_token::native_mint::id() {
        require!(
            ctx.accounts.user_base_mint_acc.owner == ctx.accounts.authority.key()
                && ctx.accounts.user_base_mint_acc.mint == ctx.accounts.base_mint.key(),
            DexError::InvalidUserMintAccount
        );
    } else {
        require!(
            ctx.accounts.user_base_mint_acc.owner == ctx.accounts.user.key()
                && ctx.accounts.user_base_mint_acc.mint == ctx.accounts.base_mint.key(),
            DexError::InvalidUserMintAccount
        );
    }

    let quote_asset_seeds = &[
        ctx.accounts.quote_mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[quote_ai.nonce],
    ];

    let base_asset_seeds = &[
        ctx.accounts.base_mint.key.as_ref(),
        ctx.accounts.dex.to_account_info().key.as_ref(),
        &[base_ai.nonce],
    ];

    let base_mint = base_ai.mint.to_bytes();
    let quote_mint = quote_ai.mint.to_bytes();

    let fee_rate = di.borrow().meta.fee_rate as u64;

    let fee = if option.is_call {
        if actual_settle_price >= option.strike_price {
            // Call option, exercised, swap base asset to quote asset, return quote asset + premium to user
            let quote_asset_price =
                get_oracle_price(quote_ai.oracle_source, &ctx.accounts.quote_asset_oracle)?;

            let swapped_quote_asset = swap(
                option.size,
                option.strike_price,
                base_ai.decimals,
                quote_asset_price,
                quote_ai.decimals,
            )?;

            let quote_asset_with_premium = swapped_quote_asset
                + swapped_quote_asset
                    .safe_mul(option.premium_rate as u64)?
                    .safe_div(FEE_RATE_BASE as u128)? as u64;

            let total = if quote_asset_with_premium <= option.borrowed_quote_funds {
                dex.di_option_refund(
                    option.quote_asset_index,
                    option.borrowed_quote_funds - quote_asset_with_premium,
                )?;
                quote_asset_with_premium
            } else {
                let borrow_more = quote_asset_with_premium - option.borrowed_quote_funds;
                if let Ok(_) = dex.has_sufficient_asset(option.quote_asset_index, borrow_more) {
                    dex.di_option_borrow(option.quote_asset_index, borrow_more)?;
                    quote_asset_with_premium
                } else {
                    option.borrowed_quote_funds
                }
            };

            dex.di_option_loss(option.quote_asset_index, total)?;

            dex.di_option_refund(option.base_asset_index, option.borrowed_base_funds)?;
            dex.di_option_add_fund(option.base_asset_index, option.size)?;

            let fee = total.safe_mul(fee_rate)?.safe_div(FEE_RATE_BASE)? as u64;
            let withdrawable = total.safe_sub(fee)?;

            dex.di_option_charge_fee(option.quote_asset_index, fee)?;
            withdraw(&ctx, false, quote_asset_seeds, withdrawable)?;

            fee
        } else {
            // Call option, not exercised, return base asset + premium to user
            let total = option.size + option.borrowed_base_funds;

            dex.di_option_loss(option.base_asset_index, option.borrowed_base_funds)?;
            dex.di_option_refund(option.quote_asset_index, option.borrowed_quote_funds)?;

            let fee = total.safe_mul(fee_rate)?.safe_div(FEE_RATE_BASE)? as u64;
            dex.di_option_charge_fee(option.base_asset_index, fee)?;

            let withdrawable = total.safe_sub(fee)?;
            withdraw(&ctx, true, base_asset_seeds, withdrawable)?;

            // If base mint is SOL,we can't create a temp WSOL account for the end user(we are settling, no user sign),
            // so have to use the authority as a "replay" to transfer the native mint to user
            if ctx.accounts.base_mint.key() == token::spl_token::native_mint::id() {
                relay_native_mint_to_user(&ctx, withdrawable)?;
            }

            fee
        }
    } else {
        if actual_settle_price <= option.strike_price {
            // Put option, exercised, swap quote asset to base asset, return base asset + premium to user
            let quote_asset_price =
                get_oracle_price(quote_ai.oracle_source, &ctx.accounts.quote_asset_oracle)?;

            let swapped_base_asset = swap(
                option.size,
                quote_asset_price,
                quote_ai.decimals,
                option.strike_price,
                base_ai.decimals,
            )?;

            let base_asset_with_premium = swapped_base_asset
                + swapped_base_asset
                    .safe_mul(option.premium_rate as u64)?
                    .safe_div(FEE_RATE_BASE as u128)? as u64;

            let total = if base_asset_with_premium <= option.borrowed_base_funds {
                dex.di_option_refund(
                    option.base_asset_index,
                    option.borrowed_base_funds - base_asset_with_premium,
                )?;
                base_asset_with_premium
            } else {
                let borrow_more = base_asset_with_premium - option.borrowed_base_funds;
                if let Ok(_) = dex.has_sufficient_asset(option.base_asset_index, borrow_more) {
                    dex.di_option_borrow(option.base_asset_index, borrow_more)?;
                    base_asset_with_premium
                } else {
                    option.borrowed_base_funds
                }
            };
            dex.di_option_loss(option.base_asset_index, total)?;

            dex.di_option_refund(option.quote_asset_index, option.borrowed_quote_funds)?;
            dex.di_option_add_fund(option.quote_asset_index, option.size)?;

            let fee = total.safe_mul(fee_rate)?.safe_div(FEE_RATE_BASE)? as u64;
            dex.di_option_charge_fee(option.base_asset_index, fee)?;

            let withdrawable = total.safe_sub(fee)?;

            withdraw(&ctx, true, base_asset_seeds, withdrawable)?;

            // If base mint is SOL,we can't create a temp WSOL account for the end user(we are settling, no user sign),
            // so have to use the authority as a "replay" to transfer the native mint to user
            if ctx.accounts.base_mint.key() == token::spl_token::native_mint::id() {
                relay_native_mint_to_user(&ctx, withdrawable)?;
            }

            fee
        } else {
            // Put option, not exercised
            let total = option.size + option.borrowed_quote_funds;

            dex.di_option_loss(option.quote_asset_index, option.borrowed_quote_funds)?;
            dex.di_option_refund(option.base_asset_index, option.borrowed_base_funds)?;

            let fee = total.safe_mul(fee_rate)?.safe_div(FEE_RATE_BASE)? as u64;
            dex.di_option_charge_fee(option.quote_asset_index, fee)?;

            let withdrawable = total.safe_sub(fee)?;
            withdraw(&ctx, false, quote_asset_seeds, withdrawable)?;

            fee
        }
    };

    di.borrow_mut().add_settle_size(id, option.size)?;
    us.borrow_mut().remove_di_option(option_slot)?;

    // Save to event queue
    let mut event_queue = EventQueue::mount(&ctx.accounts.event_queue, true)
        .map_err(|_| DexError::FailedMountEventQueue)?;

    let user_state_key = ctx.accounts.user_state.key().to_bytes();
    event_queue.settle_di_option(
        user_state_key,
        base_mint,
        quote_mint,
        option.expiry_date,
        option.strike_price,
        actual_settle_price,
        option.size,
        option.premium_rate,
        fee,
        option.is_call,
    )?;

    // Update user
    let user_list = PagedList::<UserListItem>::mount(
        &ctx.accounts.user_list_entry_page,
        &ctx.remaining_accounts,
        USER_LIST_MAGIC_BYTE,
        MountMode::ReadWrite,
    )
    .map_err(|_| DexError::FailedInitializeUserList)?;

    update_user_serial_number(&user_list, us.borrow_mut(), ctx.accounts.user_state.key())
}