use anchor_lang::prelude::*;

use crate::{
    collections::PagedList,
    dex::{Dex, UserListItem},
    errors::{DexError, DexResult},
    utils::{MAX_USER_LIST_REMAINING_PAGES_COUNT, USER_LIST_MAGIC_BYTE},
};

#[derive(Accounts)]
pub struct AddUserPage<'info> {
    #[account(mut, owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, constraint= user_list_entry_page.owner == program_id)]
    pub user_list_entry_page: UncheckedAccount<'info>,

    /// CHECK
    #[account(mut, constraint= page.owner == program_id)]
    pub page: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

//  Remaining accounts layout:
//  existing user list remaining accounts
pub fn handler(ctx: Context<AddUserPage>) -> DexResult {
    let dex = &mut ctx.accounts.dex.load_mut()?;

    require!(
        dex.delegate == ctx.accounts.authority.key()
            || dex.authority == ctx.accounts.authority.key(),
        DexError::InvalidAdminOrDelegate
    );

    require!(
        dex.user_list_remaining_pages_number < MAX_USER_LIST_REMAINING_PAGES_COUNT as u8,
        DexError::InsufficientUserPageSlots
    );

    require!(
        ctx.remaining_accounts.len() == dex.user_list_remaining_pages_number as usize,
        DexError::InvalidRemainingAccounts
    );

    for i in 0..dex.user_list_remaining_pages_number as usize {
        require_eq!(
            dex.user_list_remaining_pages[i],
            ctx.remaining_accounts[i].key(),
            DexError::InvalidRemainingAccounts
        );
    }

    PagedList::<UserListItem>::append_pages(
        &ctx.accounts.user_list_entry_page,
        &ctx.remaining_accounts[0..dex.user_list_remaining_pages_number as usize],
        &[ctx.accounts.page.to_account_info()],
        USER_LIST_MAGIC_BYTE,
    )
    .map_err(|e| {
        msg!(&e.to_string());
        DexError::FailedAppendUserPage
    })?;

    dex.user_list_remaining_pages[ctx.remaining_accounts.len()] = ctx.accounts.page.key();
    dex.user_list_remaining_pages_number += 1;

    Ok(())
}
