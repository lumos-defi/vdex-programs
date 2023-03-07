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
    #[account(mut, seeds = [dex.key().as_ref(), authority.key().as_ref()], bump, owner = *program_id)]
    pub user_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK
    #[account(executable, constraint = (token_program.key == &token::ID))]
    pub token_program: AccountInfo<'info>,
}

/// Markets_that_has_limit_orders.map({
///   order book account
///   order pool entry page
///   order pool remaining pages
///   BID_orders.map({
///     mint
///     vault
///     program signer
///     user mint acc
///   })
/// })
pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, CancelAllOrders<'info>>) -> DexResult {
    let dex = &ctx.accounts.dex.load()?;

    // Mount user state
    let us = UserState::mount(&ctx.accounts.user_state, true)?;
    let mut offset = 0usize;

    let token_program = ctx.accounts.token_program.clone();

    for market in 0..dex.markets_number as usize {
        let bid_orders = us.borrow().collect_orders(market, true);
        let ask_orders = us.borrow().collect_orders(market, false);

        if bid_orders.is_empty() && ask_orders.is_empty() {
            continue;
        }

        let mi = &dex.markets[market];

        let order_book = &ctx.remaining_accounts[offset];
        let order_pool_entry_page = &ctx.remaining_accounts[offset + 1];

        offset += 2;
        let order_pool_pages =
            &ctx.remaining_accounts[offset..offset + mi.order_pool_remaining_pages_number as usize];

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

        offset += mi.order_pool_remaining_pages_number as usize;

        // Mount order book & order pool
        let order_book = OrderBook::mount(order_book, true)?;
        let order_pool = PagedList::<Order>::mount(
            order_pool_entry_page,
            order_pool_pages,
            ORDER_POOL_MAGIC_BYTE,
            MountMode::ReadWrite,
        )
        .map_err(|_| DexError::FailedMountOrderPool)?;

        // Cancel bid orders
        for user_order_slot in bid_orders {
            let order_slot = us
                .borrow()
                .get_order_info(user_order_slot)
                .map_err(|_| DexError::InvalidOrderSlot)?;

            let (_, open, long, asset, size) = us
                .borrow_mut()
                .unlink_order(user_order_slot, true)
                .map_err(|_| DexError::InvalidOrderSlot)?;

            let mint = &ctx.remaining_accounts[offset];
            let vault = &ctx.remaining_accounts[offset + 1];
            let program_signer = &ctx.remaining_accounts[offset + 2];
            let user_mint_acc = &ctx.remaining_accounts[offset + 3];

            let ai = dex.asset_as_ref(asset)?;
            require!(
                ai.valid
                    && ai.mint == mint.key()
                    && ai.vault == vault.key()
                    && ai.program_signer == program_signer.key(),
                DexError::InvalidAssetIndex
            );

            let seeds = &[
                mint.key.as_ref(),
                ctx.accounts.dex.to_account_info().key.as_ref(),
                &[ai.nonce],
            ];
            let signer_seeds = &[&seeds[..]];

            let cpi_accounts = Transfer {
                from: vault.to_account_info().clone(),
                to: user_mint_acc.to_account_info().clone(),
                authority: program_signer.to_account_info().clone(),
            };

            let cpi_ctx =
                CpiContext::new_with_signer(token_program.clone(), cpi_accounts, signer_seeds);
            token::transfer(cpi_ctx, size)?;

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

            offset += 4;
        }

        // Cancel ask orders
        for user_order_slot in ask_orders {
            let order_slot = us
                .borrow()
                .get_order_info(user_order_slot)
                .map_err(|_| DexError::InvalidOrderSlot)?;

            let (_, open, long, _, _) = us
                .borrow_mut()
                .unlink_order(user_order_slot, true)
                .map_err(|_| DexError::InvalidOrderSlot)?;

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
    }

    Ok(())
}
