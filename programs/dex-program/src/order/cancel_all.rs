use crate::{
    collections::{MountMode, OrderBook, PagedList},
    dex::Dex,
    errors::{DexError, DexResult},
    order::{select_side, Order},
    user::state::*,
    utils::ORDER_POOL_MAGIC_BYTE,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};

#[derive(Accounts)]
pub struct CancelAllOrders<'info> {
    #[account(owner = *program_id)]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,
}

/// Orders.map( {
///   mint
///   vault
///   program signer
///   user mint acc
///   order book account
///   order pool entry page
///   order pool remaining pages
/// } )
pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, CancelAllOrders<'info>>) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;

    // Mount user state
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let orders = us.borrow().collect_orders();
    let mut offset = 0usize;

    let token_program = ctx.accounts.token_program.clone();
    for user_order_slot in orders {
        let order_slot = us
            .borrow()
            .get_order_info(user_order_slot)
            .map_err(|_| DexError::InvalidOrderSlot)?;

        let (market, open, long, asset, size) = us
            .borrow_mut()
            .unlink_order(user_order_slot, true)
            .map_err(|_| DexError::InvalidOrderSlot)?;
        require!(market < dex.markets_number, DexError::InvalidMarketIndex);

        let mint = &ctx.remaining_accounts[offset];
        let vault = &ctx.remaining_accounts[offset + 1];
        let program_signer = &ctx.remaining_accounts[offset + 2];
        let user_mint_acc = &ctx.remaining_accounts[offset + 3];
        let order_book = &ctx.remaining_accounts[offset + 4];
        let order_pool_entry_page = &ctx.remaining_accounts[offset + 5];

        let order_pool_pages = &ctx.remaining_accounts[offset + 6..];

        // Refund if it's bid order
        if open {
            let ai = dex.asset_as_ref(asset)?;
            require!(
                ai.valid
                    && ai.mint == mint.key()
                    && ai.vault == vault.key()
                    && ai.program_signer == program_signer.key(),
                DexError::InvalidAssetIndex
            );

            let seeds = &[
                ctx.remaining_accounts[offset].key.as_ref(),
                ctx.accounts.dex.to_account_info().key.as_ref(),
                &[ai.nonce],
            ];
            let signer_seeds = &[&seeds[..]];

            let cpi_accounts = Transfer {
                from: ctx.remaining_accounts[offset + 1].to_account_info().clone(),
                to: ctx.remaining_accounts[offset + 3].to_account_info().clone(),
                authority: ctx.remaining_accounts[offset + 2].to_account_info().clone(),
            };

            let cpi_ctx =
                CpiContext::new_with_signer(token_program.clone(), cpi_accounts, signer_seeds);
            // let cpi_ctx = CpiContext::new(token_program.clone(), cpi_accounts);
            token::transfer(cpi_ctx, size)?;
        }

        let mi = &dex.markets[market as usize];
        require!(
            mi.valid
                && mi.order_book == order_book.key()
                && mi.order_pool_entry_page == order_pool_entry_page.key(),
            DexError::InvalidMarketIndex
        );

        for i in 0..mi.order_pool_remaining_pages_number as usize {
            require_eq!(
                mi.order_pool_remaining_pages[i],
                ctx.remaining_accounts[offset + 6 + i].key(),
                DexError::InvalidRemainingAccounts
            );
        }

        offset += 6 + mi.order_pool_remaining_pages_number as usize;

        let ai = &dex.assets[mi.asset_index as usize];
        require!(ai.valid, DexError::InvalidMarketIndex);

        // Mount order book & order pool
        let order_book = OrderBook::mount(order_book, true)?;
        let order_pool = PagedList::<Order>::mount(
            order_pool_entry_page,
            order_pool_pages,
            ORDER_POOL_MAGIC_BYTE,
            MountMode::ReadWrite,
        )
        .map_err(|_| DexError::FailedMountOrderPool)?;

        let order = order_pool
            .from_index(order_slot)
            .map_err(|_| DexError::InvalidOrderSlot)?;

        require!(order.in_use(), DexError::InvalidOrderSlot);

        require_eq!(
            order.data.user_order_slot,
            user_order_slot,
            DexError::InvalidOrderSlot
        );

        order_book.unlink_order(select_side(open, long), order, &order_pool)?;
    }

    Ok(())
}
