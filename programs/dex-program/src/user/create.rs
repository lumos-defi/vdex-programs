use crate::{errors::DexResult, user::state::*};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(order_slot_count: u8, position_slot_count: u8)]
pub struct CreateUserState<'info> {
    /// CHECK
    #[account(
        init,
        seeds = [dex.key().as_ref(), authority.key.as_ref()],
        bump,
        payer = authority,
        space = UserState::required_account_size(order_slot_count, position_slot_count)
    )]
    pub user_state: UncheckedAccount<'info>,

    /// CHECK
    pub dex: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateUserState>,
    order_slot_count: u8,
    position_slot_count: u8,
) -> DexResult {
    let user_state = &mut ctx.accounts.user_state;

    UserState::initialize(
        user_state,
        order_slot_count,
        position_slot_count,
        ctx.accounts.authority.key(),
    )
}
